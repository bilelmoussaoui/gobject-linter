use gobject_ast::{
    AssignmentOp, BinaryOp, Expression, IfStatement, SourceLocation, Statement, UnaryOp,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

#[derive(Clone, Copy)]
enum NullCheck {
    Null,
    Zero,
    NullOrZero,
}

impl NullCheck {
    fn matches(self, expr: &Expression) -> bool {
        match self {
            Self::Null => expr.is_null(),
            Self::Zero => expr.is_zero(),
            Self::NullOrZero => expr.is_null() || expr.is_zero(),
        }
    }
}

#[derive(Clone, Copy)]
enum ClearReplacement {
    Object,
    Pointer,
    HandleId,
    SignalHandler,
    WeakPointer,
    List { clear_func: &'static str },
}

#[derive(Clone, Copy)]
struct ClearMapping {
    source_func: &'static str,
    replacement: ClearReplacement,
    null_check: NullCheck,
    min_version: (u32, u32),
}

const CLEAR_MAPPINGS: &[ClearMapping] = &[
    ClearMapping {
        source_func: "g_source_remove",
        replacement: ClearReplacement::HandleId,
        null_check: NullCheck::Zero,
        min_version: (2, 56),
    },
    ClearMapping {
        source_func: "g_source_destroy",
        replacement: ClearReplacement::HandleId,
        null_check: NullCheck::Zero,
        min_version: (2, 56),
    },
    ClearMapping {
        source_func: "g_signal_handler_disconnect",
        replacement: ClearReplacement::SignalHandler,
        null_check: NullCheck::Zero,
        min_version: (2, 0),
    },
    ClearMapping {
        source_func: "g_object_remove_weak_pointer",
        replacement: ClearReplacement::WeakPointer,
        null_check: NullCheck::Null,
        min_version: (2, 56),
    },
    ClearMapping {
        source_func: "g_list_free",
        replacement: ClearReplacement::List {
            clear_func: "g_clear_list",
        },
        null_check: NullCheck::Null,
        min_version: (2, 64),
    },
    ClearMapping {
        source_func: "g_slist_free",
        replacement: ClearReplacement::List {
            clear_func: "g_clear_slist",
        },
        null_check: NullCheck::Null,
        min_version: (2, 64),
    },
    ClearMapping {
        source_func: "g_object_unref",
        replacement: ClearReplacement::Object,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_free",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_hash_table_unref",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_hash_table_destroy",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_array_unref",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_bytes_unref",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
    ClearMapping {
        source_func: "g_variant_unref",
        replacement: ClearReplacement::Pointer,
        null_check: NullCheck::NullOrZero,
        min_version: (2, 28),
    },
];

impl ClearMapping {
    fn is_enabled(&self, config: &Config) -> bool {
        if let Some((major, minor)) = config.min_glib_version
            && (major < self.min_version.0
                || (major == self.min_version.0 && minor < self.min_version.1))
        {
            return false;
        }
        true
    }
}

fn address_of(var_name: &str) -> String {
    if let Some(inner) = var_name.strip_prefix('*') {
        inner.to_string()
    } else {
        format!("&{var_name}")
    }
}

fn format_replacement(mapping: &ClearMapping, var_name: &str, obj: Option<&str>) -> String {
    let addr = address_of(var_name);
    match mapping.replacement {
        ClearReplacement::Object => format!("g_clear_object ({addr});"),
        ClearReplacement::Pointer => {
            format!("g_clear_pointer ({addr}, {});", mapping.source_func)
        }
        ClearReplacement::HandleId => {
            format!("g_clear_handle_id ({addr}, {});", mapping.source_func)
        }
        ClearReplacement::SignalHandler => {
            let obj = obj.unwrap_or("obj");
            format!("g_clear_signal_handler ({addr}, {obj});")
        }
        ClearReplacement::WeakPointer => {
            format!("g_clear_weak_pointer ({addr});")
        }
        ClearReplacement::List { clear_func } => {
            format!("{clear_func} ({addr}, NULL);")
        }
    }
}

pub struct UseClearFunctions;

impl Rule for UseClearFunctions {
    fn name(&self) -> &'static str {
        "use_clear_functions"
    }

    fn description(&self) -> &'static str {
        "Suggest g_clear_* functions instead of manual cleanup and NULL/zero assignment"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &gobject_ast::types::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_statements(config, file, &func.body_statements, violations);
    }
}

impl UseClearFunctions {
    fn check_statements(
        &self,
        config: &Config,
        file: &gobject_ast::FileModel,
        statements: &[Statement],
        violations: &mut Vec<Violation>,
    ) {
        let source = &file.source;

        // Check consecutive pairs
        let mut i = 0;
        while i < statements.len() {
            // Try signal_handler's if-guarded pattern
            if self.try_signal_handler_if_guarded(&statements[i], config, file, violations) {
                i += 1;
                continue;
            }

            // Try handle_id's if pattern
            if self.try_handle_id_if_pattern(&statements[i], config, file, violations) {
                i += 1;
                continue;
            }

            // Try generic if-statement pattern (clear_functions style)
            if let Statement::If(if_stmt) = &statements[i]
                && self.try_generic_if_pattern(if_stmt, config, file, violations)
            {
                i += 1;
                continue;
            }

            // Try consecutive pair patterns
            if i + 1 < statements.len() {
                if let Some(matched) = self.try_consecutive_pair(
                    &statements[i],
                    &statements[i + 1],
                    config,
                    file,
                    violations,
                ) && matched
                {
                    i += 2;
                    continue;
                }

                // Try signal_handler's disconnect + zero pattern
                if self.try_signal_disconnect_then_zero(
                    &statements[i],
                    &statements[i + 1],
                    config,
                    file,
                    violations,
                ) {
                    i += 2;
                    continue;
                }
            }

            // Try signal_handler's bare member disconnect
            if self.try_bare_disconnect_on_member(
                &statements[i],
                statements,
                config,
                file,
                violations,
            ) {
                i += 1;
                continue;
            }

            // Recurse into nested blocks
            if let Statement::If(if_stmt) = &statements[i] {
                self.check_statements(config, file, &if_stmt.then_body, violations);
                if let Some(else_body) = &if_stmt.else_body {
                    self.check_statements(config, file, else_body, violations);
                }
            } else {
                statements[i].for_each_child_block(|body| {
                    self.check_statements(config, file, body, violations);
                });
            }

            i += 1;
        }

        // handle_id: check for unnecessary braces around single g_clear_handle_id
        for stmt in statements {
            if let Statement::If(if_stmt) = stmt {
                self.check_unnecessary_braces(if_stmt, file, source, violations);
            }
        }
    }

    fn try_consecutive_pair(
        &self,
        stmt1: &Statement,
        stmt2: &Statement,
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> Option<bool> {
        let source = &file.source;
        let call = stmt1.extract_call()?;

        for mapping in CLEAR_MAPPINGS {
            if !mapping.is_enabled(config) {
                continue;
            }
            // Skip signal_handler — handled separately with arg reversal
            if matches!(mapping.replacement, ClearReplacement::SignalHandler) {
                continue;
            }

            if !call.is_function(mapping.source_func) {
                continue;
            }

            let var_name = match mapping.replacement {
                ClearReplacement::WeakPointer => {
                    let gobject_ast::Argument::Expression(arg_expr) = call.arguments.get(1)?;
                    self.extract_weak_pointer_var(arg_expr, source)?
                }
                _ => call.get_arg(0)?.to_source_string(source)?,
            };

            if !stmt2.is_assignment_to(var_name, |expr| mapping.null_check.matches(expr), source) {
                continue;
            }

            let replacement = format_replacement(mapping, var_name, None);
            let message = format!(
                "Use {} instead of {} and NULL/zero assignment",
                replacement.trim_end_matches(';'),
                mapping.source_func
            );

            let stmt1_end = stmt1.location().find_semicolon_end(source);
            let fixes = vec![
                Fix::new(stmt1.location().start_byte, stmt1_end, replacement),
                Fix::delete_line(stmt2.location(), source),
            ];

            violations.push(self.violation_with_fixes(
                &file.path,
                stmt1.location().line,
                stmt1.location().column,
                message,
                fixes,
            ));

            return Some(true);
        }
        Some(false)
    }

    fn try_generic_if_pattern(
        &self,
        if_stmt: &IfStatement,
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let source = &file.source;

        if self.has_logical_operators(&if_stmt.condition) {
            return false;
        }

        let Some(checked_var) = self.find_variable_in_condition(&if_stmt.condition, source) else {
            return false;
        };

        if if_stmt.then_body.len() != 2 {
            return false;
        }

        let Some(mapping) =
            self.find_unref_in_body(&if_stmt.then_body, checked_var, config, source)
        else {
            return false;
        };

        if !self.has_null_assignment(&if_stmt.then_body, checked_var, source, mapping.null_check) {
            return false;
        }

        let replacement = format_replacement(&mapping, checked_var, None);
        let message = format!(
            "Use {} instead of manual NULL check, unref, and assignment",
            replacement.trim_end_matches(';')
        );

        let fix = Fix::new(
            if_stmt.location.start_byte,
            if_stmt.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix(
            &file.path,
            if_stmt.location.line,
            if_stmt.location.column,
            message,
            fix,
        ));

        true
    }

    fn find_variable_in_condition<'a>(
        &self,
        expr: &Expression,
        source: &'a [u8],
    ) -> Option<&'a str> {
        if let Some(var) = expr.extract_variable_name(source) {
            return Some(var);
        }

        match expr {
            Expression::Binary(bin) => {
                if let Some(var) = self.find_variable_in_condition(&bin.left, source) {
                    return Some(var);
                }
                self.find_variable_in_condition(&bin.right, source)
            }
            Expression::Unary(unary) => self.find_variable_in_condition(&unary.operand, source),
            _ => expr.to_source_string(source),
        }
    }

    fn has_logical_operators(&self, expr: &Expression) -> bool {
        let mut found = false;
        expr.walk(&mut |e| {
            if let Expression::Binary(bin) = e
                && matches!(bin.operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr)
            {
                found = true;
            }
        });
        found
    }

    fn find_unref_in_body(
        &self,
        statements: &[Statement],
        var_name: &str,
        config: &Config,
        source: &[u8],
    ) -> Option<ClearMapping> {
        for stmt in statements {
            if let Some(call) = stmt.extract_call() {
                for mapping in CLEAR_MAPPINGS {
                    if !mapping.is_enabled(config) {
                        continue;
                    }
                    // Skip patterns that don't apply to generic if-check
                    if matches!(
                        mapping.replacement,
                        ClearReplacement::SignalHandler
                            | ClearReplacement::HandleId
                            | ClearReplacement::WeakPointer
                    ) {
                        continue;
                    }
                    if call.is_function(mapping.source_func) {
                        for arg in &call.arguments {
                            if let Some(arg_text) = arg.to_source_string(source)
                                && arg_text.contains(var_name)
                            {
                                return Some(*mapping);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn has_null_assignment(
        &self,
        statements: &[Statement],
        var_name: &str,
        source: &[u8],
        null_check: NullCheck,
    ) -> bool {
        statements
            .iter()
            .any(|stmt| stmt.is_assignment_to(var_name, |expr| null_check.matches(expr), source))
    }

    fn try_handle_id_if_pattern(
        &self,
        stmt: &Statement,
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let Statement::If(if_stmt) = stmt else {
            return false;
        };
        let source = &file.source;

        let conversions = self.check_handle_cleanup_then_zero(config, file, &if_stmt.then_body);

        if conversions.is_empty() {
            return false;
        }

        let stmt_count = if_stmt.then_body.len();
        let has_else = if_stmt.else_body.is_some();
        let cond_id = if_stmt.extract_nonzero_check_variable(source);

        for (var_name, mapping, first_loc, second_loc) in conversions {
            let replacement = format_replacement(&mapping, var_name, None);
            let message = format!(
                "Use {} instead of {} and zero assignment",
                replacement.trim_end_matches(';'),
                mapping.source_func
            );
            let can_remove_if = !has_else && cond_id == Some(var_name) && stmt_count == 2;

            let fix = if can_remove_if {
                Fix::new(
                    if_stmt.location.start_byte,
                    if_stmt.location.end_byte,
                    replacement,
                )
            } else if stmt_count == 2 {
                let first_start = if_stmt.then_body[0].location().start_byte;
                let (mut brace_start, brace_end) =
                    SourceLocation::find_braces_around(first_start, source);

                while brace_start > 0 && source[brace_start - 1] != b'\n' {
                    brace_start -= 1;
                }
                brace_start = brace_start.saturating_sub(1);

                let brace_location = SourceLocation::new(0, 0, brace_start + 1, brace_start + 1);
                let indent = brace_location.extract_line_indentation(source);
                let formatted_replacement = format!("\n{}{}", indent, replacement);

                Fix::new(brace_start, brace_end, formatted_replacement)
            } else {
                Fix::new(first_loc.start_byte, second_loc.end_byte, replacement)
            };

            violations.push(self.violation_with_fix(
                &file.path,
                first_loc.line,
                first_loc.column,
                message,
                fix,
            ));
        }
        true
    }

    fn check_handle_cleanup_then_zero<'a>(
        &self,
        config: &Config,
        file: &'a gobject_ast::FileModel,
        statements: &[Statement],
    ) -> Vec<(&'a str, ClearMapping, SourceLocation, SourceLocation)> {
        let mut results = Vec::new();

        Statement::for_each_pair(statements, |first, second| {
            if let Some((var_name, mapping)) =
                self.extract_handle_cleanup(first, config, &file.source)
                && second.is_assignment_to(var_name, Expression::is_zero, &file.source)
            {
                results.push((var_name, mapping, *first.location(), *second.location()));
            }
        });

        results
    }

    fn extract_handle_cleanup<'a>(
        &self,
        stmt: &Statement,
        config: &Config,
        source: &'a [u8],
    ) -> Option<(&'a str, ClearMapping)> {
        let call = stmt.extract_call()?;
        let func_name = call.function_name_str()?;

        let mapping = CLEAR_MAPPINGS.iter().find(|m| {
            matches!(m.replacement, ClearReplacement::HandleId)
                && m.source_func == func_name
                && m.is_enabled(config)
        })?;

        let arg_expr = call.get_arg(0)?;
        let var_name = arg_expr.location().as_str(source)?.trim();

        Some((var_name, *mapping))
    }

    fn check_unnecessary_braces(
        &self,
        if_stmt: &IfStatement,
        file: &gobject_ast::FileModel,
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        if if_stmt.then_body.len() == 1
            && if_stmt.then_has_braces
            && let Statement::Expression(expr_stmt) = &if_stmt.then_body[0]
            && let Expression::Call(call) = expr_stmt.as_ref()
            && call.is_function("g_clear_handle_id")
        {
            let call_text = call.location.as_str(source).unwrap_or("");
            let loc = if_stmt.then_body[0].location();
            let fix = Fix::new(loc.start_byte, loc.end_byte, format!("{};", call_text));

            violations.push(self.violation_with_fix(
                &file.path,
                if_stmt.location.line,
                if_stmt.location.column,
                "Remove unnecessary braces around single g_clear_handle_id call".to_string(),
                fix,
            ));
        }
    }

    fn try_signal_handler_if_guarded(
        &self,
        stmt: &Statement,
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let signal_mapping = match self.find_signal_mapping(config) {
            Some(m) => m,
            None => return false,
        };

        let Statement::If(if_stmt) = stmt else {
            return false;
        };

        if if_stmt.has_else() {
            return false;
        }

        let Some(guarded_id) = if_stmt.extract_nonzero_check_variable(&file.source) else {
            return false;
        };

        if if_stmt.then_body.len() != 2 {
            return false;
        }

        let Some((obj, handler_id)) =
            self.extract_disconnect_args(&if_stmt.then_body[0], &file.source)
        else {
            return false;
        };

        if handler_id != guarded_id {
            return false;
        }

        if !self.is_zero_assign(&if_stmt.then_body[1], handler_id, &file.source) {
            return false;
        }

        let replacement = format_replacement(&signal_mapping, handler_id, Some(obj));
        let message = format!(
            "Use {} instead of if-guarded g_signal_handler_disconnect",
            replacement.trim_end_matches(';')
        );
        let fix = Fix::new(
            if_stmt.location.start_byte,
            if_stmt.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix(
            &file.path,
            if_stmt.location.line,
            if_stmt.location.column,
            message,
            fix,
        ));
        true
    }

    fn try_signal_disconnect_then_zero(
        &self,
        s1: &Statement,
        s2: &Statement,
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let signal_mapping = match self.find_signal_mapping(config) {
            Some(m) => m,
            None => return false,
        };

        let Some((obj, handler_id)) = self.extract_disconnect_args(s1, &file.source) else {
            return false;
        };

        if !self.is_zero_assign(s2, handler_id, &file.source) {
            return false;
        }

        let replacement = format_replacement(&signal_mapping, handler_id, Some(obj));
        let message = format!(
            "Use {} instead of g_signal_handler_disconnect and zeroing the ID",
            replacement.trim_end_matches(';')
        );
        let s1_end = s1.location().find_semicolon_end(&file.source);

        let fixes = vec![
            Fix::new(s1.location().start_byte, s1_end, replacement),
            Fix::delete_line(s2.location(), &file.source),
        ];

        violations.push(self.violation_with_fixes(
            &file.path,
            s1.location().line,
            s1.location().column,
            message,
            fixes,
        ));
        true
    }

    fn try_bare_disconnect_on_member(
        &self,
        stmt: &Statement,
        all_stmts: &[Statement],
        config: &Config,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let signal_mapping = match self.find_signal_mapping(config) {
            Some(m) => m,
            None => return false,
        };

        let Some((obj, handler_id)) = self.extract_disconnect_args(stmt, &file.source) else {
            return false;
        };

        if !handler_id.contains("->") {
            return false;
        }

        let base = handler_id.split("->").next().unwrap_or("").trim();
        if base.is_empty() {
            return false;
        }

        if self.is_freed_in_stmts(all_stmts, base, &file.source)
            || self.is_freed_in_stmts(all_stmts, obj, &file.source)
        {
            return false;
        }

        let replacement = format_replacement(&signal_mapping, handler_id, Some(obj));
        let message = format!(
            "Use {} instead of g_signal_handler_disconnect (also zeroes the stored ID)",
            replacement.trim_end_matches(';')
        );
        let stmt_end = stmt.location().find_semicolon_end(&file.source);
        let fix = Fix::new(stmt.location().start_byte, stmt_end, replacement);

        violations.push(self.violation_with_fix(
            &file.path,
            stmt.location().line,
            stmt.location().column,
            message,
            fix,
        ));
        true
    }

    fn find_signal_mapping(&self, config: &Config) -> Option<ClearMapping> {
        CLEAR_MAPPINGS
            .iter()
            .find(|m| {
                matches!(m.replacement, ClearReplacement::SignalHandler) && m.is_enabled(config)
            })
            .copied()
    }

    fn extract_disconnect_args<'a>(
        &self,
        stmt: &Statement,
        source: &'a [u8],
    ) -> Option<(&'a str, &'a str)> {
        let call = stmt.extract_call()?;

        if !call.is_function("g_signal_handler_disconnect") {
            return None;
        }

        if call.arguments.len() != 2 {
            return None;
        }

        let obj = call.get_arg(0)?.extract_variable_name(source)?;
        let handler_id = call.get_arg(1)?.extract_variable_name(source)?;

        Some((obj, handler_id))
    }

    fn is_zero_assign(&self, stmt: &Statement, expected_id: &str, source: &[u8]) -> bool {
        let Statement::Expression(expr_stmt) = stmt else {
            return false;
        };

        let Expression::Assignment(assign) = expr_stmt.as_ref() else {
            return false;
        };

        assign.lhs_as_text(source) == expected_id
            && assign.operator == AssignmentOp::Assign
            && assign.rhs.is_zero()
    }

    fn is_freed_in_stmts(&self, stmts: &[Statement], target: &str, source: &[u8]) -> bool {
        for stmt in stmts {
            let Statement::Expression(expr_stmt) = stmt else {
                continue;
            };

            let Expression::Call(call) = expr_stmt.as_ref() else {
                continue;
            };

            if !call.function_contains("free", source)
                && !call.function_contains("unref", source)
                && !call.function_contains("destroy", source)
                && !call.function_contains("clear", source)
            {
                continue;
            }

            for arg in &call.arguments {
                if self.arg_references(arg, target, source) {
                    return true;
                }
            }
        }
        false
    }

    fn arg_references(&self, arg: &gobject_ast::Argument, target: &str, source: &[u8]) -> bool {
        let gobject_ast::Argument::Expression(expr) = arg;

        let mut found = false;
        expr.walk(&mut |e| match e {
            Expression::Identifier(id) if id.name == target => {
                found = true;
            }
            Expression::FieldAccess(f) if f.location.as_str(source) == Some(target) => {
                found = true;
            }
            _ => {}
        });
        found
    }

    fn extract_weak_pointer_var<'a>(&self, expr: &Expression, source: &'a [u8]) -> Option<&'a str> {
        // Handle cast expressions: (gpointer*)&var
        let inner_expr = match expr {
            Expression::Cast(cast) => &*cast.operand,
            other => other,
        };

        if let Expression::Unary(unary) = inner_expr
            && unary.operator == UnaryOp::AddressOf
        {
            return unary.operand.extract_variable_name(source);
        }

        None
    }
}
