use std::collections::HashMap;

use clap::ValueEnum;
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    EnumInfo, GObjectType, TypeInfo, VariableDecl, VirtualFunction,
    model::{
        SourceLocation, Statement,
        doc::{FunctionDoc, PropertyDoc, SignalDoc, TypeDoc},
        expression::{CallExpression, Expression},
        types::{ParamSpecAssignment, Parameter, Property, Signal},
    },
};

/// Coarse kind of a top-level item, useful for filtering without
/// pattern-matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TopLevelItemKind {
    FunctionDefinition,
    FunctionDeclaration,
    Typedef,
    Struct,
    Enum,
    Include,
    Define,
    GObjectType,
    Conditional,
    GObjectDeclsBlock,
    Declaration,
    Expression,
    Other,
}

/// Represents a top-level item in a C file
#[derive(Debug, Clone)]
pub enum TopLevelItem {
    /// Preprocessor directive (#define, #include, etc.)
    Preprocessor(PreprocessorDirective),
    /// Type definition (typedef, enum, struct)
    TypeDefinition(TypeDefItem),
    /// Function declaration (forward declaration)
    FunctionDeclaration(FunctionDeclItem),
    /// Function definition (with body)
    FunctionDefinition(FunctionDefItem),
    /// Standalone variable declaration
    Declaration(Box<VariableDecl>),
    /// Standalone expression statement
    Expression(Box<Expression>),
}

impl TopLevelItem {
    /// The name associated with this item, if any.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::FunctionDefinition(f) => Some(&f.name),
            Self::FunctionDeclaration(f) => Some(&f.name),
            Self::TypeDefinition(td) => match td {
                TypeDefItem::Typedef { name, .. } | TypeDefItem::Struct { name, .. } => Some(name),
                TypeDefItem::Enum(enum_info) => enum_info.name.as_deref(),
            },
            Self::Preprocessor(PreprocessorDirective::Include { path, .. }) => Some(path),
            Self::Preprocessor(PreprocessorDirective::Define { name, .. }) => Some(name),
            Self::Preprocessor(PreprocessorDirective::GObjectType(gobject_type)) => {
                Some(&gobject_type.type_name)
            }
            Self::Declaration(decl) => Some(&decl.name),
            _ => None,
        }
    }

    /// The coarse kind of this item.
    pub fn kind(&self) -> TopLevelItemKind {
        match self {
            Self::FunctionDefinition(_) => TopLevelItemKind::FunctionDefinition,
            Self::FunctionDeclaration(_) => TopLevelItemKind::FunctionDeclaration,
            Self::TypeDefinition(TypeDefItem::Typedef { .. }) => TopLevelItemKind::Typedef,
            Self::TypeDefinition(TypeDefItem::Struct { .. }) => TopLevelItemKind::Struct,
            Self::TypeDefinition(TypeDefItem::Enum(_)) => TopLevelItemKind::Enum,
            Self::Preprocessor(PreprocessorDirective::Include { .. }) => TopLevelItemKind::Include,
            Self::Preprocessor(PreprocessorDirective::Define { .. }) => TopLevelItemKind::Define,
            Self::Preprocessor(PreprocessorDirective::GObjectType { .. }) => {
                TopLevelItemKind::GObjectType
            }
            Self::Preprocessor(PreprocessorDirective::Conditional { .. }) => {
                TopLevelItemKind::Conditional
            }
            Self::Preprocessor(PreprocessorDirective::GObjectDeclsBlock { .. }) => {
                TopLevelItemKind::GObjectDeclsBlock
            }
            Self::Declaration(_) => TopLevelItemKind::Declaration,
            Self::Expression(_) => TopLevelItemKind::Expression,
            Self::Preprocessor(_) => TopLevelItemKind::Other,
        }
    }
}

impl Serialize for TopLevelItem {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Preprocessor(directive) => directive.serialize(s),
            Self::TypeDefinition(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("type_definition", v)?;
                m.end()
            }
            Self::FunctionDeclaration(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("function_declaration", v)?;
                m.end()
            }
            Self::FunctionDefinition(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("function_definition", v)?;
                m.end()
            }
            Self::Declaration(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("declaration", v)?;
                m.end()
            }
            Self::Expression(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("expression", v)?;
                m.end()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
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
    GObjectType(Box<GObjectType>),
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
            | Self::AutoptrCleanupFunc { location, .. }
            | Self::AutoCleanupClearFunc { location, .. }
            | Self::MacroWithCode { location, .. }
            | Self::Conditional { location, .. }
            | Self::GObjectDeclsBlock { location, .. } => location,
            Self::GObjectType(gt) => &gt.location,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalKind {
    Ifdef,
    Ifndef,
    If,
    Elif,
    Else,
}

/// A parsed field from a struct body (e.g. `GObject parent` → field_type =
/// "GObject")
#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub field_type: TypeInfo,
    /// Field name, if present (anonymous bitfields have none)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    pub location: SourceLocation,
    /// Bit-width for bitfield members (`unsigned flags : 1` → `Some(1)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_width: Option<u32>,
    /// Non-empty for anonymous struct/union fields: the members of the
    /// embedded aggregate (e.g. `union { A a; B b; } d` → inner_fields = [a,
    /// b]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_fields: Vec<Self>,
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
        F: FnMut(&Self),
    {
        f(self);
        for inner in &self.inner_fields {
            inner.walk(f);
        }
    }
}

/// The right-hand side of a typedef declaration.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedefTarget {
    /// Plain type alias: `typedef struct _Foo Foo`, `typedef gint MyInt`.
    Type(TypeInfo),
    /// Function-pointer alias: `typedef void (*FooCallback)(GObject *,
    /// gpointer)`.
    Callback {
        return_type: TypeInfo,
        parameters: Vec<Parameter>,
    },
}

impl TypedefTarget {
    /// Return the inner `TypeInfo` if this is a plain type alias.
    pub fn as_type(&self) -> Option<&TypeInfo> {
        match self {
            Self::Type(t) => Some(t),
            Self::Callback { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeDefItem {
    Typedef {
        name: String,
        target: TypedefTarget,
        /// Fields when the typedef wraps an inline struct body:
        /// `typedef struct { FieldType field; } Name;`
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        struct_fields: Vec<StructField>,
        location: SourceLocation,
        #[serde(skip_serializing_if = "Option::is_none")]
        doc: Option<TypeDoc>,
    },
    Struct {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fields: Vec<StructField>,
        /// Virtual functions (function pointer fields) extracted from class
        /// structs (structs whose name ends with `Class`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        vfuncs: Vec<VirtualFunction>,
        location: SourceLocation,
        #[serde(skip_serializing_if = "Option::is_none")]
        doc: Option<TypeDoc>,
    },
    Enum(Box<EnumInfo>),
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
    pub export_macros: Vec<String>,
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
        self.body_statements
            .iter()
            .flat_map(Statement::iter_calls)
            .filter(|call| call.function_name_str().is_some_and(&predicate))
            .collect()
    }

    /// Extract signal registrations from the function body.
    /// Populates `enum_value` when the signal is assigned via
    /// `signals[ENUM] = g_signal_new(...)`.
    pub fn find_signal_registrations(&self, source: &[u8]) -> Vec<Signal> {
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
                let Some(mut signal) = Signal::from_g_signal_new_call(call, source) else {
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
                    signal.doc = SignalDoc::from_comment(c);
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
                if let Some(mut signal) = Signal::from_g_signal_new_call(call, source) {
                    if i > 0
                        && let Statement::Comment(c) = &self.body_statements[i - 1]
                    {
                        signal.doc = SignalDoc::from_comment(c);
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
    pub fn local_var_types(&self) -> std::collections::HashMap<String, TypeInfo> {
        let mut map = std::collections::HashMap::new();
        for param in &self.parameters {
            if let Parameter::Regular {
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
    pub fn get_param_by_name(&self, name: &str) -> Option<&Parameter> {
        self.parameters
            .iter()
            .find(|p| matches!(p, Parameter::Regular { name: Some(n), .. } if n == name))
    }

    /// Find all param_spec assignments in the function body
    /// Handles array pattern (props[PROP_X] = ...), variable pattern
    /// (param_spec = ...), and override pattern
    /// (g_object_class_override_property(...))
    pub(crate) fn find_param_spec_assignments(&self, source: &[u8]) -> Vec<ParamSpecAssignment> {
        let mut assignments = Vec::new();
        let mut array_assignments: HashMap<String, Vec<usize>> = HashMap::new();
        let mut variable_assignments: HashMap<String, Vec<usize>> = HashMap::new();

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
                                    property.doc = PropertyDoc::from_comment(c);
                                }

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
                                && let Some(mut property) =
                                    Property::from_override_property_call(call)
                                && let Some(enum_arg) = call.get_arg(1)
                                && let Some(enum_value) = enum_arg.to_source_string(source)
                            {
                                if i > 0
                                    && let Statement::Comment(c) = &self.body_statements[i - 1]
                                {
                                    property.doc = PropertyDoc::from_comment(c);
                                }
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
        for (i, stmt) in self.body_statements.iter().enumerate() {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s
                    && let Expression::Call(call) = expr_stmt.as_ref()
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
                            && let Some(mut property) = Property::from_param_spec_call(spec_call)
                        {
                            if i > 0
                                && let Statement::Comment(c) = &self.body_statements[i - 1]
                            {
                                property.doc = PropertyDoc::from_comment(c);
                            }
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
