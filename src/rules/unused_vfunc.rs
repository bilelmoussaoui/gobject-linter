use std::collections::{HashMap, HashSet};

use gobject_ast::model::{Expression, TypeDefItem, UnaryOp};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct UnusedVfunc;

impl Rule for UnusedVfunc {
    fn name(&self) -> &'static str {
        "unused_vfunc"
    }

    fn description(&self) -> &'static str {
        "Detect virtual methods assigned in class_init but never called through the vtable"
    }

    fn category(&self) -> Category {
        Category::Suspicious
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let mut vfunc_fields: HashMap<&str, Vec<(&std::path::Path, usize, usize, String)>> =
            HashMap::new();
        let mut signal_fields: HashSet<&str> = HashSet::new();

        for (_path, file) in ast_context.iter_all_files() {
            for gt in file.iter_all_gobject_types() {
                let Some(class_struct_name) = gt.class_struct_name() else {
                    continue;
                };

                for sig in &gt.signals {
                    if let Some(offset) = &sig.class_offset {
                        signal_fields.insert(offset.field.as_str());
                    }
                }

                let class_struct_vfuncs = match file.find_class_struct_for(gt) {
                    Some(TypeDefItem::Struct { vfuncs, .. }) => vfuncs,
                    _ => continue,
                };

                let assigned_vfuncs = file.resolve_class_init_vfuncs(gt);
                let class_init_name = gt.class_init_function_name();

                for (class_type, field) in assigned_vfuncs.keys() {
                    if is_gobject_builtin(field) || class_type != &class_struct_name {
                        continue;
                    }
                    if let Some(vf) = class_struct_vfuncs.iter().find(|v| v.name == *field) {
                        vfunc_fields.entry(field).or_default().push((
                            &file.path,
                            vf.location.line,
                            vf.location.column,
                            class_init_name.clone(),
                        ));
                    }
                }
            }
        }

        for field in &signal_fields {
            vfunc_fields.remove(field);
        }

        if vfunc_fields.is_empty() {
            return;
        }

        let mut called_fields: HashSet<&str> = HashSet::new();

        for (_path, file) in ast_context.iter_all_files() {
            for func in file.iter_function_definitions() {
                for stmt in &func.body_statements {
                    stmt.walk_expressions(&mut |expr| {
                        expr.walk(&mut |e| {
                            if let Expression::Call(call) = e {
                                let func = match &*call.function {
                                    Expression::FieldAccess(fa) => Some(fa),
                                    Expression::Unary(u) if u.operator == UnaryOp::Dereference => {
                                        if let Expression::FieldAccess(fa) = &*u.operand {
                                            Some(fa)
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };
                                if let Some(fa) = func {
                                    let field = fa.field.as_str();
                                    if vfunc_fields.contains_key(field) {
                                        called_fields.insert(field);
                                    }
                                }
                            }
                        });
                    });
                }
            }
        }

        for (field, infos) in &vfunc_fields {
            if called_fields.contains(field) {
                continue;
            }
            for (file_path, line, column, class_init_name) in infos {
                violations.push(self.violation(
                    file_path,
                    *line,
                    *column,
                    format!(
                        "Virtual method '{}' is assigned in {}() but never called through the class vtable",
                        field, class_init_name
                    ),
                ));
            }
        }
    }
}

fn is_gobject_builtin(field: &str) -> bool {
    matches!(
        field,
        "dispose"
            | "finalize"
            | "constructed"
            | "get_property"
            | "set_property"
            | "notify"
            | "dispatch_properties_changed"
            | "constructor"
    )
}
