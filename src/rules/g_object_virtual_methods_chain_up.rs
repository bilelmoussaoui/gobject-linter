use gobject_ast::model::{Expression, Parameter, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Fix, Rule, Violation},
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

    fn fixable(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
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

                    if has_chainup_call(&func.body_statements, field) {
                        continue;
                    }

                    let param_name = func
                        .parameters
                        .first()
                        .and_then(|p| match p {
                            Parameter::Regular { name, .. } => name.as_deref(),
                            _ => None,
                        })
                        .unwrap_or("object");

                    let parent_class = format!("{}_parent_class", gt.function_prefix);
                    let cast = config.style.format_call("G_OBJECT_CLASS", &[&parent_class]);
                    let args = config.style.format_call("", &[param_name]);
                    let chainup_call = format!("{cast}->{field}{args};");

                    let fix = if let Some(body_loc) = &func.body_location {
                        let indent = func
                            .body_statements
                            .iter()
                            .find(|s| {
                                matches!(s, Statement::Declaration(_) | Statement::Expression(_))
                            })
                            .map_or_else(
                                || "  ".to_string(),
                                |s| s.location().extract_line_indentation(),
                            );

                        let (pos, before, after) = if *field == "constructed" {
                            let last_decl = func
                                .body_statements
                                .iter()
                                .filter_map(|s| match s {
                                    Statement::Declaration(d) => Some(&d.location),
                                    _ => None,
                                })
                                .next_back();

                            if let Some(loc) = last_decl {
                                let after = if loc.count_trailing_newlines() >= 2 {
                                    ""
                                } else {
                                    "\n"
                                };
                                (loc.end_byte, "\n\n", after)
                            } else {
                                (body_loc.start_byte + 1, "\n", "\n")
                            }
                        } else {
                            let trailing = func
                                .body_statements
                                .last()
                                .map_or(1, |s| s.location().count_trailing_newlines());
                            let before = if trailing >= 2 { "" } else { "\n" };
                            (body_loc.end_byte - 1, before, "\n")
                        };

                        Some(Fix::new(
                            pos,
                            pos,
                            format!("{before}{indent}{chainup_call}{after}"),
                        ))
                    } else {
                        None
                    };

                    let msg = format!(
                        "{func_name} must chain up to parent class (e.g., {chainup_call})",
                    );
                    let violation = if let Some(fix) = fix {
                        self.violation_with_fix(
                            &file.path,
                            func.location.line,
                            func.location.column,
                            msg,
                            fix,
                        )
                    } else {
                        self.violation(&file.path, func.location.line, func.location.column, msg)
                    };
                    violations.push(violation);
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
                    if let Expression::Call(call) = e {
                        let func = match &*call.function {
                            Expression::Unary(u) => &u.operand,
                            other => other,
                        };
                        if let Expression::FieldAccess(fa) = func
                            && fa.field == method_type
                        {
                            found = true;
                        }
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
