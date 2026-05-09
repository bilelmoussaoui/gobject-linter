use gobject_ast::model::{FunctionAnnotation, PropertyAnnotation};

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

struct MacroVersion {
    version: String,
    keyword: &'static str,
}

fn extract_version_from_macro(macro_name: &str) -> Option<MacroVersion> {
    let (suffix, keyword) = if let Some(s) = macro_name.split("AVAILABLE_IN_").nth(1) {
        (s, "AVAILABLE_IN")
    } else {
        let s = macro_name.split("DEPRECATED_IN_").nth(1)?;
        (s, "DEPRECATED_IN")
    };
    let (major, minor) = suffix.split_once('_')?;
    if major.chars().all(|c| c.is_ascii_digit()) && minor.chars().all(|c| c.is_ascii_digit()) {
        Some(MacroVersion {
            version: format!("{major}.{minor}"),
            keyword,
        })
    } else {
        None
    }
}

fn extract_version_from_macros(macros: &[String]) -> Option<MacroVersion> {
    macros.iter().find_map(|m| extract_version_from_macro(m))
}

fn extract_deprecated_version(deprecated: &str) -> Option<&str> {
    let trimmed = deprecated.trim();
    let ver = trimmed.split(':').next()?.trim();
    let ver = ver.trim_end_matches('.');
    if ver.contains('.') && ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some(ver)
    } else {
        None
    }
}

impl GiMissingSince {
    fn check_type_since(&self, ast_context: &AstContext, violations: &mut Vec<Violation>) {
        for (path, file) in ast_context.iter_header_files() {
            if !ast_context.is_public_header(path).unwrap_or(false) {
                continue;
            }

            for gt in file.iter_all_gobject_types() {
                let macro_ver = extract_version_from_macros(&gt.export_macros);
                let type_doc = ast_context.find_type_doc(&gt.type_name);

                match &macro_ver {
                    Some(mv) if mv.keyword == "DEPRECATED_IN" => {
                        let deprecated_ver = type_doc
                            .and_then(|d| d.deprecated.as_deref())
                            .and_then(extract_deprecated_version)
                            .map(str::to_owned);
                        match deprecated_ver.as_deref() {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has DEPRECATED_IN_{} but is missing a Deprecated: annotation",
                                        gt.type_name,
                                        mv.version.replace('.', "_"),
                                    ),
                                ));
                            }
                            Some(dep_ver) if dep_ver != mv.version => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has DEPRECATED_IN_{} but Deprecated: says {}",
                                        gt.type_name,
                                        mv.version.replace('.', "_"),
                                        dep_ver,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    Some(mv) => {
                        let type_since = type_doc.and_then(|d| d.since.clone());
                        match type_since.as_deref() {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has AVAILABLE_IN_{} but is missing a Since: annotation",
                                        gt.type_name,
                                        mv.version.replace('.', "_"),
                                    ),
                                ));
                            }
                            Some(since_ver) if since_ver != mv.version => {
                                violations.push(self.violation(
                                    path,
                                    gt.location.line,
                                    gt.location.column,
                                    format!(
                                        "Type '{}' has AVAILABLE_IN_{} but Since: says {}",
                                        gt.type_name,
                                        mv.version.replace('.', "_"),
                                        since_ver,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    None if !gt.export_macros.is_empty() => {
                        let type_since = type_doc.and_then(|d| d.since.clone());
                        if let Some(since_ver) = type_since {
                            violations.push(self.violation(
                                path,
                                gt.location.line,
                                gt.location.column,
                                format!(
                                    "Type '{}' has Since: {} but is missing a versioned export macro",
                                    gt.type_name,
                                    since_ver,
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

                let macro_ver = extract_version_from_macros(&func_decl.export_macros);
                let func_doc = ast_context.find_func_doc(&func_decl.name);

                match &macro_ver {
                    Some(mv) if mv.keyword == "DEPRECATED_IN" => {
                        let deprecated_ver = func_doc
                            .and_then(|d| d.deprecated.as_deref())
                            .and_then(extract_deprecated_version)
                            .map(str::to_owned);
                        match deprecated_ver.as_deref() {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has DEPRECATED_IN_{} but is missing a Deprecated: annotation",
                                        func_decl.name,
                                        mv.version.replace('.', "_"),
                                    ),
                                ));
                            }
                            Some(dep_ver) if dep_ver != mv.version => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has DEPRECATED_IN_{} but Deprecated: says {}",
                                        func_decl.name,
                                        mv.version.replace('.', "_"),
                                        dep_ver,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    Some(mv) => {
                        let since = func_doc.and_then(|d| d.since.clone());

                        let parent_type_ver = file
                            .iter_all_gobject_types()
                            .find(|gt| func_decl.name.starts_with(&gt.function_prefix))
                            .and_then(|gt| extract_version_from_macros(&gt.export_macros))
                            .or_else(|| {
                                file.iter_function_declarations()
                                    .filter(|d| d.name.ends_with("_get_type"))
                                    .find(|d| {
                                        let prefix = &d.name[..d.name.len() - "_get_type".len()];
                                        func_decl.name.starts_with(prefix)
                                    })
                                    .and_then(|d| extract_version_from_macros(&d.export_macros))
                            });

                        if parent_type_ver
                            .as_ref()
                            .is_some_and(|p| p.version == mv.version)
                        {
                            continue;
                        }

                        match since.as_deref() {
                            None => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has AVAILABLE_IN_{} but is missing a Since: annotation",
                                        func_decl.name,
                                        mv.version.replace('.', "_"),
                                    ),
                                ));
                            }
                            Some(since_ver) if since_ver != mv.version => {
                                violations.push(self.violation(
                                    path,
                                    func_decl.location.line,
                                    func_decl.location.column,
                                    format!(
                                        "Function '{}' has AVAILABLE_IN_{} but Since: says {}",
                                        func_decl.name,
                                        mv.version.replace('.', "_"),
                                        since_ver,
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                    None if !func_decl.export_macros.is_empty() => {
                        let since = func_doc.and_then(|d| d.since.clone());
                        if let Some(since_ver) = since {
                            violations.push(self.violation(
                                path,
                                func_decl.location.line,
                                func_decl.location.column,
                                format!(
                                    "Function '{}' has Since: {} but is missing a versioned export macro",
                                    func_decl.name,
                                    since_ver,
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
                    let prop_since = property.doc.as_ref().and_then(|d| d.since.as_deref());

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
                        let func_since = self.find_function_since(ast_context, func_name);

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

    fn find_function_since<'a>(
        &self,
        ast_context: &'a AstContext,
        func_name: &str,
    ) -> Option<&'a str> {
        for (_, file) in ast_context.iter_header_files() {
            for func_decl in file.iter_function_declarations() {
                if func_decl.name == func_name
                    && let Some(since) = func_decl.doc.as_ref().and_then(|d| d.since.as_deref())
                {
                    return Some(since);
                }
            }
        }

        for (_, file) in ast_context.iter_c_files() {
            for func_def in file.iter_function_definitions() {
                if func_def.name == func_name
                    && let Some(since) = func_def.doc.as_ref().and_then(|d| d.since.as_deref())
                {
                    return Some(since);
                }
            }
        }

        None
    }
}
