use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use gobject_ast::model::{
    AssignmentOp, BinaryOp, CallExpression, Expression, FunctionDefItem, Parameter, Statement,
    TopLevelItem, TypeDefItem, TypedefTarget, UnaryOp,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionContext {
    KnownCoroutine,
    KnownNonCoroutine,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallPolicy {
    Coroutine,
    NonCoroutine,
}

impl CallPolicy {
    fn annotation_name(self) -> &'static str {
        match self {
            Self::Coroutine => "coroutine_fn",
            Self::NonCoroutine => "no_coroutine_fn",
        }
    }

    fn accepts(self, context: ExecutionContext) -> bool {
        matches!(
            (self, context),
            (Self::Coroutine, ExecutionContext::KnownCoroutine)
                | (Self::NonCoroutine, ExecutionContext::KnownNonCoroutine)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    Restricted(CallPolicy),
    Unrestricted,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default)]
struct AnnotationSet {
    coroutine: bool,
    mixed: bool,
    no_coroutine: bool,
}

impl AnnotationSet {
    fn from_modifiers(modifiers: &[String]) -> Self {
        let mut result = Self::default();
        for modifier in modifiers {
            match modifier.as_str() {
                "coroutine_fn" => result.coroutine = true,
                "coroutine_mixed_fn" => result.mixed = true,
                "no_coroutine_fn" | "co_wrapper" | "co_wrapper_bdrv_rdlock" => {
                    result.no_coroutine = true;
                }
                "co_wrapper_mixed" | "co_wrapper_mixed_bdrv_rdlock" => {
                    result.no_coroutine = true;
                    result.mixed = true;
                }
                // no_co_wrapper variants drive QEMU's code generator. They do
                // not alter the coroutine policy of the accompanying function.
                "no_co_wrapper" | "no_co_wrapper_bdrv_rdlock" | "no_co_wrapper_bdrv_wrlock" => {}
                _ => {}
            }
        }
        result
    }

    fn resolution(self) -> Option<Resolution> {
        match (self.coroutine, self.mixed, self.no_coroutine) {
            (false, false, false) => None,
            (true, false, false) => Some(Resolution::Restricted(CallPolicy::Coroutine)),
            (false, false, true) => Some(Resolution::Restricted(CallPolicy::NonCoroutine)),
            (false, true, false) => Some(Resolution::Unrestricted),
            (false, true, true) => Some(Resolution::Unrestricted),
            _ => Some(Resolution::Unknown),
        }
    }

    fn body_context(self) -> ExecutionContext {
        match (self.coroutine, self.mixed, self.no_coroutine) {
            (true, false, false) => ExecutionContext::KnownCoroutine,
            (false, true, _) => ExecutionContext::Mixed,
            _ => ExecutionContext::KnownNonCoroutine,
        }
    }
}

#[derive(Debug, Clone)]
struct FieldFact {
    field_type: String,
    callable: Resolution,
}

#[derive(Default)]
struct CoroutineProgram {
    external_functions: HashMap<String, Resolution>,
    file_local_functions: HashMap<PathBuf, HashMap<String, Resolution>>,
    callback_types: HashMap<String, Resolution>,
    aliases: HashMap<String, String>,
    fields: HashMap<String, HashMap<String, FieldFact>>,
}

impl CoroutineProgram {
    fn build(
        ast_context: &AstContext,
        rule: &QemuCoroutineFn,
        violations: &mut Vec<Violation>,
    ) -> Self {
        let mut program = Self::default();

        for (_path, file) in ast_context.iter_all_files() {
            for item in file.iter_all_items() {
                match item {
                    TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
                        name,
                        target:
                            TypedefTarget::Callback {
                                macro_modifiers, ..
                            },
                        ..
                    }) => {
                        if let Some(resolution) =
                            AnnotationSet::from_modifiers(macro_modifiers).resolution()
                        {
                            program.callback_types.insert(name.clone(), resolution);
                        }
                    }
                    TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
                        name,
                        target: TypedefTarget::Type(target),
                        ..
                    }) => {
                        program
                            .aliases
                            .insert(name.clone(), target.base_type.clone());
                    }
                    _ => {}
                }
            }
        }

        for (_path, file) in ast_context.iter_all_files() {
            for item in file.iter_all_items() {
                match item {
                    TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
                        name,
                        struct_fields,
                        ..
                    }) => program.add_fields(name, struct_fields),
                    TopLevelItem::TypeDefinition(TypeDefItem::Struct { name, fields, .. }) => {
                        program.add_fields(name, fields);
                    }
                    _ => {}
                }
            }
        }

        for (path, file) in ast_context.iter_all_files() {
            for declaration in file.iter_function_declarations() {
                let Some(resolution) =
                    AnnotationSet::from_modifiers(&declaration.macro_modifiers).resolution()
                else {
                    continue;
                };
                if resolution == Resolution::Unknown {
                    violations.push(rule.violation_at(
                        path,
                        &declaration.location,
                        format!(
                            "declaration of `{}` has an invalid coroutine annotation combination",
                            declaration.name,
                        ),
                    ));
                } else if let Some(previous) =
                    program.function_exact(path, &declaration.name, declaration.is_static)
                    && previous != resolution
                    && previous != Resolution::Unknown
                {
                    violations.push(rule.violation_at(
                        path,
                        &declaration.location,
                        format!(
                            "declaration of `{}` has a different coroutine contract from an earlier declaration",
                            declaration.name,
                        ),
                    ));
                }
                program.merge_function(path, &declaration.name, declaration.is_static, resolution);
            }
        }

        for (path, file) in ast_context.iter_all_files() {
            for definition in file.iter_function_definitions() {
                let Some(resolution) =
                    AnnotationSet::from_modifiers(&definition.macro_modifiers).resolution()
                else {
                    continue;
                };
                if resolution == Resolution::Unknown {
                    violations.push(rule.violation_at(
                        path,
                        &definition.location,
                        format!(
                            "definition of `{}` has an invalid coroutine annotation combination",
                            definition.name,
                        ),
                    ));
                    program.insert_function(
                        path,
                        &definition.name,
                        definition.is_static,
                        Resolution::Unknown,
                    );
                } else if let Some(previous) =
                    program.function_exact(path, &definition.name, definition.is_static)
                    && previous != resolution
                    && previous != Resolution::Unknown
                {
                    violations.push(rule.violation_at(
                        path,
                        &definition.location,
                        format!(
                            "definition of `{}` is {} but its declaration has a different coroutine contract",
                            definition.name,
                            Self::resolution_name(resolution),
                        ),
                    ));
                    program.insert_function(
                        path,
                        &definition.name,
                        definition.is_static,
                        Resolution::Unknown,
                    );
                } else {
                    program.merge_function(
                        path,
                        &definition.name,
                        definition.is_static,
                        resolution,
                    );
                }
            }
        }

        program
    }

    fn resolution_name(resolution: Resolution) -> &'static str {
        match resolution {
            Resolution::Restricted(policy) => policy.annotation_name(),
            Resolution::Unrestricted => "coroutine_mixed_fn",
            Resolution::Unknown => "an invalid annotation combination",
        }
    }

    fn function_map_mut(
        &mut self,
        file: &Path,
        is_static: bool,
    ) -> &mut HashMap<String, Resolution> {
        if is_static {
            self.file_local_functions
                .entry(file.to_path_buf())
                .or_default()
        } else {
            &mut self.external_functions
        }
    }

    fn insert_function(
        &mut self,
        file: &Path,
        name: &str,
        is_static: bool,
        resolution: Resolution,
    ) {
        self.function_map_mut(file, is_static)
            .insert(name.to_owned(), resolution);
    }

    fn merge_function(&mut self, file: &Path, name: &str, is_static: bool, resolution: Resolution) {
        self.function_map_mut(file, is_static)
            .entry(name.to_owned())
            .and_modify(|current| {
                if *current != resolution {
                    *current = Resolution::Unknown;
                }
            })
            .or_insert(resolution);
    }

    fn add_fields(&mut self, owner: &str, fields: &[gobject_ast::model::StructField]) {
        for field in fields {
            if let Some(name) = &field.field_name {
                let callable = field
                    .callable
                    .as_ref()
                    .and_then(|signature| {
                        AnnotationSet::from_modifiers(&signature.macro_modifiers).resolution()
                    })
                    .or_else(|| self.callback_type(&field.field_type.base_type))
                    .unwrap_or(Resolution::Unrestricted);
                self.fields.entry(owner.to_owned()).or_default().insert(
                    name.clone(),
                    FieldFact {
                        field_type: field.field_type.base_type.clone(),
                        callable,
                    },
                );
            }
            self.add_fields(owner, &field.inner_fields);
        }
    }

    fn canonical_type<'a>(&'a self, type_name: &'a str) -> Option<&'a str> {
        let mut current = type_name;
        for _ in 0..=self.aliases.len() {
            let Some(next) = self.aliases.get(current) else {
                return Some(current);
            };
            current = next;
        }
        None
    }

    fn function(&self, file: &Path, name: &str) -> Option<Resolution> {
        self.file_local_functions
            .get(file)
            .and_then(|functions| functions.get(name))
            .or_else(|| self.external_functions.get(name))
            .copied()
    }

    fn function_exact(&self, file: &Path, name: &str, is_static: bool) -> Option<Resolution> {
        if is_static {
            self.file_local_functions
                .get(file)
                .and_then(|functions| functions.get(name))
                .copied()
        } else {
            self.external_functions.get(name).copied()
        }
    }

    fn callback_type(&self, type_name: &str) -> Option<Resolution> {
        self.callback_types
            .get(type_name)
            .or_else(|| {
                self.canonical_type(type_name)
                    .and_then(|canonical| self.callback_types.get(canonical))
            })
            .copied()
    }

    fn field(&self, owner: &str, field: &str) -> Option<&FieldFact> {
        self.fields
            .get(owner)
            .and_then(|fields| fields.get(field))
            .or_else(|| {
                self.canonical_type(owner)
                    .and_then(|canonical| self.fields.get(canonical))
                    .and_then(|fields| fields.get(field))
            })
    }
}

#[derive(Debug, Clone)]
struct FlowState {
    context: ExecutionContext,
    variable_types: HashMap<String, String>,
    assigned_callables: HashMap<String, Resolution>,
}

impl FlowState {
    fn new(context: ExecutionContext, function: &FunctionDefItem) -> Self {
        let mut variable_types = HashMap::new();
        for parameter in &function.parameters {
            if let Parameter::Regular {
                name: Some(name),
                type_info,
                ..
            } = parameter
            {
                variable_types.insert(name.clone(), type_info.base_type.clone());
            }
        }
        Self {
            context,
            variable_types,
            assigned_callables: HashMap::new(),
        }
    }

    fn join(&mut self, other: &Self) {
        if self.context != other.context {
            self.context = ExecutionContext::Mixed;
        }
        self.variable_types
            .retain(|name, ty| other.variable_types.get(name) == Some(ty));
        let keys: Vec<String> = self.assigned_callables.keys().cloned().collect();
        for key in keys {
            let other_value = other.assigned_callables.get(&key).copied();
            if other_value != self.assigned_callables.get(&key).copied() {
                self.assigned_callables.insert(key, Resolution::Unknown);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GuardInfo {
    then_context: Option<ExecutionContext>,
    else_context: Option<ExecutionContext>,
}

pub struct QemuCoroutineFn;

impl QemuCoroutineFn {
    fn qemu_in_coroutine(expr: &Expression) -> Option<bool> {
        match expr {
            Expression::Call(call) if call.function_name_str() == Some("qemu_in_coroutine") => {
                Some(true)
            }
            Expression::Unary(unary) if unary.operator == UnaryOp::Not => {
                Self::qemu_in_coroutine(&unary.operand).map(|value| !value)
            }
            _ => None,
        }
    }

    fn analyze_guard(expr: &Expression) -> Option<GuardInfo> {
        if let Some(value) = Self::qemu_in_coroutine(expr) {
            return Some(GuardInfo {
                then_context: Some(if value {
                    ExecutionContext::KnownCoroutine
                } else {
                    ExecutionContext::KnownNonCoroutine
                }),
                else_context: Some(if value {
                    ExecutionContext::KnownNonCoroutine
                } else {
                    ExecutionContext::KnownCoroutine
                }),
            });
        }
        let Expression::Binary(binary) = expr else {
            return None;
        };
        match binary.operator {
            BinaryOp::LogicalAnd => {
                let context = Self::analyze_guard(&binary.left)
                    .and_then(|guard| guard.then_context)
                    .or_else(|| {
                        Self::analyze_guard(&binary.right).and_then(|guard| guard.then_context)
                    });
                context.map(|then_context| GuardInfo {
                    then_context: Some(then_context),
                    else_context: None,
                })
            }
            BinaryOp::LogicalOr => {
                let context = Self::analyze_guard(&binary.left)
                    .and_then(|guard| guard.else_context)
                    .or_else(|| {
                        Self::analyze_guard(&binary.right).and_then(|guard| guard.else_context)
                    });
                context.map(|else_context| GuardInfo {
                    then_context: None,
                    else_context: Some(else_context),
                })
            }
            _ => None,
        }
    }

    fn assertion_context(statement: &Statement) -> Option<ExecutionContext> {
        let Statement::Expression(expression) = statement else {
            return None;
        };
        let Expression::Call(call) = expression.as_ref() else {
            return None;
        };
        if !matches!(call.function_name_str(), Some("assert" | "g_assert")) {
            return None;
        }
        call.arguments.iter().find_map(|argument| {
            Self::qemu_in_coroutine(argument).map(|value| {
                if value {
                    ExecutionContext::KnownCoroutine
                } else {
                    ExecutionContext::KnownNonCoroutine
                }
            })
        })
    }

    fn block_exits(statements: &[Statement]) -> bool {
        statements.iter().any(|statement| match statement {
            Statement::Return(_) | Statement::Break(_) | Statement::Goto(_) => true,
            Statement::If(if_statement) => if_statement.else_body.as_ref().is_some_and(|body| {
                Self::block_exits(&if_statement.then_body) && Self::block_exits(body)
            }),
            _ => false,
        })
    }

    fn infer_type(
        expression: &Expression,
        state: &FlowState,
        program: &CoroutineProgram,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(identifier) => {
                state.variable_types.get(&identifier.name).cloned()
            }
            Expression::FieldAccess(field) => {
                let owner = Self::infer_type(&field.base, state, program)?;
                program
                    .field(&owner, &field.field)
                    .map(|fact| fact.field_type.clone())
            }
            Expression::Unary(unary)
                if matches!(unary.operator, UnaryOp::Dereference | UnaryOp::AddressOf) =>
            {
                Self::infer_type(&unary.operand, state, program)
            }
            Expression::Subscript(subscript) => Self::infer_type(&subscript.array, state, program),
            Expression::Cast(cast) => Some(cast.type_info.base_type.clone()),
            _ => None,
        }
    }

    fn resolve_callee(
        expression: &Expression,
        file: &Path,
        state: &FlowState,
        program: &CoroutineProgram,
    ) -> Resolution {
        match expression {
            Expression::Identifier(identifier) => {
                if let Some(resolution) = state.assigned_callables.get(&identifier.name) {
                    return *resolution;
                }
                if let Some(type_name) = state.variable_types.get(&identifier.name)
                    && let Some(resolution) = program.callback_type(type_name)
                {
                    return resolution;
                }
                program
                    .function(file, &identifier.name)
                    .unwrap_or(Resolution::Unrestricted)
            }
            Expression::Unary(unary)
                if matches!(unary.operator, UnaryOp::Dereference | UnaryOp::AddressOf) =>
            {
                Self::resolve_callee(&unary.operand, file, state, program)
            }
            Expression::FieldAccess(field) => {
                let Some(owner) = Self::infer_type(&field.base, state, program) else {
                    return Resolution::Unknown;
                };
                program
                    .field(&owner, &field.field)
                    .map_or(Resolution::Unknown, |fact| fact.callable)
            }
            Expression::Subscript(subscript) => {
                let Some(type_name) = Self::infer_type(&subscript.array, state, program) else {
                    return Resolution::Unknown;
                };
                program
                    .callback_type(&type_name)
                    .unwrap_or(Resolution::Unknown)
            }
            Expression::Cast(cast) => program
                .callback_type(&cast.type_info.base_type)
                .unwrap_or_else(|| Self::resolve_callee(&cast.operand, file, state, program)),
            _ => Resolution::Unknown,
        }
    }

    fn check_call(
        &self,
        file: &Path,
        context: ExecutionContext,
        call: &CallExpression,
        resolution: Resolution,
        violations: &mut Vec<Violation>,
    ) {
        let Resolution::Restricted(policy) = resolution else {
            return;
        };
        if policy.accepts(context) {
            return;
        }
        let callee = call
            .function
            .location()
            .as_str()
            .unwrap_or("<indirect call>");
        violations.push(self.violation_at(
            file,
            &call.location,
            format!(
                "calling {} `{}` from {} context",
                policy.annotation_name(),
                callee,
                match context {
                    ExecutionContext::KnownCoroutine => "coroutine_fn",
                    ExecutionContext::KnownNonCoroutine => "non-coroutine",
                    ExecutionContext::Mixed => "coroutine_mixed_fn",
                },
            ),
        ));
    }

    fn check_expression_calls(
        &self,
        file: &Path,
        expression: &Expression,
        state: &FlowState,
        program: &CoroutineProgram,
        violations: &mut Vec<Violation>,
    ) {
        expression.walk(&mut |nested| {
            if let Expression::Call(call) = nested {
                let resolution = Self::resolve_callee(&call.function, file, state, program);
                self.check_call(file, state.context, call, resolution, violations);
            }
        });
    }

    fn apply_assignments(
        expression: &Expression,
        file: &Path,
        state: &mut FlowState,
        program: &CoroutineProgram,
    ) {
        expression.walk(&mut |nested| {
            let Expression::Assignment(assignment) = nested else {
                return;
            };
            let Expression::Identifier(identifier) = assignment.lhs.as_ref() else {
                return;
            };
            let resolution = if assignment.operator == AssignmentOp::Assign {
                Self::resolve_callee(&assignment.rhs, file, state, program)
            } else {
                Resolution::Unknown
            };
            state
                .assigned_callables
                .insert(identifier.name.clone(), resolution);
        });
    }

    fn check_condition_calls(
        &self,
        file: &Path,
        expression: &Expression,
        state: &FlowState,
        program: &CoroutineProgram,
        violations: &mut Vec<Violation>,
    ) {
        if let Expression::Binary(binary) = expression {
            match binary.operator {
                BinaryOp::LogicalAnd => {
                    self.check_condition_calls(file, &binary.left, state, program, violations);
                    let mut right_state = state.clone();
                    if let Some(context) =
                        Self::analyze_guard(&binary.left).and_then(|guard| guard.then_context)
                    {
                        right_state.context = context;
                    }
                    self.check_condition_calls(
                        file,
                        &binary.right,
                        &right_state,
                        program,
                        violations,
                    );
                    return;
                }
                BinaryOp::LogicalOr => {
                    self.check_condition_calls(file, &binary.left, state, program, violations);
                    let mut right_state = state.clone();
                    if let Some(context) =
                        Self::analyze_guard(&binary.left).and_then(|guard| guard.else_context)
                    {
                        right_state.context = context;
                    }
                    self.check_condition_calls(
                        file,
                        &binary.right,
                        &right_state,
                        program,
                        violations,
                    );
                    return;
                }
                _ => {}
            }
        }
        self.check_expression_calls(file, expression, state, program, violations);
    }

    fn check_statements(
        &self,
        file: &Path,
        statements: &[Statement],
        state: &mut FlowState,
        program: &CoroutineProgram,
        violations: &mut Vec<Violation>,
    ) {
        for statement in statements {
            if let Some(context) = Self::assertion_context(statement) {
                state.context = context;
                continue;
            }

            if let Statement::If(if_statement) = statement
                && let Some(guard) = Self::analyze_guard(&if_statement.condition)
            {
                self.check_condition_calls(
                    file,
                    &if_statement.condition,
                    state,
                    program,
                    violations,
                );
                let mut then_state = state.clone();
                if let Some(context) = guard.then_context {
                    then_state.context = context;
                }
                self.check_statements(
                    file,
                    &if_statement.then_body,
                    &mut then_state,
                    program,
                    violations,
                );

                let mut else_state = state.clone();
                if let Some(context) = guard.else_context {
                    else_state.context = context;
                }
                if let Some(else_body) = &if_statement.else_body {
                    self.check_statements(file, else_body, &mut else_state, program, violations);
                }

                let then_exits = Self::block_exits(&if_statement.then_body);
                let else_exits = if_statement
                    .else_body
                    .as_ref()
                    .is_some_and(|body| Self::block_exits(body));
                match (then_exits, else_exits) {
                    (true, false) => *state = else_state,
                    (false, true) => *state = then_state,
                    (false, false) => {
                        then_state.join(&else_state);
                        *state = then_state;
                    }
                    (true, true) => {}
                }
                continue;
            }

            statement.visit_expressions(&mut |expression| {
                self.check_expression_calls(file, expression, state, program, violations);
            });

            if let Statement::Declaration(declaration) = statement {
                state.variable_types.insert(
                    declaration.name.clone(),
                    declaration.type_info.base_type.clone(),
                );
                if let Some(initializer) = &declaration.initializer {
                    let resolution = Self::resolve_callee(initializer, file, state, program);
                    state
                        .assigned_callables
                        .insert(declaration.name.clone(), resolution);
                }
            }
            statement.visit_expressions(&mut |expression| {
                Self::apply_assignments(expression, file, state, program);
            });

            statement.for_each_child_block(|body| {
                let mut child_state = state.clone();
                self.check_statements(file, body, &mut child_state, program, violations);
                state.join(&child_state);
            });
        }
    }
}

impl Rule for QemuCoroutineFn {
    fn name(&self) -> &'static str {
        "qemu:coroutine_fn"
    }

    fn description(&self) -> &'static str {
        "Detect QEMU coroutine calling-context violations"
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn opt_in_reason(&self) -> Option<&'static str> {
        Some("QEMU-specific coroutine calling-context check")
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let program = CoroutineProgram::build(ast_context, self, violations);
        for (path, file) in ast_context.iter_all_files() {
            for function in file.iter_function_definitions() {
                let annotations = AnnotationSet::from_modifiers(&function.macro_modifiers);
                let context = if annotations.resolution().is_some() {
                    annotations.body_context()
                } else {
                    match program.function_exact(path, &function.name, function.is_static) {
                        Some(Resolution::Restricted(CallPolicy::Coroutine)) => {
                            ExecutionContext::KnownCoroutine
                        }
                        Some(Resolution::Unrestricted) => ExecutionContext::Mixed,
                        _ => ExecutionContext::KnownNonCoroutine,
                    }
                };
                let mut state = FlowState::new(context, function);
                self.check_statements(
                    path,
                    &function.body_statements,
                    &mut state,
                    &program,
                    violations,
                );
            }
        }
    }
}
