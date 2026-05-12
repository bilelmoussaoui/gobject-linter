use gobject_ast::model::{FileModel, FunctionDefItem, Statement, VariableDecl};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct GErrorInit;

impl Rule for GErrorInit {
    fn name(&self) -> &'static str {
        "g_error_init"
    }

    fn description(&self) -> &'static str {
        "Ensure GError* variables are initialized to NULL"
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
        self.check_block(&func.body_statements, file, violations);
    }
}

impl GErrorInit {
    fn check_block(&self, stmts: &[Statement], file: &FileModel, violations: &mut Vec<Violation>) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Statement::Declaration(decl) = stmt {
                self.check_declaration(decl, &stmts[i + 1..], file, violations);
            }
            stmt.for_each_child_block(|block| {
                self.check_block(block, file, violations);
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
        if !decl.type_info.is_base_type("GError") || !decl.type_info.is_pointer() {
            return;
        }

        let is_initialized_to_null = match &decl.initializer {
            None => false,
            Some(expr) if expr.is_null() || expr.is_zero() => true,
            Some(_) => return,
        };

        if is_initialized_to_null {
            return;
        }

        if self.first_use_is_assignment(&decl.name, following, &file.source) {
            return;
        }

        let insert_pos = decl.location.end_byte - 1;

        let fix = Fix::new(insert_pos, insert_pos, " = NULL".to_string());

        violations.push(self.violation_with_fix_at(
            &file.path,
            &decl.location,
            format!("GError *{} must be initialized to NULL", decl.name),
            fix,
        ));
    }

    fn first_use_is_assignment(&self, var: &str, stmts: &[Statement], source: &[u8]) -> bool {
        for stmt in stmts {
            let mut references_var = false;
            stmt.visit_expressions(&mut |expr| {
                if expr.contains_identifier(var) {
                    references_var = true;
                }
            });
            if !references_var {
                continue;
            }
            return stmt.is_assignment_to(var, |_| true, source);
        }
        true
    }
}
