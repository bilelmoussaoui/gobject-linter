use gobject_ast::model::{
    AssignmentOp, Expression, FileModel, FunctionDefItem, SourceLocation, Statement,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGStealPointer;

impl Rule for UseGStealPointer {
    fn name(&self) -> &'static str {
        "use_g_steal_pointer"
    }

    fn description(&self) -> &'static str {
        "Use g_steal_pointer() instead of manually copying a pointer and setting it to NULL"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_function(func, file, violations);
    }
}

impl UseGStealPointer {
    fn check_function(
        &self,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_statements(&func.body_statements, file, violations);
    }

    fn check_statements(
        &self,
        statements: &[Statement],
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let mut i = 0;
        while i < statements.len() {
            if self.try_if_else_steal(&statements[i], file, violations) {
                i += 1;
                continue;
            }
            if self.try_if_no_else_steal(&statements[i], file, violations) {
                i += 1;
                continue;
            }
            if i + 2 < statements.len()
                && self.try_declare_null_return(
                    &statements[i],
                    &statements[i + 1],
                    &statements[i + 2],
                    file,
                    violations,
                )
            {
                i += 3;
                continue;
            }
            if i + 1 < statements.len()
                && self.try_assign_null(&statements[i], &statements[i + 1], file, violations)
            {
                i += 2;
                continue;
            }
            statements[i].for_each_child_block(|body| {
                self.check_statements(body, file, violations);
            });
            i += 1;
        }
    }

    /// Matches: `T *tmp = ptr_expr; ptr_expr = NULL; return tmp;`
    fn try_declare_null_return(
        &self,
        s1: &Statement,
        s2: &Statement,
        s3: &Statement,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        // s1: T *tmp = ptr_expr
        let Statement::Declaration(decl) = s1 else {
            return false;
        };

        let Some(init_expr) = &decl.initializer else {
            return false;
        };

        // Skip NULL initializers
        if init_expr.is_null() {
            return false;
        }

        // Get the variable name from the initializer
        let Some(ptr_expr) = init_expr.extract_variable_name(&file.source) else {
            return false;
        };

        // Skip dereferences
        if ptr_expr.starts_with('*') {
            return false;
        }

        let tmp_name = &decl.name;

        // s2: ptr_expr = NULL
        if !s2.is_null_assignment_to(ptr_expr, &file.source) {
            return false;
        }

        // s3: return tmp
        let Statement::Return(ret) = s3 else {
            return false;
        };

        if let Some(Expression::Identifier(id)) = &ret.value {
            if id.name != *tmp_name {
                return false;
            }
        } else {
            return false;
        }

        let replacement = format!("return g_steal_pointer (&{ptr_expr});");
        let message =
            format!("Use {replacement} instead of copying {ptr_expr} and setting it to NULL");

        // Use three separate fixes to preserve comments between statements
        let fixes = vec![
            // Delete the first two lines
            Fix::delete_line(s1.location(), &file.source),
            Fix::delete_line(s2.location(), &file.source),
            // Replace the third statement (return)
            Fix::new(
                s3.location().start_byte,
                s3.location().end_byte,
                replacement,
            ),
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

    /// Matches: `other_expr = ptr_expr; ptr_expr = NULL;`
    fn try_assign_null(
        &self,
        s1: &Statement,
        s2: &Statement,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let Some((other_expr, ptr_expr)) = self.extract_assignment(s1, &file.source) else {
            return false;
        };

        // Skip dereference expressions — g_steal_pointer (&*expr) is confusing
        if ptr_expr.starts_with('*') {
            return false;
        }

        if !s2.is_null_assignment_to(ptr_expr, &file.source) {
            return false;
        }

        let replacement = format!("{other_expr} = g_steal_pointer (&{ptr_expr});");
        let message =
            format!("Use g_steal_pointer (&{ptr_expr}) instead of copying and setting to NULL");

        // Use two separate fixes to preserve comments between statements
        let s2_end = s2.location().find_semicolon_end(&file.source);
        let fixes = vec![
            // Delete the entire first line
            Fix::delete_line(s1.location(), &file.source),
            // Replace the second statement
            Fix::new(s2.location().start_byte, s2_end, replacement),
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

    /// Matches: if (expr) { dest = expr; expr = NULL; } else { dest = NULL; }
    fn try_if_else_steal(
        &self,
        stmt: &Statement,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let Statement::If(if_stmt) = stmt else {
            return false;
        };

        // Must have else block
        let Some(else_body) = &if_stmt.else_body else {
            return false;
        };

        // Extract tested expression from condition
        let Some(expr_text) = if_stmt.extract_null_check_variable(&file.source) else {
            return false;
        };

        // Skip dereference expressions
        if expr_text.starts_with('*') {
            return false;
        }

        // Then-block must have exactly 2 statements
        if if_stmt.then_body.len() != 2 {
            return false;
        }

        // then_body[0]: dest = expr
        let Some((dest_expr, rhs)) = self.extract_assignment(&if_stmt.then_body[0], &file.source)
        else {
            return false;
        };
        if rhs != expr_text {
            return false;
        }

        // then_body[1]: expr = NULL
        if !if_stmt.then_body[1].is_null_assignment_to(expr_text, &file.source) {
            return false;
        }

        // Else-block must have exactly 1 statement: dest = NULL
        if else_body.len() != 1 {
            return false;
        }
        if !else_body[0].is_null_assignment_to(dest_expr, &file.source) {
            return false;
        }

        let replacement = format!("{dest_expr} = g_steal_pointer (&{expr_text});");
        let message =
            format!("Use g_steal_pointer (&{expr_text}) instead of if/else copy-and-NULL pattern");
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

    /// Matches if-without-else with steal pattern in body
    /// if (c) { dest = ptr; ptr = NULL; } or if (c) { T *tmp = ptr; ptr = NULL;
    /// return tmp; }
    fn try_if_no_else_steal(
        &self,
        stmt: &Statement,
        file: &FileModel,

        violations: &mut Vec<Violation>,
    ) -> bool {
        let Statement::If(if_stmt) = stmt else {
            return false;
        };

        // Must have no else
        if if_stmt.else_body.is_some() {
            return false;
        }

        // Try to extract condition expression
        let condition_expr = if_stmt.extract_null_check_variable(&file.source);

        // Pattern 1: 2 statements - dest = ptr; ptr = NULL;
        if if_stmt.then_body.len() == 2 {
            let Some((dest_expr, ptr_expr)) =
                self.extract_assignment(&if_stmt.then_body[0], &file.source)
            else {
                return false;
            };

            // Skip dereference expressions
            if ptr_expr.starts_with('*') {
                return false;
            }

            if !if_stmt.then_body[1].is_null_assignment_to(ptr_expr, &file.source) {
                return false;
            }

            let replacement = format!("{dest_expr} = g_steal_pointer (&{ptr_expr});");
            let message =
                format!("Use g_steal_pointer (&{ptr_expr}) instead of copying and setting to NULL");

            // If condition tests the same variable being stolen, remove entire if
            // Otherwise just replace the body
            let fix = if condition_expr == Some(ptr_expr) {
                Fix::new(
                    if_stmt.location.start_byte,
                    if_stmt.location.end_byte,
                    replacement,
                )
            } else if if_stmt.then_has_braces {
                let body_start = if_stmt.then_body[0].location().start_byte;
                let (open_brace, close_brace) =
                    SourceLocation::find_braces_around(body_start, &file.source);
                Fix::new(open_brace, close_brace, replacement)
            } else {
                let body_start = if_stmt.then_body[0].location().start_byte;
                let body_end = if_stmt.then_body[1].location().end_byte;
                Fix::new(body_start, body_end, replacement)
            };

            violations.push(self.violation_with_fix(
                &file.path,
                if_stmt.then_body[0].location().line,
                if_stmt.then_body[0].location().column,
                message,
                fix,
            ));
            return true;
        }

        // Pattern 2: 3 statements - T *tmp = ptr; ptr = NULL; return tmp;
        if if_stmt.then_body.len() == 3 {
            let Statement::Declaration(decl) = &if_stmt.then_body[0] else {
                return false;
            };

            let Some(init_expr) = &decl.initializer else {
                return false;
            };

            // Skip NULL initializers
            if init_expr.is_null() {
                return false;
            }

            let Some(ptr_expr) = init_expr.extract_variable_name(&file.source) else {
                return false;
            };

            // Skip dereference expressions
            if ptr_expr.starts_with('*') {
                return false;
            }

            let tmp_name = &decl.name;

            if !if_stmt.then_body[1].is_null_assignment_to(ptr_expr, &file.source) {
                return false;
            }

            // Third statement must be return tmp
            let Statement::Return(ret) = &if_stmt.then_body[2] else {
                return false;
            };

            if let Some(Expression::Identifier(id)) = &ret.value {
                if id.name != *tmp_name {
                    return false;
                }
            } else {
                return false;
            }

            let replacement = format!("return g_steal_pointer (&{ptr_expr});");
            let message =
                format!("Use {replacement} instead of copying {ptr_expr} and setting it to NULL");

            // If condition tests the same variable being stolen, remove entire if
            let fix = if condition_expr == Some(ptr_expr) {
                Fix::new(
                    if_stmt.location.start_byte,
                    if_stmt.location.end_byte,
                    replacement,
                )
            } else if if_stmt.then_has_braces {
                let body_start = if_stmt.then_body[0].location().start_byte;
                let (open_brace, close_brace) =
                    SourceLocation::find_braces_around(body_start, &file.source);
                Fix::new(open_brace, close_brace, replacement)
            } else {
                let body_start = if_stmt.then_body[0].location().start_byte;
                let body_end = if_stmt.then_body[2].location().end_byte;
                Fix::new(body_start, body_end, replacement)
            };

            violations.push(self.violation_with_fix(
                &file.path,
                if_stmt.then_body[0].location().line,
                if_stmt.then_body[0].location().column,
                message,
                fix,
            ));
            return true;
        }

        false
    }

    /// Extract (lhs, rhs) from assignment statement
    fn extract_assignment<'a>(
        &self,
        stmt: &'a Statement,
        source: &'a [u8],
    ) -> Option<(&'a str, &'a str)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Assignment(assign) = expr_stmt.as_ref() else {
            return None;
        };

        if assign.operator != AssignmentOp::Assign {
            return None;
        }

        // Get rhs as string - handle various expression types
        let rhs = match &*assign.rhs {
            Expression::Identifier(id) => id.name.as_str(),
            Expression::FieldAccess(f) => f.location.as_str(source).unwrap_or(""),
            Expression::Null(_) | Expression::Call(_) => {
                // For NULL or function calls like g_strdup(), we don't want to suggest
                // g_steal_pointer
                return None;
            }
            _ => {
                return None;
            }
        };

        let lhs = assign.lhs_as_text(source);
        if lhs.is_empty() {
            return None;
        }
        Some((lhs, rhs))
    }
}
