use gobject_ast::model::{ExportMacro, FunctionAnnotation, PropertyAnnotation};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct GiMissingSince;

impl Rule for GiMissingSince {
    fn name(&self) -> &'static str {
        "gi_missing_since"
    }

    fn description(&self) -> &'static str {
        "Detect public API with AVAILABLE_IN macros but missing or mismatched Since: annotations"
    }

    fn category(&self) -> Category {
        Category::Introspection
    }

    fn requires_meson(&self) -> bool {
        true
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn opt_in_reason(&self) -> Option<&'static str> {
        Some("Only relevant to libraries maintaining GObject Introspection annotations")
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        if !ast_context.has_public_private_info() {
            return;
        }

        self.check_type_since(ast_context, violations);
        self.check_functions_since(ast_context, violations);
        self.check_property_since_consistency(ast_context, violations);
    }
}

impl GiMissingSince {
    fn check_type_since(&self, ast_context: &AstContext, violations: &mut Vec<Violation>) {
        for (path, file) in ast_context.iter_header_files() {
            if !ast_context.is_public_header(path).unwrap_or(false) {
                continue;
            }

            for gt in file.iter_all_gobject_types() {
                let type_doc = ast_context.find_type_doc(&gt.type_name);

                match gt.export_macros.iter().find(|m| m.version().is_some()) {
                    Some(
                        ExportMacro::DeprecatedIn(macro_ver)
                        | ExportMacro::DeprecatedInFor(macro_ver, _),
                    ) => {
                        let dep_ver = type_doc.and_then(|d| d.deprecated.as_ref().map(|(v, _)| v));
                        match dep_ver {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has DEPRECATED_IN_{}_{} but is missing a Deprecated: annotation",
                                        gt.type_name,
                                        macro_ver.major, macro_ver.minor,
                                    ),
                                ));
                            }
                            Some(v) if v != macro_ver => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has DEPRECATED_IN_{}_{} but Deprecated: says {}",
                                        gt.type_name, macro_ver.major, macro_ver.minor, v,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    Some(ExportMacro::AvailableIn(macro_ver)) => {
                        let since = type_doc.and_then(|d| d.since.as_ref());
                        match since {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has AVAILABLE_IN_{}_{} but is missing a Since: annotation",
                                        gt.type_name,
                                        macro_ver.major, macro_ver.minor,
                                    ),
                                ));
                            }
                            Some(v) if v != macro_ver => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has AVAILABLE_IN_{}_{} but Since: says {}",
                                        gt.type_name, macro_ver.major, macro_ver.minor, v,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    None if !gt.export_macros.is_empty() => {
                        if let Some(since) = type_doc.and_then(|d| d.since.as_ref()) {
                            violations.push(self.violation(
                                path,
                                gt.location.line,
                                gt.location.column,
                                format!(
                                    "Type '{}' has Since: {} but is missing a versioned export macro",
                                    gt.type_name,
                                    since,
                                ),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_functions_since(&self, ast_context: &AstContext, violations: &mut Vec<Violation>) {
        for (path, file) in ast_context.iter_header_files() {
            if !ast_context.is_public_header(path).unwrap_or(false) {
                continue;
            }

            for func_decl in file.iter_function_declarations() {
                if func_decl.name.ends_with("_get_type") || func_decl.name.ends_with("_error_quark")
                {
                    continue;
                }

                let func_doc = ast_context.find_func_doc(&func_decl.name);

                match func_decl
                    .export_macros
                    .iter()
                    .find(|m| m.version().is_some())
                {
                    Some(
                        ExportMacro::DeprecatedIn(macro_ver)
                        | ExportMacro::DeprecatedInFor(macro_ver, _),
                    ) => {
                        let dep_ver = func_doc.and_then(|d| d.deprecated.as_ref().map(|(v, _)| v));
                        match dep_ver {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has DEPRECATED_IN_{}_{} but is missing a Deprecated: annotation",
                                        func_decl.name,
                                        macro_ver.major, macro_ver.minor,
                                    ),
                                ));
                            }
                            Some(v) if v != macro_ver => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has DEPRECATED_IN_{}_{} but Deprecated: says {}",
                                        func_decl.name,
                                        macro_ver.major, macro_ver.minor, v,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    Some(ExportMacro::AvailableIn(macro_ver)) => {
                        let since = func_doc.and_then(|d| d.since.as_ref());

                        let parent_type_ver = file
                            .iter_all_gobject_types()
                            .find(|gt| func_decl.name.starts_with(&gt.function_prefix))
                            .and_then(|gt| gt.export_macros.iter().find_map(|m| m.version()))
                            .or_else(|| {
                                file.iter_function_declarations()
                                    .filter(|d| d.name.ends_with("_get_type"))
                                    .find(|d| {
                                        let prefix = &d.name[..d.name.len() - "_get_type".len()];
                                        func_decl.name.starts_with(prefix)
                                    })
                                    .and_then(|d| d.export_macros.iter().find_map(|m| m.version()))
                            });

                        if parent_type_ver.is_some_and(|p| p == macro_ver) {
                            continue;
                        }

                        match since {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has AVAILABLE_IN_{}_{} but is missing a Since: annotation",
                                        func_decl.name,
                                        macro_ver.major, macro_ver.minor,
                                    ),
                                ));
                            }
                            Some(v) if v != macro_ver => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has AVAILABLE_IN_{}_{} but Since: says {}",
                                        func_decl.name, macro_ver.major, macro_ver.minor, v,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    None if !func_decl.export_macros.is_empty() => {
                        if let Some(since) = func_doc.and_then(|d| d.since.as_ref()) {
                            violations.push(self.violation(
                                path,
                                func_decl.location.line,
                                func_decl.location.column,
                                format!(
                                    "Function '{}' has Since: {} but is missing a versioned export macro",
                                    func_decl.name,
                                    since,
                                ),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_property_since_consistency(
        &self,
        ast_context: &AstContext,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_all_files() {
            for gt in file.iter_all_gobject_types() {
                for prop_assignment in &gt.properties {
                    let property = prop_assignment.property();
                    let prop_name = &property.name;
                    let prop_since = property.doc.as_ref().and_then(|d| d.since.as_ref());

                    let mut getter_setter_names: Vec<(String, &str)> = Vec::new();

                    if let Some(doc) = &property.doc {
                        let prefix = &gt.function_prefix;
                        for ann in &doc.annotations {
                            match ann {
                                PropertyAnnotation::Getter(short) => {
                                    getter_setter_names
                                        .push((format!("{prefix}_{short}"), "getter"));
                                }
                                PropertyAnnotation::Setter(short) => {
                                    getter_setter_names
                                        .push((format!("{prefix}_{short}"), "setter"));
                                }
                                _ => {}
                            }
                        }
                    }

                    for (_, decl_file) in ast_context.iter_header_files() {
                        for func_decl in decl_file.iter_function_declarations() {
                            if let Some(doc) = &func_decl.doc {
                                for ann in &doc.annotations {
                                    match ann {
                                        FunctionAnnotation::GetProperty(p)
                                            if p == prop_name
                                                && !getter_setter_names
                                                    .iter()
                                                    .any(|(n, _)| n == &func_decl.name) =>
                                        {
                                            getter_setter_names
                                                .push((func_decl.name.clone(), "getter"));
                                        }
                                        FunctionAnnotation::SetProperty(p)
                                            if p == prop_name
                                                && !getter_setter_names
                                                    .iter()
                                                    .any(|(n, _)| n == &func_decl.name) =>
                                        {
                                            getter_setter_names
                                                .push((func_decl.name.clone(), "setter"));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }

                    for (func_name, role) in &getter_setter_names {
                        let func_since = ast_context
                            .find_func_doc(func_name)
                            .and_then(|d| d.since.as_ref());

                        match (prop_since, func_since) {
                            (Some(pv), None) => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Property '{prop_name}' has Since: {pv} but \
                                         {role} '{func_name}' has no Since: annotation"
                                    ),
                                ));
                            }
                            (None, Some(fv)) => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Property '{prop_name}' has no Since: annotation but \
                                         {role} '{func_name}' has Since: {fv}"
                                    ),
                                ));
                            }
                            (Some(pv), Some(fv)) if pv != fv => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Property '{prop_name}' Since: {pv} does not match \
                                         {role} '{func_name}' Since: {fv}"
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
