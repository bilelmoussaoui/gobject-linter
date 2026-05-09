use gobject_ast::model::{
    Argument, BinaryOp, Expression, FileModel, FunctionDefItem, SourceLocation,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGStrHasPrefixSuffix;

impl Rule for UseGStrHasPrefixSuffix {
    fn name(&self) -> &'static str {
        "use_g_str_has_prefix_suffix"
    }

    fn description(&self) -> &'static str {
        "Use g_str_has_prefix/g_str_has_suffix() instead of manual strncmp/strcmp comparisons"
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
        for stmt in &func.body_statements {
            stmt.walk_expressions(&mut |expr| {
                expr.walk(&mut |e| {
                    self.check_expression(e, file, violations);
                });
            });
        }
    }
}

impl UseGStrHasPrefixSuffix {
    fn check_expression(
        &self,
        expr: &Expression,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let Expression::Binary(bin) = expr else {
            return;
        };
        if !matches!(bin.operator, BinaryOp::Equal | BinaryOp::NotEqual) {
            return;
        }
        self.check_for_prefix_pattern(
            &bin.left,
            &bin.right,
            &bin.operator,
            file,
            &bin.location,
            violations,
        );
        self.check_for_prefix_pattern(
            &bin.right,
            &bin.left,
            &bin.operator,
            file,
            &bin.location,
            violations,
        );
        self.check_for_suffix_pattern(
            &bin.left,
            &bin.right,
            &bin.operator,
            file,
            &bin.location,
            violations,
        );
        self.check_for_suffix_pattern(
            &bin.right,
            &bin.left,
            &bin.operator,
            file,
            &bin.location,
            violations,
        );
    }

    /// Check for strncmp(str, "prefix", strlen("prefix")) == 0 pattern
    fn check_for_prefix_pattern(
        &self,
        strncmp_side: &Expression,
        value_side: &Expression,
        operator: &BinaryOp,
        file: &FileModel,
        location: &SourceLocation,
        violations: &mut Vec<Violation>,
    ) {
        // strncmp_side must be a call to strncmp
        let Expression::Call(call) = strncmp_side else {
            return;
        };

        if !call.is_function("strncmp") {
            return;
        }

        // value_side must be 0
        if !value_side.is_zero() {
            return;
        }

        // Must have 3 arguments
        if call.arguments.len() != 3 {
            return;
        }

        // Second argument must be a string literal
        let Some(prefix_text) = call.arguments[1].extract_string_value() else {
            return;
        };

        // Third argument must be strlen(prefix_text)
        if !self.is_strlen_of(&call.arguments[2], &prefix_text) {
            return;
        }

        let str_arg_text = call
            .get_arg(0)
            .and_then(|e| e.to_source_string(&file.source))
            .unwrap_or_default();

        let replacement = if *operator == BinaryOp::Equal {
            format!("g_str_has_prefix ({str_arg_text}, \"{prefix_text}\")")
        } else {
            format!("!g_str_has_prefix ({str_arg_text}, \"{prefix_text}\")")
        };
        let message = format!(
            "Use {replacement} instead of strncmp() {} 0",
            operator.as_str()
        );
        let fix = Fix::new(location.start_byte, location.end_byte, replacement);

        violations.push(self.violation_with_fix(
            &file.path,
            location.line,
            location.column,
            message,
            fix,
        ));
    }

    /// Check for strcmp(str + strlen(str) - strlen("suffix"), "suffix") == 0
    /// pattern
    fn check_for_suffix_pattern(
        &self,
        strcmp_side: &Expression,
        value_side: &Expression,
        operator: &BinaryOp,
        file: &FileModel,
        location: &SourceLocation,
        violations: &mut Vec<Violation>,
    ) {
        // strcmp_side must be a call to strcmp
        let Expression::Call(call) = strcmp_side else {
            return;
        };

        if !call.is_function("strcmp") {
            return;
        }

        // value_side must be 0
        if !value_side.is_zero() {
            return;
        }

        // Must have 2 arguments
        if call.arguments.len() != 2 {
            return;
        }

        // Second argument must be a string literal
        let Some(suffix_text) = call.arguments[1].extract_string_value() else {
            return;
        };

        // First argument must be: str + strlen(str) - strlen("suffix")
        let Some(str_expr) =
            self.extract_suffix_base(&call.arguments[0], &suffix_text, &file.source)
        else {
            return;
        };

        let replacement = if *operator == BinaryOp::Equal {
            format!("g_str_has_suffix ({str_expr}, \"{suffix_text}\")")
        } else {
            format!("!g_str_has_suffix ({str_expr}, \"{suffix_text}\")")
        };
        let message = format!(
            "Use {replacement} instead of strcmp() {} 0",
            operator.as_str()
        );
        let fix = Fix::new(location.start_byte, location.end_byte, replacement);

        violations.push(self.violation_with_fix(
            &file.path,
            location.line,
            location.column,
            message,
            fix,
        ));
    }

    /// Validates that arg is `<str_expr> + strlen(<str_expr>) -
    /// strlen("suffix")` and returns `str_expr` if so.
    fn extract_suffix_base<'a>(
        &self,
        arg: &Argument,
        suffix_text: &str,
        source: &'a [u8],
    ) -> Option<&'a str> {
        let Argument::Expression(expr) = arg;

        // Top level: X - strlen("suffix")
        let Expression::Binary(top_bin) = &**expr else {
            return None;
        };

        if top_bin.operator != BinaryOp::Subtract {
            return None;
        }

        // Right side must be strlen("suffix") - note suffix_text comes from
        // extract_string_value so no quotes We need to wrap it in quotes for
        // comparison since expr_to_text adds quotes
        if !self.is_strlen_of_arg_by_value(&top_bin.right, suffix_text) {
            return None;
        }

        // Left side: <str_expr> + strlen(<str_expr>)
        let Expression::Binary(inner_bin) = &*top_bin.left else {
            return None;
        };

        if inner_bin.operator != BinaryOp::Add {
            return None;
        }

        let str_expr = inner_bin.left.to_source_string(source)?;

        // Right side must be strlen(str_expr)
        if !self.is_strlen_of_arg(&inner_bin.right, str_expr, source) {
            return None;
        }

        Some(str_expr)
    }

    /// Returns true if arg is strlen(expected_text)
    fn is_strlen_of(&self, arg: &Argument, expected_text: &str) -> bool {
        let Argument::Expression(expr) = arg;

        let Expression::Call(call) = &**expr else {
            return false;
        };

        if !call.is_function("strlen") {
            return false;
        }

        if call.arguments.len() != 1 {
            return false;
        }

        // Extract string value and compare
        if let Some(str_val) = call.arguments[0].extract_string_value() {
            return str_val == expected_text;
        }

        false
    }

    /// Returns true if expr is strlen(expected_text_with_quotes)
    fn is_strlen_of_arg(
        &self,
        expr: &Expression,
        expected_text_with_quotes: &str,
        source: &[u8],
    ) -> bool {
        let Expression::Call(call) = expr else {
            return false;
        };

        if !call.is_function("strlen") {
            return false;
        }

        if call.arguments.len() != 1 {
            return false;
        }

        call.get_arg(0).is_some_and(|e| {
            e.to_source_string(source)
                .is_some_and(|s| s == expected_text_with_quotes)
        })
    }

    /// Returns true if expr is strlen("expected_string_value")
    fn is_strlen_of_arg_by_value(&self, expr: &Expression, expected_string_value: &str) -> bool {
        let Expression::Call(call) = expr else {
            return false;
        };

        if !call.is_function("strlen") {
            return false;
        }

        if call.arguments.len() != 1 {
            return false;
        }

        // Extract string value and compare
        if let Some(str_val) = call.arguments[0].extract_string_value() {
            return str_val == expected_string_value;
        }

        false
    }
}
