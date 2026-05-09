use std::{collections::HashMap, path::Path};

use gobject_ast::model::{Parameter, TypeInfo};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct InconsistentFunctionSignature;

struct DeclInfo<'a> {
    return_type: &'a TypeInfo,
    parameters: &'a [Parameter],
}

struct DefInfo<'a> {
    line: usize,
    column: usize,
    path: &'a Path,
    return_type: &'a TypeInfo,
    parameters: &'a [Parameter],
}

impl Rule for InconsistentFunctionSignature {
    fn name(&self) -> &'static str {
        "inconsistent_function_signature"
    }

    fn description(&self) -> &'static str {
        "Detect functions whose return type or parameter types in the declaration do not match the definition"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Suspicious
    }

    fn fixable(&self) -> bool {
        false
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let mut global_decls: HashMap<&str, DeclInfo> = HashMap::new();
        let mut all_defs: HashMap<&str, Vec<DefInfo>> = HashMap::new();
        let mut static_violations: Vec<Violation> = Vec::new();

        for (path, file) in ast_context.iter_all_files() {
            let ext = path.extension().and_then(|e| e.to_str());

            if ext == Some("h") {
                for decl in file.iter_function_declarations() {
                    if !decl.is_static {
                        global_decls
                            .entry(decl.name.as_str())
                            .or_insert_with(|| DeclInfo {
                                return_type: &decl.return_type,
                                parameters: &decl.parameters,
                            });
                    }
                }
            }

            if ext == Some("c") {
                let mut local_decls: HashMap<&str, DeclInfo> = HashMap::new();
                for decl in file.iter_function_declarations() {
                    local_decls
                        .entry(decl.name.as_str())
                        .or_insert_with(|| DeclInfo {
                            return_type: &decl.return_type,
                            parameters: &decl.parameters,
                        });
                }

                for func in file.iter_function_definitions() {
                    if func.is_static {
                        if let Some(decl) = local_decls.get(func.name.as_str()) {
                            self.check_signatures(
                                &func.name,
                                decl.return_type,
                                decl.parameters,
                                &func.return_type,
                                &func.parameters,
                                path,
                                func.location.line,
                                func.location.column,
                                &mut static_violations,
                            );
                        }
                    } else {
                        all_defs
                            .entry(func.name.as_str())
                            .or_default()
                            .push(DefInfo {
                                line: func.location.line,
                                column: func.location.column,
                                path,
                                return_type: &func.return_type,
                                parameters: &func.parameters,
                            });
                    }
                }
            }
        }

        for (name, defs) in &all_defs {
            let Some(decl) = global_decls.get(name) else {
                continue;
            };

            let first = &defs[0];
            let definitions_agree = defs.iter().skip(1).all(|d| {
                first.return_type.matches(d.return_type)
                    && self.params_match(first.parameters, d.parameters)
            });
            if !definitions_agree {
                continue;
            }

            for def in defs {
                self.check_signatures(
                    name,
                    decl.return_type,
                    decl.parameters,
                    def.return_type,
                    def.parameters,
                    def.path,
                    def.line,
                    def.column,
                    violations,
                );
            }
        }

        violations.extend(static_violations);
    }
}

impl InconsistentFunctionSignature {
    /// `(void)` and `()` both mean "no parameters" in C.
    fn effective_params<'a>(&self, params: &'a [Parameter]) -> &'a [Parameter] {
        if let [
            Parameter::Regular {
                name: None,
                type_info,
                ..
            },
        ] = params
            && type_info.base_type == "void"
            && type_info.pointer_depth == 0
        {
            return &[];
        }
        params
    }

    fn params_match(&self, a: &[Parameter], b: &[Parameter]) -> bool {
        let a = self.effective_params(a);
        let b = self.effective_params(b);
        a.len() == b.len()
            && a.iter().zip(b.iter()).all(|(pa, pb)| match (pa, pb) {
                (
                    Parameter::Regular { type_info: ta, .. },
                    Parameter::Regular { type_info: tb, .. },
                ) => ta.matches(tb),
                (Parameter::Variadic, Parameter::Variadic) => true,
                _ => false,
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn check_signatures(
        &self,
        name: &str,
        decl_ret: &TypeInfo,
        decl_params: &[Parameter],
        def_ret: &TypeInfo,
        def_params: &[Parameter],
        path: &Path,
        line: usize,
        column: usize,
        violations: &mut Vec<Violation>,
    ) {
        if !decl_ret.matches(def_ret) {
            violations.push(self.violation(
                path,
                line,
                column,
                format!(
                    "'{}' declared as returning '{}' but defined as returning '{}'",
                    name, decl_ret.full_text, def_ret.full_text,
                ),
            ));
        }

        let decl_params = self.effective_params(decl_params);
        let def_params = self.effective_params(def_params);

        if decl_params.len() != def_params.len() {
            violations.push(self.violation(
                path,
                line,
                column,
                format!(
                    "'{}' declared with {} parameter(s) but defined with {}",
                    name,
                    decl_params.len(),
                    def_params.len(),
                ),
            ));
            return;
        }

        for (i, (dp, fp)) in decl_params.iter().zip(def_params.iter()).enumerate() {
            match (dp, fp) {
                (Parameter::Variadic, Parameter::Variadic) => {}
                (
                    Parameter::Regular {
                        type_info: dt,
                        name: dn,
                        ..
                    },
                    Parameter::Regular {
                        type_info: ft,
                        name: fn_,
                        ..
                    },
                ) => {
                    if !dt.matches(ft) {
                        let param_id = dn
                            .as_deref()
                            .or(fn_.as_deref())
                            .map_or_else(|| format!("{}", i + 1), |n| format!("'{n}'"));
                        violations.push(self.violation(
                            path,
                            line,
                            column,
                            format!(
                                "'{}' parameter {} declared as '{}' but defined as '{}'",
                                name, param_id, dt.full_text, ft.full_text,
                            ),
                        ));
                    }
                }
                _ => {
                    violations.push(self.violation(
                        path,
                        line,
                        column,
                        format!("'{}' parameter {} variadic mismatch between declaration and definition", name, i + 1),
                    ));
                }
            }
        }
    }
}
