use std::collections::HashMap;

use serde::Serialize;

use crate::model::{
    CallExpression, ExportMacro, Expression, FunctionDoc, ParamSpecAssignment, Property,
    PropertyDoc, Signal, SignalDoc, SourceLocation, Statement, TypeInfo, VariableDecl,
};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Parameter {
    Regular {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        type_info: TypeInfo,
        location: SourceLocation,
    },
    Variadic,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDeclItem {
    pub name: String,
    pub return_type: TypeInfo,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_inline: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub export_macros: Vec<ExportMacro>,
    pub location: SourceLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<FunctionDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefItem {
    pub name: String,
    pub return_type: TypeInfo,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_inline: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body_statements: Vec<Statement>,
    pub location: SourceLocation,
    #[serde(skip)]
    pub body_location: Option<SourceLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<FunctionDoc>,
}

impl FunctionDefItem {
    /// Find all calls to specific functions in the body
    /// Returns references to all CallExpression nodes that match any of the
    /// given function names
    pub fn find_calls<'a>(&'a self, function_names: &[&str]) -> Vec<&'a CallExpression> {
        self.find_calls_matching(|name| function_names.contains(&name))
    }

    /// Find all calls matching a predicate in the body
    pub fn find_calls_matching<F>(&self, predicate: F) -> Vec<&CallExpression>
    where
        F: Fn(&str) -> bool,
    {
        let mut exprs: Vec<&Expression> = Vec::new();
        for stmt in &self.body_statements {
            stmt.walk_expressions(&mut |expr| exprs.push(expr));
        }

        let mut results = Vec::new();
        for expr in exprs {
            expr.walk(&mut |e| {
                if let Expression::Call(call) = e
                    && call.function_name_str().is_some_and(&predicate)
                {
                    results.push(call);
                }
            });
        }
        results
    }

    /// Extract signal registrations from the function body.
    /// Populates `enum_value` when the signal is assigned via
    /// `signals[ENUM] = g_signal_new(...)`.
    pub fn find_signal_registrations(&self, type_name: &str) -> Vec<Signal> {
        let mut signals = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        // First pass: assignments like `signals[ENUM] = g_signal_new(...)`
        for (i, stmt) in self.body_statements.iter().enumerate() {
            for assignment in stmt.iter_assignments() {
                let Expression::Call(call) = &*assignment.rhs else {
                    continue;
                };
                if !call.function_contains("g_signal_new") {
                    continue;
                }
                let Some(mut signal) = Signal::from_g_signal_new_call(call) else {
                    continue;
                };
                if let Expression::Subscript(sub) = &*assignment.lhs
                    && let Expression::Identifier(id) = &*sub.index
                {
                    signal.enum_value = Some(id.name.clone());
                }
                if i > 0
                    && let Statement::Comment(c) = &self.body_statements[i - 1]
                {
                    signal.doc = SignalDoc::from_comment_for(c, type_name, &signal.name);
                }
                seen_names.insert(signal.name.clone());
                signals.push(signal);
            }
        }

        // Second pass: standalone g_signal_new calls not already captured
        for (i, stmt) in self.body_statements.iter().enumerate() {
            for call in stmt.iter_calls() {
                if !call.function_name().starts_with("g_signal_new") {
                    continue;
                }
                let Some(name) = call.extract_string_from_arg(0) else {
                    continue;
                };
                if seen_names.contains(&name) {
                    continue;
                }
                if let Some(mut signal) = Signal::from_g_signal_new_call(call) {
                    if i > 0
                        && let Statement::Comment(c) = &self.body_statements[i - 1]
                    {
                        signal.doc = SignalDoc::from_comment_for(c, type_name, &signal.name);
                    }
                    signals.push(signal);
                }
            }
        }

        signals
    }

    /// Iterate all local variable declarations in the function body recursively
    pub fn iter_local_declarations(&self) -> impl Iterator<Item = &VariableDecl> {
        self.body_statements
            .iter()
            .flat_map(Statement::iter_declarations)
    }

    /// Collect all return values from the function body
    pub fn collect_return_values(&self) -> Vec<&Expression> {
        self.body_statements
            .iter()
            .flat_map(Statement::iter_returns)
            .filter_map(|r| r.value.as_ref())
            .collect()
    }

    /// Check if any variable of the given type is directly returned from the
    /// function
    pub fn is_var_returned(&self, type_info: &TypeInfo) -> bool {
        for stmt in &self.body_statements {
            for ret in stmt.iter_returns() {
                if let Some(Expression::Identifier(id)) = &ret.value {
                    // Find the declaration of this identifier in all body statements
                    for body_stmt in &self.body_statements {
                        for decl in body_stmt.iter_declarations() {
                            if decl.name == id.name
                                && decl.type_info.base_type == type_info.base_type
                                && decl.type_info.is_pointer() == type_info.is_pointer()
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if any variable of the given type is passed to a cleanup call
    /// (g_object_unref, g_free, etc.)
    pub fn is_var_passed_to_cleanup(&self, type_info: &TypeInfo) -> bool {
        for stmt in &self.body_statements {
            for call in stmt.iter_calls() {
                if call.is_cleanup_call()
                    && let Some(arg) = call.get_arg(0)
                    && let Expression::Identifier(id) = arg
                {
                    // Find the declaration of this identifier
                    for body_stmt in &self.body_statements {
                        for decl in body_stmt.iter_declarations() {
                            if decl.name == id.name
                                && decl.type_info.base_type == type_info.base_type
                                && decl.type_info.is_pointer() == type_info.is_pointer()
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if the named variable is passed to a specific function at a
    /// specific argument position
    pub fn is_var_passed_to_function(
        &self,
        var_name: &str,
        func_name: &str,
        arg_index: usize,
    ) -> bool {
        self.body_statements.iter().any(|stmt| {
            stmt.iter_calls().any(|call| {
                call.is_function(func_name)
                    && call.get_arg(arg_index).is_some_and(
                        |arg| matches!(arg, Expression::Identifier(id) if id.name == var_name),
                    )
            })
        })
    }

    /// Check if any variable of the given type is allocated via an allocation
    /// call Uses `call.is_allocation_call()` to detect allocations by
    /// default
    pub fn is_var_allocated(&self, type_info: &TypeInfo) -> bool {
        self.is_var_allocated_with(type_info, CallExpression::is_allocation_call)
    }

    /// Check if any variable of the given type is allocated via a custom
    /// allocation predicate
    pub fn is_var_allocated_with(
        &self,
        type_info: &TypeInfo,
        is_allocation: impl Fn(&CallExpression) -> bool,
    ) -> bool {
        for stmt in &self.body_statements {
            let mut found = false;
            stmt.walk(&mut |s| {
                match s {
                    // Check init: Type *var = allocation_call()
                    Statement::Declaration(decl) => {
                        if decl.type_info.base_type == type_info.base_type
                            && decl.type_info.is_pointer() == type_info.is_pointer()
                            && let Some(Expression::Call(call)) = &decl.initializer
                            && is_allocation(call)
                        {
                            found = true;
                        }
                    }
                    // Check assignment: var = allocation_call()
                    Statement::Expression(expr_stmt) => {
                        if let Expression::Assignment(assign) = expr_stmt.as_ref()
                            && let Expression::Identifier(id) = &*assign.lhs
                            && let Expression::Call(call) = &*assign.rhs
                            && is_allocation(call)
                        {
                            // Find the declaration of the assigned variable
                            for body_stmt in &self.body_statements {
                                for decl in body_stmt.iter_declarations() {
                                    if decl.name == id.name
                                        && decl.type_info.base_type == type_info.base_type
                                        && decl.type_info.is_pointer() == type_info.is_pointer()
                                    {
                                        found = true;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            });
            if found {
                return true;
            }
        }
        false
    }

    /// Find all g_object_class_install_properties calls in the function body
    pub fn find_install_properties_calls(&self) -> Vec<&CallExpression> {
        self.find_calls(&["g_object_class_install_properties"])
    }

    /// Map every named parameter and local variable to its `TypeInfo`.
    /// Parameters appear first; local declarations in body order after that,
    /// so an inner-scope shadowing declaration overwrites the outer one.
    pub fn local_var_types(&self) -> std::collections::HashMap<&str, &TypeInfo> {
        let mut map = std::collections::HashMap::new();
        for param in &self.parameters {
            if let Parameter::Regular {
                name: Some(name),
                type_info,
                ..
            } = param
            {
                map.insert(name.as_str(), type_info);
            }
        }
        for stmt in &self.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Declaration(decl) = s {
                    map.insert(decl.name.as_str(), &decl.type_info);
                }
            });
        }
        map
    }

    /// Get a parameter by name
    pub fn get_param_by_name(&self, name: &str) -> Option<&Parameter> {
        self.parameters
            .iter()
            .find(|p| matches!(p, Parameter::Regular { name: Some(n), .. } if n == name))
    }

    /// Find all param_spec assignments in the function body
    /// Handles array pattern (props[PROP_X] = ...), variable pattern
    /// (param_spec = ...), and override pattern
    /// (g_object_class_override_property(...))
    pub(crate) fn find_param_spec_assignments(&self, type_name: &str) -> Vec<ParamSpecAssignment> {
        let mut assignments = Vec::new();
        let mut array_assignments: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut variable_assignments: HashMap<&str, Vec<usize>> = HashMap::new();

        // First pass: collect all assignments
        for (i, stmt) in self.body_statements.iter().enumerate() {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s {
                    match expr_stmt.as_ref() {
                        // Assignment: props[PROP_X] = g_param_spec_*() or spec = g_param_spec_*()
                        Expression::Assignment(assignment) => {
                            if let Expression::Call(param_call) = &*assignment.rhs {
                                let func_name = param_call.function_name();
                                if !func_name.contains("_param_spec_") {
                                    return;
                                }

                                // Parse property from call
                                let Some(mut property) = Property::from_param_spec_call(param_call)
                                else {
                                    return;
                                };
                                if i > 0
                                    && let Statement::Comment(c) = &self.body_statements[i - 1]
                                {
                                    property.doc =
                                        PropertyDoc::from_comment_for(c, type_name, &property.name);
                                }

                                // Check LHS: array subscript or variable?
                                if let Expression::Subscript(subscript) = &*assignment.lhs {
                                    // Array pattern: props[PROP_X] = g_param_spec_*()
                                    if let Some(array_name) = subscript.array.location().as_str()
                                        && let Some(enum_value) =
                                            subscript.index.location().as_str()
                                    {
                                        let idx = assignments.len();
                                        array_assignments.entry(array_name).or_default().push(idx);
                                        assignments.push(ParamSpecAssignment::ArraySubscript {
                                            array_name: array_name.to_owned(),
                                            enum_value: enum_value.to_owned(),
                                            statement_location: s.location().clone(),
                                            call: param_call.clone(),
                                            property,
                                            install_call: None,
                                        });
                                    }
                                } else if let Some(var_name) = assignment.lhs.location().as_str() {
                                    // Variable pattern: param_spec = g_param_spec_*()
                                    let idx = assignments.len();
                                    variable_assignments.entry(var_name).or_default().push(idx);
                                    assignments.push(ParamSpecAssignment::Variable {
                                        variable_name: var_name.to_owned(),
                                        statement_location: s.location().clone(),
                                        call: param_call.clone(),
                                        property,
                                        install_call: None,
                                    });
                                }
                            }
                        }
                        // Direct call: g_object_class_override_property(class, PROP_X, "name")
                        Expression::Call(call) => {
                            if call.function_contains("override_property")
                                && let Some(mut property) =
                                    Property::from_override_property_call(call)
                                && let Some(enum_arg) = call.get_arg(1)
                                && let Some(enum_value) = enum_arg.location().as_str()
                            {
                                if i > 0
                                    && let Statement::Comment(c) = &self.body_statements[i - 1]
                                {
                                    property.doc =
                                        PropertyDoc::from_comment_for(c, type_name, &property.name);
                                }
                                assignments.push(ParamSpecAssignment::OverrideProperty {
                                    enum_value: enum_value.to_owned(),
                                    statement_location: s.location().clone(),
                                    call: call.clone(),
                                    property,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        // Second pass: find install calls and link them to assignments
        for (i, stmt) in self.body_statements.iter().enumerate() {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s
                    && let Expression::Call(call) = expr_stmt.as_ref()
                {
                    // g_object_class_install_properties(class, N_PROPS, array)
                    if call.function_contains("install_properties") {
                        if let Some(array_arg) = call.get_arg(2)
                            && let Some(array_name) = array_arg.location().as_str()
                            && let Some(indices) = array_assignments.get(&array_name)
                        {
                            for &idx in indices {
                                if let ParamSpecAssignment::ArraySubscript {
                                    install_call, ..
                                } = &mut assignments[idx]
                                {
                                    *install_call = Some(call.clone());
                                }
                            }
                        }
                    }
                    // g_object_class_install_property(class, PROP_X, spec) — 3 args
                    // g_object_interface_install_property(iface, spec) — 2 args
                    else if call.function_contains("install_property") {
                        let is_interface = call.function_contains("interface_install_property");
                        let spec_arg_idx = if is_interface { 1 } else { 2 };

                        if let Some(spec_expr) = call.get_arg(spec_arg_idx) {
                            if let Expression::Call(spec_call) = spec_expr
                                && spec_call.function_contains("_param_spec_")
                                && let Some(mut property) =
                                    Property::from_param_spec_call(spec_call)
                            {
                                let enum_value = if is_interface {
                                    String::new()
                                } else if let Some(enum_arg) = call.get_arg(1)
                                    && let Some(ev) = enum_arg.location().as_str()
                                {
                                    ev.to_owned()
                                } else {
                                    return;
                                };

                                if i > 0
                                    && let Statement::Comment(c) = &self.body_statements[i - 1]
                                {
                                    property.doc = PropertyDoc::from_comment(c);
                                }
                                assignments.push(ParamSpecAssignment::DirectInstall {
                                    enum_value,
                                    statement_location: s.location().clone(),
                                    call: spec_call.clone(),
                                    property,
                                    install_call: call.clone(),
                                });
                            } else if !is_interface
                                && let Some(var_name) = spec_expr.location().as_str()
                                && let Some(indices) = variable_assignments.get(var_name)
                            {
                                let indices = indices.clone();
                                for idx in indices {
                                    if let ParamSpecAssignment::Variable { install_call, .. } =
                                        &mut assignments[idx]
                                        && install_call.is_none()
                                    {
                                        *install_call = Some(call.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        assignments.sort_by_key(|a| a.statement_location().start_byte);
        assignments
    }
}
