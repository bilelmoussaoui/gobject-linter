use std::fmt;

use gobject_ast::model::{
    AutoCleanupMacro, Expression, FileModel, FunctionDefItem, Statement, VariableDecl,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct GAutoInit;

impl Rule for GAutoInit {
    fn name(&self) -> &'static str {
        "g_auto_init"
    }

    fn description(&self) -> &'static str {
        "Ensure g_auto*/g_autofree/g_autofd variables are initialized"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/g_auto_init.md"))
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
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
        self.check_statements(&func.body_statements, file, violations);
    }
}

impl GAutoInit {
    fn check_statements(
        &self,
        stmts: &[Statement],
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Statement::Declaration(decl) = stmt {
                self.check_declaration(decl, &stmts[i + 1..], file, violations);
            }
            stmt.for_each_child_block(|block| {
                self.check_statements(block, file, violations);
            });
        }
    }

    fn check_declaration(
        &self,
        decl: &VariableDecl,
        following: &[Statement],
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let Some(auto) = &decl.type_info.auto_cleanup else {
            return;
        };

        let expected = match auto {
            AutoCleanupMacro::Autofd => ExpectedInit::NegativeOne,
            AutoCleanupMacro::Auto(type_name) => match Self::auto_init_macro(type_name) {
                Some(m) => ExpectedInit::Macro(m),
                None => return,
            },
            _ => ExpectedInit::Null,
        };

        let is_properly_initialized = match &decl.initializer {
            None => false,
            Some(expr) => expected.matches(expr),
        };

        if is_properly_initialized {
            return;
        }

        // Any other initializer (e.g. g_strdup(), open()) means the variable
        // is set — not our concern.
        if decl.initializer.is_some() {
            return;
        }

        if self.first_use_is_assignment(decl, following) {
            return;
        }

        let insert_pos = decl.location.end_byte - 1;
        let fix = Fix::new(insert_pos, insert_pos, format!(" = {expected}"));

        violations.push(self.violation_with_fix_at(
            &file.path,
            &decl.location,
            format!(
                "{} variable '{}' must be initialized to {expected}",
                auto.name(),
                decl.name,
            ),
            fix,
        ));
    }

    fn auto_init_macro(type_name: &str) -> Option<&'static str> {
        match type_name {
            "GQueue" => Some("G_QUEUE_INIT"),
            "GValue" => Some("G_VALUE_INIT"),
            "GOnce" => Some("G_ONCE_INIT"),
            "GPathBuf" => Some("G_PATH_BUF_INIT"),
            "GUnixPipe" => Some("G_UNIX_PIPE_INIT"),
            _ => None,
        }
    }

    fn first_use_is_assignment(&self, decl: &VariableDecl, stmts: &[Statement]) -> bool {
        for stmt in stmts {
            let mut references_var = false;
            stmt.visit_expressions(&mut |expr| {
                if expr.contains_identifier(&decl.name) {
                    references_var = true;
                }
            });
            if !references_var {
                continue;
            }
            return stmt.is_assignment_to(&decl.as_expression(), |_| true);
        }
        false
    }
}

enum ExpectedInit {
    Null,
    NegativeOne,
    Macro(&'static str),
}

impl ExpectedInit {
    fn matches(&self, expr: &Expression) -> bool {
        match self {
            Self::Null => expr.is_null() || expr.is_zero(),
            Self::NegativeOne => expr.is_negative_one(),
            Self::Macro(name) => {
                matches!(expr, Expression::Identifier(id) if id.name == *name)
            }
        }
    }
}

impl fmt::Display for ExpectedInit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::NegativeOne => f.write_str("-1"),
            Self::Macro(name) => f.write_str(name),
        }
    }
}
