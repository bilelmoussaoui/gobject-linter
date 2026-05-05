use std::collections::{HashMap, HashSet};

use gobject_ast::model::{
    TypeInfo,
    expression::{Designator, Expression},
    top_level::{PreprocessorDirective, TopLevelItem, TypeDefItem},
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct DeadCode;

impl Rule for DeadCode {
    fn name(&self) -> &'static str {
        "dead_code"
    }

    fn description(&self) -> &'static str {
        "Detect unused internal functions and types"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(
            "Detects internal functions and types that are never used anywhere in the codebase. \
             For functions: tracks both direct calls and function pointer usage (e.g., callbacks). \
             For types: tracks usage in variable declarations, casts, sizeof, and GObject macros. \
             Only reports items in private headers (not installed by meson) and static functions/types \
             defined in .c files.",
        )
    }

    fn category(&self) -> Category {
        Category::Suspicious
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

        let aliases = AliasMaps::new(ast_context);
        let (func_defs, func_decls) = collect_function_maps(ast_context);
        let (func_refs, type_refs) = aliases.collect_references(ast_context);
        let (field_refs_qualified, field_refs_unqualified) =
            aliases.collect_field_refs(ast_context);

        let type_defs = collect_type_defs(ast_context);
        let field_defs = collect_field_defs(ast_context);
        let enum_value_defs = collect_enum_value_defs(ast_context);

        self.report_function_violations(
            ast_context,
            &func_defs,
            &func_decls,
            &func_refs,
            violations,
        );
        self.report_type_violations(&type_defs, &type_refs, &aliases, violations);
        self.report_enum_value_violations(&enum_value_defs, &func_refs, violations);
        self.report_field_violations(
            &field_defs,
            &field_refs_qualified,
            &field_refs_unqualified,
            &aliases,
            violations,
        );
    }
}

struct AliasMaps {
    typedef_to_tag: HashMap<String, String>,
    tag_to_typedef: HashMap<String, String>,
}

impl AliasMaps {
    fn new(ast_context: &AstContext) -> Self {
        let mut typedef_to_tag: HashMap<String, String> = HashMap::new();
        let mut tag_to_typedef: HashMap<String, String> = HashMap::new();

        // G_DECLARE_FINAL_TYPE etc. expand to `typedef struct _Foo Foo` at compile
        // time; synthesise that alias for every known GObject type our parser sees.
        for (_path, file) in ast_context.iter_all_files() {
            for (name, target) in file.iter_typedef_pairs() {
                typedef_to_tag.insert(name.to_owned(), target.base_type.clone());
                if target.is_struct || target.is_union {
                    tag_to_typedef.insert(target.base_type.clone(), name.to_owned());
                }
            }
            for gt in file.iter_all_gobject_types() {
                let tag = format!("_{}", gt.type_name);
                typedef_to_tag
                    .entry(gt.type_name.clone())
                    .or_insert_with(|| tag.clone());
                tag_to_typedef
                    .entry(tag)
                    .or_insert_with(|| gt.type_name.clone());
            }
        }

        Self {
            typedef_to_tag,
            tag_to_typedef,
        }
    }

    fn canonical<'a>(&'a self, name: &'a str) -> &'a str {
        self.typedef_to_tag
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(name)
    }

    fn is_referenced(&self, name: &str, refs: &HashSet<String>) -> bool {
        refs.contains(name)
            || self
                .typedef_to_tag
                .get(name)
                .is_some_and(|t| refs.contains(t))
            || self
                .tag_to_typedef
                .get(name)
                .is_some_and(|a| refs.contains(a))
    }

    fn field_is_referenced(
        &self,
        struct_name: &str,
        field_name: &str,
        qualified: &HashSet<(String, String)>,
    ) -> bool {
        let has = |s: &str| qualified.contains(&(s.to_owned(), field_name.to_owned()));
        has(struct_name)
            || self.typedef_to_tag.get(struct_name).is_some_and(|t| has(t))
            || self.tag_to_typedef.get(struct_name).is_some_and(|a| has(a))
    }

    fn type_map_for(
        &self,
        func: &gobject_ast::model::top_level::FunctionDefItem,
    ) -> HashMap<String, String> {
        use gobject_ast::model::statement::Statement;
        let mut map: HashMap<String, String> = HashMap::new();
        for param in &func.parameters {
            if let Some(name) = &param.name {
                let base = &param.type_info.base_type;
                if !base.is_empty() {
                    map.insert(name.clone(), self.canonical(base).to_owned());
                }
            }
        }
        for stmt in &func.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Declaration(decl) = s {
                    let base = &decl.type_info.base_type;
                    if !base.is_empty() {
                        map.insert(decl.name.clone(), self.canonical(base).to_owned());
                    }
                }
            });
        }
        map
    }

    fn insert_qualified(
        &self,
        type_name: &str,
        field_name: &str,
        qualified: &mut HashSet<(String, String)>,
    ) {
        qualified.insert((type_name.to_owned(), field_name.to_owned()));
        if let Some(alias) = self.tag_to_typedef.get(type_name) {
            qualified.insert((alias.clone(), field_name.to_owned()));
        }
        if let Some(tag) = self.typedef_to_tag.get(type_name) {
            qualified.insert((tag.clone(), field_name.to_owned()));
        }
    }

    fn collect_references(&self, ast_context: &AstContext) -> (HashSet<String>, HashSet<String>) {
        let mut func_refs: HashSet<String> = HashSet::new();
        let mut type_refs: HashSet<String> = HashSet::new();

        for (_path, file) in ast_context.iter_all_files() {
            for func in file.iter_function_definitions() {
                collect_type_ref(&func.return_type, &mut type_refs);
                for param in &func.parameters {
                    collect_type_ref(&param.type_info, &mut type_refs);
                }
                for stmt in &func.body_statements {
                    collect_func_refs_from_stmt(stmt, &mut func_refs);
                    collect_type_refs_from_stmt(stmt, &mut type_refs);
                }
            }

            for func in file.iter_function_declarations() {
                collect_type_ref(&func.return_type, &mut type_refs);
                for param in &func.parameters {
                    collect_type_ref(&param.type_info, &mut type_refs);
                }
            }

            for item in file.iter_all_items() {
                if let TopLevelItem::Declaration(decl) = item {
                    collect_func_refs_from_stmt(decl, &mut func_refs);
                }
                collect_type_refs_from_top_level_item(item, &mut type_refs);
                match item {
                    TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                        value: Some(value),
                        ..
                    }) => {
                        extract_function_calls_from_text(value, &mut func_refs);
                    }
                    TopLevelItem::Preprocessor(
                        PreprocessorDirective::AutoptrCleanupFunc {
                            type_name,
                            cleanup_function,
                            ..
                        }
                        | PreprocessorDirective::AutoCleanupClearFunc {
                            type_name,
                            cleanup_function,
                            ..
                        },
                    ) => {
                        func_refs.insert(cleanup_function.clone());
                        type_refs.insert(type_name.clone());
                    }
                    _ => {}
                }
            }

            collect_gobject_implicit_refs(file, &mut func_refs, &mut type_refs);

            for enum_info in file.iter_all_enums() {
                for value in &enum_info.values {
                    if let Some(expr) = &value.value_expr {
                        func_refs.extend(expr.collect_identifiers());
                    }
                }
            }
        }

        (func_refs, type_refs)
    }

    fn collect_field_refs(
        &self,
        ast_context: &AstContext,
    ) -> (HashSet<(String, String)>, HashSet<String>) {
        let mut qualified: HashSet<(String, String)> = HashSet::new();
        let mut unqualified: HashSet<String> = HashSet::new();
        let empty_map: HashMap<String, String> = HashMap::new();

        for (_path, file) in ast_context.iter_all_files() {
            for func in file.iter_function_definitions() {
                let type_map = self.type_map_for(func);
                for stmt in &func.body_statements {
                    collect_field_refs_from_stmt(
                        stmt,
                        &type_map,
                        self,
                        &mut qualified,
                        &mut unqualified,
                    );
                }
            }

            for item in file.iter_all_items() {
                match item {
                    TopLevelItem::Declaration(stmt) => {
                        collect_field_refs_from_stmt(
                            stmt,
                            &empty_map,
                            self,
                            &mut qualified,
                            &mut unqualified,
                        );
                    }
                    TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                        value: Some(value),
                        ..
                    }) => {
                        extract_field_refs_from_text(value, &mut unqualified);
                    }
                    _ => {}
                }
            }
        }

        (qualified, unqualified)
    }
}

type FuncDefMap<'a> =
    HashMap<String, Vec<(&'a std::path::Path, bool, gobject_ast::SourceLocation)>>;
type FuncDeclMap<'a> = HashMap<String, Vec<(&'a std::path::Path, gobject_ast::SourceLocation)>>;

fn collect_function_maps<'a>(ast_context: &'a AstContext) -> (FuncDefMap<'a>, FuncDeclMap<'a>) {
    let mut defs: FuncDefMap = HashMap::new();
    let mut decls: FuncDeclMap = HashMap::new();

    for (path, file) in ast_context.iter_c_files() {
        for func in file.iter_function_definitions() {
            defs.entry(func.name.clone())
                .or_default()
                .push((path, func.is_static, func.location));
        }
    }

    for (path, file) in ast_context.iter_header_files() {
        for func in file.iter_function_declarations() {
            decls
                .entry(func.name.clone())
                .or_default()
                .push((path, func.location));
        }
    }

    (defs, decls)
}

type TypeDefMap<'a> = HashMap<String, Vec<(&'a std::path::Path, gobject_ast::SourceLocation)>>;

fn collect_type_defs<'a>(ast_context: &'a AstContext) -> TypeDefMap<'a> {
    let mut defs: TypeDefMap = HashMap::new();
    for (path, file) in ast_context.iter_private_files() {
        for item in file.iter_all_items() {
            match item {
                TopLevelItem::TypeDefinition(TypeDefItem::Struct {
                    name,
                    has_body: true,
                    location,
                    ..
                }) => {
                    defs.entry(name.clone())
                        .or_default()
                        .push((path, *location));
                }
                TopLevelItem::TypeDefinition(TypeDefItem::Typedef { name, location, .. }) => {
                    defs.entry(name.clone())
                        .or_default()
                        .push((path, *location));
                }
                _ => {}
            }
        }
    }
    defs
}

type EnumValueDefMap<'a> = HashMap<String, Vec<(&'a std::path::Path, gobject_ast::SourceLocation)>>;

fn collect_enum_value_defs<'a>(ast_context: &'a AstContext) -> EnumValueDefMap<'a> {
    let mut defs: EnumValueDefMap = HashMap::new();
    for (path, file) in ast_context.iter_private_files() {
        for enum_info in file.iter_all_enums() {
            for value in &enum_info.values {
                if value.is_prop_0()
                    || value.is_prop_last()
                    || value.is_signal_last()
                    || (value.value == Some(0) && value.value_expr.is_some())
                {
                    continue;
                }
                defs.entry(value.name.clone())
                    .or_default()
                    .push((path, value.location));
            }
        }
    }
    defs
}

type FieldDefMap<'a> =
    HashMap<String, Vec<(&'a std::path::Path, gobject_ast::SourceLocation, String)>>;

fn collect_field_defs<'a>(ast_context: &'a AstContext) -> FieldDefMap<'a> {
    let mut defs: FieldDefMap = HashMap::new();
    for (path, file) in ast_context.iter_private_files() {
        for item in file.iter_all_items() {
            match item {
                TopLevelItem::TypeDefinition(
                    td @ TypeDefItem::Struct {
                        name,
                        has_body: true,
                        fields,
                        ..
                    },
                ) if !td.is_vtable_struct() => {
                    collect_fields_into_defs(fields, name, path, &mut defs);
                }
                TopLevelItem::TypeDefinition(
                    td @ TypeDefItem::Typedef {
                        name,
                        struct_fields,
                        ..
                    },
                ) if !td.is_vtable_struct() => {
                    collect_fields_into_defs(struct_fields, name, path, &mut defs);
                }
                _ => {}
            }
        }
    }
    defs
}

fn collect_fields_into_defs<'a>(
    fields: &'a [gobject_ast::model::top_level::StructField],
    struct_name: &str,
    path: &'a std::path::Path,
    defs: &mut FieldDefMap<'a>,
) {
    let last_non_reserved = fields.iter().rposition(|f| !f.is_reserved());

    for (idx, field) in fields.iter().enumerate() {
        let Some(field_name) = &field.field_name else {
            continue;
        };
        if !field.inner_fields.is_empty() {
            collect_fields_into_defs(&field.inner_fields, struct_name, path, defs);
            continue;
        }
        if idx == 0 && field_name.starts_with("parent") {
            continue;
        }
        if field.is_reserved() && last_non_reserved.is_none_or(|last| idx > last) {
            continue;
        }
        defs.entry(field_name.clone()).or_default().push((
            path,
            field.location,
            struct_name.to_owned(),
        ));
    }
}

fn collect_gobject_implicit_refs(
    file: &gobject_ast::FileModel,
    func_refs: &mut HashSet<String>,
    type_refs: &mut HashSet<String>,
) {
    use gobject_ast::model::types::GObjectTypeKind;

    for gt in file.iter_all_gobject_types() {
        if gt.is_interface() {
            func_refs.insert(gt.default_init_function_name());
        } else {
            func_refs.insert(gt.class_init_function_name());
            func_refs.insert(gt.init_function_name());
        }

        for iface in &gt.interfaces {
            func_refs.insert(iface.init_function.clone());
        }

        if let GObjectTypeKind::DefineBoxed {
            copy_func,
            free_func,
        } = &gt.kind
        {
            func_refs.insert(copy_func.clone());
            func_refs.insert(free_func.clone());
        }

        if let Some(quark_fn) = gt.kind.quark_function_name() {
            func_refs.insert(quark_fn);
        }

        if gt.has_private {
            let priv_name = format!("{}Private", gt.type_name);
            type_refs.insert(priv_name.clone());
            type_refs.insert(format!("_{priv_name}"));
        }

        let tn = &gt.type_name;
        type_refs.insert(format!("_{tn}"));
        if gt.is_interface() {
            type_refs.insert(format!("_{tn}Interface"));
        } else if !matches!(gt.kind, GObjectTypeKind::DefineBoxed { .. }) {
            type_refs.insert(format!("_{tn}Class"));
        }

        for stmt in &gt.code_block_statements {
            collect_func_refs_from_stmt(stmt, func_refs);
            collect_type_refs_from_stmt(stmt, type_refs);
        }
    }
}

fn collect_type_ref(type_info: &TypeInfo, refs: &mut HashSet<String>) {
    if !type_info.base_type.is_empty() {
        refs.insert(type_info.base_type.clone());
    }
}

fn collect_type_refs_from_stmt(
    stmt: &gobject_ast::model::statement::Statement,
    refs: &mut HashSet<String>,
) {
    use gobject_ast::model::statement::Statement;
    stmt.walk(&mut |s| {
        if let Statement::Declaration(decl) = s {
            collect_type_ref(&decl.type_info, refs);
        }
    });
    stmt.walk_expressions(&mut |expr| {
        expr.walk(&mut |e| match e {
            Expression::Cast(cast) => collect_type_ref(&cast.type_info, refs),
            Expression::Sizeof(sizeof) => {
                if let Some(name) = sizeof.type_name()
                    && !name.is_empty()
                {
                    refs.insert(name);
                }
            }
            _ => {}
        });
    });
}

fn collect_type_refs_from_top_level_item(item: &TopLevelItem, refs: &mut HashSet<String>) {
    match item {
        TopLevelItem::Declaration(stmt) => collect_type_refs_from_stmt(stmt, refs),
        TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
            target,
            struct_fields,
            ..
        }) => {
            if let Some(target_type) = target.as_type() {
                collect_type_ref(target_type, refs);
            }
            collect_type_refs_from_fields(struct_fields, refs);
        }
        TopLevelItem::TypeDefinition(TypeDefItem::Struct { fields, .. }) => {
            collect_type_refs_from_fields(fields, refs);
        }
        _ => {}
    }
}

fn collect_type_refs_from_fields(
    fields: &[gobject_ast::model::top_level::StructField],
    refs: &mut HashSet<String>,
) {
    for field in fields {
        field.walk(&mut |f| collect_type_ref(&f.field_type, refs));
    }
}

fn collect_func_refs_from_stmt(
    stmt: &gobject_ast::model::statement::Statement,
    refs: &mut HashSet<String>,
) {
    use gobject_ast::model::statement::Statement;
    stmt.walk_expressions(&mut |expr| refs.extend(expr.collect_identifiers()));
    stmt.walk(&mut |s| {
        if let Statement::Preprocessor(PreprocessorDirective::Define {
            value: Some(value), ..
        }) = s
        {
            extract_function_calls_from_text(value, refs);
        }
    });
}

fn extract_function_calls_from_text(text: &str, refs: &mut HashSet<String>) {
    let mut chars = text.chars().peekable();
    let mut ident = String::new();

    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
        } else {
            if !ident.is_empty() {
                if c == '(' {
                    refs.insert(ident.clone());
                } else if c.is_whitespace() || c == '\\' {
                    let mut lookahead = chars.clone();
                    while let Some(&nc) = lookahead.peek() {
                        if nc.is_whitespace() || nc == '\\' {
                            lookahead.next();
                        } else {
                            if nc == '(' {
                                refs.insert(ident.clone());
                            }
                            break;
                        }
                    }
                }
                ident.clear();
            }
        }
    }
}

fn collect_field_refs_from_stmt(
    stmt: &gobject_ast::model::statement::Statement,
    type_map: &HashMap<String, String>,
    aliases: &AliasMaps,
    qualified: &mut HashSet<(String, String)>,
    unqualified: &mut HashSet<String>,
) {
    use gobject_ast::model::statement::Statement;

    stmt.walk_expressions(&mut |expr| {
        expr.walk(&mut |e| match e {
            Expression::FieldAccess(f) => {
                if let Expression::Identifier(id) = f.base.as_ref()
                    && let Some(type_name) = type_map.get(&id.name)
                {
                    aliases.insert_qualified(type_name, &f.field, qualified);
                    return;
                }
                unqualified.insert(f.field.clone());
            }
            Expression::InitializerList(init) => {
                for item in &init.items {
                    if let Some(Designator::Field(name)) = &item.designator {
                        unqualified.insert(name.clone());
                    }
                }
            }
            _ => {}
        });
    });

    stmt.walk(&mut |s| {
        if let Statement::Preprocessor(PreprocessorDirective::Define {
            value: Some(value), ..
        }) = s
        {
            extract_field_refs_from_text(value, unqualified);
        }
    });
}

fn extract_field_refs_from_text(text: &str, unqualified: &mut HashSet<String>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let arrow = i + 1 < b.len() && b[i] == b'-' && b[i + 1] == b'>';
        let dot = b[i] == b'.'
            && i > 0
            && (b[i - 1].is_ascii_alphanumeric()
                || b[i - 1] == b'_'
                || matches!(b[i - 1], b')' | b']'));
        if arrow || dot {
            let start = i + if arrow { 2 } else { 1 };
            let mut end = start;
            while end < b.len() && (b[end].is_ascii_alphanumeric() || b[end] == b'_') {
                end += 1;
            }
            if end > start {
                unqualified.insert(text[start..end].to_owned());
            }
            i = end;
        } else {
            i += 1;
        }
    }
}

impl DeadCode {
    fn report_function_violations(
        &self,
        ast_context: &AstContext,
        func_defs: &FuncDefMap,
        func_decls: &FuncDeclMap,
        func_refs: &HashSet<String>,
        violations: &mut Vec<Violation>,
    ) {
        for (func_name, defs) in func_defs {
            if func_refs.contains(func_name) {
                continue;
            }
            for (def_path, is_static, location) in defs {
                if *is_static {
                    violations.push(self.violation(
                        def_path,
                        location.line,
                        location.column,
                        format!("Static function '{}' is never used", func_name),
                    ));
                    continue;
                }
                if let Some(decls) = func_decls.get(func_name) {
                    if decls
                        .iter()
                        .any(|(p, _)| ast_context.is_public_header(p) == Some(true))
                    {
                        continue;
                    }
                    for (decl_path, decl_location) in decls {
                        violations.push(self.violation(
                            decl_path,
                            decl_location.line,
                            decl_location.column,
                            format!(
                                "Internal function '{}' is never used (declared in private header)",
                                func_name
                            ),
                        ));
                    }
                }
            }
        }

        for (func_name, decls) in func_decls {
            if func_refs.contains(func_name) || func_defs.contains_key(func_name) {
                continue;
            }
            if decls
                .iter()
                .any(|(p, _)| ast_context.is_public_header(p) == Some(true))
            {
                continue;
            }
            for (decl_path, decl_location) in decls {
                violations.push(self.violation(
                    decl_path,
                    decl_location.line,
                    decl_location.column,
                    format!(
                        "Internal function '{}' is never used (declared but not defined)",
                        func_name
                    ),
                ));
            }
        }
    }

    fn report_type_violations(
        &self,
        type_defs: &TypeDefMap,
        type_refs: &HashSet<String>,
        aliases: &AliasMaps,
        violations: &mut Vec<Violation>,
    ) {
        for (type_name, defs) in type_defs {
            if aliases.is_referenced(type_name, type_refs) {
                continue;
            }
            for (def_path, location) in defs {
                violations.push(self.violation(
                    def_path,
                    location.line,
                    location.column,
                    format!("Type '{}' is defined but never used", type_name),
                ));
            }
        }
    }

    fn report_enum_value_violations(
        &self,
        enum_value_defs: &EnumValueDefMap,
        func_refs: &HashSet<String>,
        violations: &mut Vec<Violation>,
    ) {
        for (value_name, defs) in enum_value_defs {
            if func_refs.contains(value_name) {
                continue;
            }
            for (def_path, location) in defs {
                violations.push(self.violation(
                    def_path,
                    location.line,
                    location.column,
                    format!("Enum value '{}' is defined but never used", value_name),
                ));
            }
        }
    }

    fn report_field_violations(
        &self,
        field_defs: &FieldDefMap,
        field_refs_qualified: &HashSet<(String, String)>,
        field_refs_unqualified: &HashSet<String>,
        aliases: &AliasMaps,
        violations: &mut Vec<Violation>,
    ) {
        for (field_name, defs) in field_defs {
            if field_refs_unqualified.contains(field_name) {
                continue;
            }
            for (def_path, location, struct_name) in defs {
                if aliases.field_is_referenced(struct_name, field_name, field_refs_qualified) {
                    continue;
                }
                violations.push(self.violation(
                    def_path,
                    location.line,
                    location.column,
                    format!("Field '{}' in '{}' is never used", field_name, struct_name),
                ));
            }
        }
    }
}
