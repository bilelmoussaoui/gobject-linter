use serde::{Deserialize, Serialize};

use super::{SourceLocation, Statement};

/// Represents a top-level item in a C file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopLevelItem {
    /// Preprocessor directive (#define, #include, etc.)
    Preprocessor(PreprocessorDirective),
    /// Type definition (typedef, enum, struct)
    TypeDefinition(TypeDefItem),
    /// Function declaration (forward declaration)
    FunctionDeclaration(FunctionDeclItem),
    /// Function definition (with body)
    FunctionDefinition(FunctionDefItem),
    /// Standalone declaration (variables, etc.)
    Declaration(Box<Statement>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PragmaKind {
    /// #pragma once
    Once,
    /// #pragma GCC/clang diagnostic push
    DiagnosticPush,
    /// #pragma GCC/clang diagnostic pop
    DiagnosticPop,
    /// #pragma GCC/clang diagnostic ignored "-Wwarning-name"
    DiagnosticIgnored { warning: String },
    /// Other pragma directive
    Other {
        name: String,
        arguments: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreprocessorDirective {
    Include {
        path: String,
        is_system: bool,
        location: SourceLocation,
    },
    Define {
        name: String,
        value: Option<String>,
        location: SourceLocation,
    },
    Call {
        directive: String,
        location: SourceLocation,
    },
    Pragma {
        kind: PragmaKind,
        location: SourceLocation,
    },
    /// GObject type declaration/definition (G_DECLARE_*, G_DEFINE_*)
    GObjectType {
        gobject_type: Box<super::types::GObjectType>,
        location: SourceLocation,
    },
    /// G_DEFINE_AUTOPTR_CLEANUP_FUNC (Type, cleanup_func)
    AutoptrCleanupFunc {
        type_name: String,
        cleanup_function: String,
        location: SourceLocation,
    },
    /// G_DEFINE_AUTO_CLEANUP_CLEAR_FUNC (Type, cleanup_func)
    AutoCleanupClearFunc {
        type_name: String,
        cleanup_function: String,
        location: SourceLocation,
    },
    /// Macro call with code block (e.g., G_DEFINE_BOXED_TYPE_WITH_CODE)
    /// Contains the macro name and parsed statements from the code block
    MacroWithCode {
        macro_name: String,
        arguments: Vec<String>,
        code_statements: Vec<Statement>,
        location: SourceLocation,
    },
    Conditional {
        kind: ConditionalKind,
        condition: Option<String>,
        body: Vec<TopLevelItem>,
        location: SourceLocation,
    },
    /// G_BEGIN_DECLS ... G_END_DECLS block
    GObjectDeclsBlock {
        body: Vec<TopLevelItem>,
        location: SourceLocation,
    },
}

impl PreprocessorDirective {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::Include { location, .. }
            | Self::Define { location, .. }
            | Self::Call { location, .. }
            | Self::Pragma { location, .. }
            | Self::GObjectType { location, .. }
            | Self::AutoptrCleanupFunc { location, .. }
            | Self::AutoCleanupClearFunc { location, .. }
            | Self::MacroWithCode { location, .. }
            | Self::Conditional { location, .. }
            | Self::GObjectDeclsBlock { location, .. } => location,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionalKind {
    Ifdef,
    Ifndef,
    If,
    Elif,
    Else,
}

/// A parsed field from a struct body (e.g. `GObject parent` → field_type =
/// "GObject")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub field_type: super::TypeInfo,
    /// Field name, if present (anonymous bitfields have none)
    pub field_name: Option<String>,
    pub location: super::SourceLocation,
    /// Bit-width for bitfield members (`unsigned flags : 1` → `Some(1)`).
    #[serde(default)]
    pub bit_width: Option<u32>,
    /// Non-empty for anonymous struct/union fields: the members of the
    /// embedded aggregate (e.g. `union { A a; B b; } d` → inner_fields = [a,
    /// b]).
    #[serde(default)]
    pub inner_fields: Vec<StructField>,
}

impl StructField {
    /// True for future-use padding fields that should never be flagged as dead
    /// code: names starting with `rfu`, `reserved`, `padding`, or `_padding`.
    pub fn is_reserved(&self) -> bool {
        self.field_name.as_deref().is_some_and(|n| {
            n.starts_with("rfu")
                || n.starts_with("reserved")
                || n.starts_with("padding")
                || n.starts_with("_padding")
        })
    }

    /// Visit this field and all nested fields (anonymous struct/union members)
    /// in pre-order, matching the pattern used by `Statement::walk`.
    pub fn walk<F>(&self, f: &mut F)
    where
        F: FnMut(&StructField),
    {
        f(self);
        for inner in &self.inner_fields {
            inner.walk(f);
        }
    }
}

/// The right-hand side of a typedef declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypedefTarget {
    /// Plain type alias: `typedef struct _Foo Foo`, `typedef gint MyInt`.
    Type(super::TypeInfo),
    /// Function-pointer alias: `typedef void (*FooCallback)(GObject *,
    /// gpointer)`.
    Callback {
        return_type: super::TypeInfo,
        parameters: Vec<super::types::Parameter>,
    },
}

impl TypedefTarget {
    /// Return the inner `TypeInfo` if this is a plain type alias.
    pub fn as_type(&self) -> Option<&super::TypeInfo> {
        match self {
            Self::Type(t) => Some(t),
            Self::Callback { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeDefItem {
    Typedef {
        name: String,
        target: TypedefTarget,
        /// Fields when the typedef wraps an inline struct body:
        /// `typedef struct { FieldType field; } Name;`
        #[serde(default)]
        struct_fields: Vec<StructField>,
        location: SourceLocation,
    },
    Struct {
        name: String,
        has_body: bool,
        #[serde(default)]
        fields: Vec<StructField>,
        /// Virtual functions (function pointer fields) extracted from class
        /// structs (structs whose name ends with `Class`).
        #[serde(default)]
        vfuncs: Vec<super::types::VirtualFunction>,
        location: SourceLocation,
    },
    Enum {
        enum_info: Box<super::types::EnumInfo>,
    },
}

impl TypeDefItem {
    /// True for GObject class/interface vtable structs whose fields should not
    /// be checked for dead code: any struct with vfuncs, or any type whose
    /// bare name ends with `Class` or `Interface`.
    pub fn is_vtable_struct(&self) -> bool {
        match self {
            Self::Struct { name, vfuncs, .. } => {
                let bare = name.trim_start_matches('_');
                bare.ends_with("Class") || bare.ends_with("Interface") || !vfuncs.is_empty()
            }
            Self::Typedef { name, .. } => name.ends_with("Class") || name.ends_with("Interface"),
            Self::Enum { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclItem {
    pub name: String,
    pub return_type: super::TypeInfo,
    pub is_static: bool,
    pub is_inline: bool,
    pub parameters: Vec<super::types::Parameter>,
    pub export_macros: Vec<String>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefItem {
    pub name: String,
    pub return_type: super::TypeInfo,
    pub is_static: bool,
    pub is_inline: bool,
    pub parameters: Vec<super::types::Parameter>,
    pub body_statements: Vec<Statement>,
    pub location: SourceLocation,
    pub body_location: Option<SourceLocation>,
}

impl FunctionDefItem {
    /// Find all calls to specific functions in the body
    /// Returns references to all CallExpression nodes that match any of the
    /// given function names
    pub fn find_calls<'a>(
        &'a self,
        function_names: &[&str],
    ) -> Vec<&'a super::expression::CallExpression> {
        self.find_calls_matching(|name| function_names.contains(&name))
    }

    /// Find all calls matching a predicate in the body
    pub fn find_calls_matching<F>(&self, predicate: F) -> Vec<&super::expression::CallExpression>
    where
        F: Fn(&str) -> bool,
    {
        self.body_statements
            .iter()
            .flat_map(|s| s.iter_calls())
            .filter(|call| call.function_name_str().is_some_and(&predicate))
            .collect()
    }

    /// Iterate all local variable declarations in the function body recursively
    pub fn iter_local_declarations(&self) -> impl Iterator<Item = &super::statement::VariableDecl> {
        self.body_statements
            .iter()
            .flat_map(|s| s.iter_declarations())
    }

    /// Collect all return values from the function body
    pub fn collect_return_values(&self) -> Vec<&super::expression::Expression> {
        self.body_statements
            .iter()
            .flat_map(|s| s.iter_returns())
            .filter_map(|r| r.value.as_ref())
            .collect()
    }

    /// Check if any variable of the given type is directly returned from the
    /// function
    pub fn is_var_returned(&self, type_info: &crate::TypeInfo) -> bool {
        use super::expression::Expression;

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
    pub fn is_var_passed_to_cleanup(&self, type_info: &crate::TypeInfo) -> bool {
        use super::expression::Expression;

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
        use super::expression::Expression;

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
    pub fn is_var_allocated(&self, type_info: &crate::TypeInfo) -> bool {
        self.is_var_allocated_with(type_info, |call| call.is_allocation_call())
    }

    /// Check if any variable of the given type is allocated via a custom
    /// allocation predicate
    pub fn is_var_allocated_with(
        &self,
        type_info: &crate::TypeInfo,
        is_allocation: impl Fn(&super::expression::CallExpression) -> bool,
    ) -> bool {
        use super::expression::Expression;

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
                        if let Expression::Assignment(assign) = &expr_stmt.expr
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
    pub fn find_install_properties_calls(&self) -> Vec<&super::expression::CallExpression> {
        self.find_calls(&["g_object_class_install_properties"])
    }

    /// Map every named parameter and local variable to its `TypeInfo`.
    /// Parameters appear first; local declarations in body order after that,
    /// so an inner-scope shadowing declaration overwrites the outer one.
    pub fn local_var_types(&self) -> std::collections::HashMap<String, super::TypeInfo> {
        let mut map = std::collections::HashMap::new();
        for param in &self.parameters {
            if let super::types::Parameter::Regular {
                name: Some(name),
                type_info,
                ..
            } = param
            {
                map.insert(name.clone(), type_info.clone());
            }
        }
        for stmt in &self.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Declaration(decl) = s {
                    map.insert(decl.name.clone(), decl.type_info.clone());
                }
            });
        }
        map
    }

    /// Get a parameter by name
    pub fn get_param_by_name(&self, name: &str) -> Option<&super::types::Parameter> {
        self.parameters.iter().find(
            |p| matches!(p, super::types::Parameter::Regular { name: Some(n), .. } if n == name),
        )
    }

    /// Find all param_spec assignments in the function body
    /// Handles array pattern (props[PROP_X] = ...), variable pattern
    /// (param_spec = ...), and override pattern
    /// (g_object_class_override_property(...))
    pub fn find_param_spec_assignments(
        &self,
        source: &[u8],
    ) -> Vec<super::types::ParamSpecAssignment> {
        use std::collections::HashMap;

        use super::{
            Statement,
            expression::Expression,
            types::{ParamSpecAssignment, Property},
        };

        let mut assignments = Vec::new();
        let mut array_assignments: HashMap<String, Vec<usize>> = HashMap::new();
        let mut variable_assignments: HashMap<String, Vec<usize>> = HashMap::new();

        // First pass: collect all assignments
        for stmt in &self.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s {
                    match &expr_stmt.expr {
                        // Assignment: props[PROP_X] = g_param_spec_*() or spec = g_param_spec_*()
                        Expression::Assignment(assignment) => {
                            if let Expression::Call(param_call) = &*assignment.rhs {
                                let func_name = param_call.function_name();
                                if !func_name.contains("_param_spec_") {
                                    return;
                                }

                                // Parse property from call
                                let Some(property) = Property::from_param_spec_call(param_call)
                                else {
                                    return;
                                };

                                // Check LHS: array subscript or variable?
                                if let Expression::Subscript(subscript) = &*assignment.lhs {
                                    // Array pattern: props[PROP_X] = g_param_spec_*()
                                    if let Some(array_name) =
                                        subscript.array.to_source_string(source)
                                        && let Some(enum_value) =
                                            subscript.index.to_source_string(source)
                                    {
                                        let idx = assignments.len();
                                        array_assignments
                                            .entry(array_name.clone())
                                            .or_default()
                                            .push(idx);
                                        assignments.push(ParamSpecAssignment::ArraySubscript {
                                            array_name,
                                            enum_value,
                                            property_name: property.name.clone(),
                                            statement_location: *s.location(),
                                            call: param_call.clone(),
                                            property,
                                            install_call: None,
                                        });
                                    }
                                } else if let Some(var_name) =
                                    assignment.lhs.to_source_string(source)
                                {
                                    // Variable pattern: param_spec = g_param_spec_*()
                                    let idx = assignments.len();
                                    variable_assignments
                                        .entry(var_name.clone())
                                        .or_default()
                                        .push(idx);
                                    assignments.push(ParamSpecAssignment::Variable {
                                        variable_name: var_name,
                                        property_name: property.name.clone(),
                                        statement_location: *s.location(),
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
                                && let Some(property) = Property::from_override_property_call(call)
                                && let Some(enum_arg) = call.get_arg(1)
                                && let Some(enum_value) = enum_arg.to_source_string(source)
                            {
                                assignments.push(ParamSpecAssignment::OverrideProperty {
                                    enum_value,
                                    property_name: property.name.clone(),
                                    statement_location: *s.location(),
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
        for stmt in &self.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s
                    && let Expression::Call(call) = &expr_stmt.expr
                {
                    // g_object_class_install_properties(class, N_PROPS, array)
                    if call.function_contains("install_properties") {
                        if let Some(array_arg) = call.get_arg(2)
                            && let Some(array_name) = array_arg.to_source_string(source)
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
                    // g_object_class_install_property(class, PROP_X, spec)
                    else if call.function_contains("install_property")
                        && let Some(spec_expr) = call.get_arg(2)
                    {
                        if let Expression::Call(spec_call) = spec_expr
                            && spec_call.function_name().contains("_param_spec_")
                            && let Some(enum_arg) = call.get_arg(1)
                            && let Some(enum_value) = enum_arg.to_source_string(source)
                            && let Some(property) = Property::from_param_spec_call(spec_call)
                        {
                            // Inline param_spec: g_object_class_install_property(class, PROP_X,
                            // g_param_spec_*(...))
                            assignments.push(ParamSpecAssignment::DirectInstall {
                                enum_value,
                                property_name: property.name.clone(),
                                statement_location: *s.location(),
                                call: spec_call.clone(),
                                property,
                                install_call: call.clone(),
                            });
                        } else if let Some(var_name) = spec_expr.to_source_string(source)
                            && let Some(indices) = variable_assignments.get(&var_name)
                        {
                            let indices = indices.clone();
                            for idx in indices {
                                if let ParamSpecAssignment::Variable { install_call, .. } =
                                    &mut assignments[idx]
                                {
                                    *install_call = Some(call.clone());
                                }
                            }
                        }
                    }
                }
            });
        }

        assignments
    }
}
