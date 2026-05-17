use gobject_ast::model::{Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

const CHAINABLE_VFUNCS: &[&str] = &["dispose", "finalize", "constructed"];

pub struct GObjectVirtualMethodsChainUp;

impl Rule for GObjectVirtualMethodsChainUp {
    fn name(&self) -> &'static str {
        "g_object_virtual_methods_chain_up"
    }

    fn description(&self) -> &'static str {
        "Ensure dispose/finalize/constructed methods chain up to parent class"
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (_path, file) in ast_context.iter_all_files() {
            for gt in file
                .iter_all_gobject_types()
                .filter(|gt| gt.kind.is_define())
            {
                let vfuncs = file.resolve_class_init_vfuncs(gt);

                for ((class_type, field), func_name) in &vfuncs {
                    if class_type != "GObjectClass" || !CHAINABLE_VFUNCS.contains(field) {
                        continue;
                    }

                    let Some(func) = file
                        .iter_function_definitions()
                        .find(|f| f.name == *func_name)
                    else {
                        continue;
                    };

                    if !has_chainup_call(&func.body_statements, field) {
                        violations.push(self.violation(
                            &file.path,
                            func.location.line,
                            func.location.column,
                            format!(
                                "{} must chain up to parent class (e.g., G_OBJECT_CLASS (parent_class)->{} (object))",
                                func_name, field
                            ),
                        ));
                    }
                }
            }
        }
    }
}

fn has_chainup_call(statements: &[Statement], method_type: &str) -> bool {
    for stmt in statements {
        let mut found = false;
        stmt.walk(&mut |s| {
            s.visit_expressions(&mut |expr| {
                expr.walk(&mut |e| {
                    if let Expression::Call(call) = e
                        && let Expression::FieldAccess(fa) = &*call.function
                        && fa.field == method_type
                    {
                        found = true;
                    }
                });
            });
        });
        if found {
            return true;
        }
    }
    false
}
