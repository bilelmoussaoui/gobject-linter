use std::collections::{HashMap, HashSet};

use gobject_ast::model::{
    Argument, AssignmentOp, Designator, Expression, FileModel, GObjectTypeKind, Parameter,
    PreprocessorDirective, SourceLocation, Statement, StructField, TopLevelItem, TypeDefItem,
    TypeInfo, VariableDecl,
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

    fn opt_in(&self) -> bool {
        true
    }

    fn opt_in_reason(&self) -> Option<&'static str> {
        Some(
            "May produce false positives due to fundamental limitations of static analysis without a preprocessor or full call graph",
        )
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

        let (func_defs, func_decls) = collect_function_maps(ast_context);
        let refs = collect_all_refs(ast_context);

        let (type_defs, field_defs, enum_value_defs) = collect_private_defs(ast_context);

        self.report_function_violations(
            ast_context,
            &func_defs,
            &func_decls,
            &refs.func,
            violations,
        );
        self.report_type_violations(ast_context, &type_defs, &refs.types, violations);
        self.report_enum_value_violations(&enum_value_defs, &refs.func, violations);
        self.report_field_violations(
            ast_context,
            &field_defs,
            &refs.field_qualified,
            &refs.field_unqualified,
            violations,
        );
    }
}

fn collect_all_refs(ast_context: &AstContext) -> AllRefs {
    let mut func_refs: HashSet<String> = HashSet::new();
    let mut type_refs: HashSet<String> = HashSet::new();
    let mut field_qualified: HashMap<String, HashSet<String>> = HashMap::new();
    let mut field_unqualified: HashSet<String> = HashSet::new();
    let empty_map: HashMap<&str, String> = HashMap::new();

    for (_path, file) in ast_context.iter_all_files() {
        for func in file.iter_function_definitions() {
            collect_type_ref(&func.return_type, &mut type_refs);
            for param in &func.parameters {
                if let Parameter::Regular { type_info, .. } = param {
                    collect_type_ref(type_info, &mut type_refs);
                }
            }

            let type_map: HashMap<&str, String> = func
                .local_var_types()
                .into_iter()
                .filter(|(_, ti)| !ti.base_type.is_empty())
                .map(|(name, ti)| {
                    (
                        name,
                        ast_context
                            .type_aliases()
                            .canonical(&ti.base_type)
                            .to_owned(),
                    )
                })
                .collect();

            for stmt in &func.body_statements {
                collect_func_refs_from_stmt(stmt, &mut func_refs);
                collect_type_refs_from_stmt(stmt, &mut type_refs);
                collect_field_refs_from_stmt(
                    ast_context,
                    stmt,
                    &type_map,
                    &mut field_qualified,
                    &mut field_unqualified,
                );
            }
        }

        for func in file.iter_function_declarations() {
            collect_type_ref(&func.return_type, &mut type_refs);
            for param in &func.parameters {
                if let Parameter::Regular { type_info, .. } = param {
                    collect_type_ref(type_info, &mut type_refs);
                }
            }
        }

        for item in file.iter_all_items() {
            if let TopLevelItem::Declaration(decl) = item {
                collect_func_refs_from_decl(decl, &mut func_refs);
                collect_field_refs_from_decl(
                    ast_context,
                    decl,
                    &empty_map,
                    &mut field_qualified,
                    &mut field_unqualified,
                );
            }
            collect_type_refs_from_top_level_item(item, &mut type_refs);
            match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                    value: Some(value),
                    ..
                }) => {
                    extract_function_calls_from_text(value, &mut func_refs);
                    extract_field_refs_from_text(value, &mut field_unqualified);
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

    AllRefs {
        func: func_refs,
        types: type_refs,
        field_qualified,
        field_unqualified,
    }
}

type FuncDefMap<'a> = HashMap<&'a str, Vec<(&'a std::path::Path, bool, SourceLocation)>>;
type FuncDeclMap<'a> = HashMap<&'a str, Vec<(&'a std::path::Path, SourceLocation)>>;

fn collect_function_maps<'a>(ast_context: &'a AstContext) -> (FuncDefMap<'a>, FuncDeclMap<'a>) {
    let mut defs: FuncDefMap = HashMap::new();
    let mut decls: FuncDeclMap = HashMap::new();

    for (path, file) in ast_context.iter_all_files() {
        let ext = path.extension().and_then(|e| e.to_str());
        if ext == Some("c") {
            for func in file.iter_function_definitions() {
                defs.entry(func.name.as_str()).or_default().push((
                    path,
                    func.is_static,
                    func.location,
                ));
            }
        }
        if ext == Some("h") {
            for func in file.iter_function_declarations() {
                decls
                    .entry(func.name.as_str())
                    .or_default()
                    .push((path, func.location));
            }
        }
    }

    (defs, decls)
}

struct AllRefs {
    func: HashSet<String>,
    types: HashSet<String>,
    field_qualified: HashMap<String, HashSet<String>>,
    field_unqualified: HashSet<String>,
}

type TypeDefMap<'a> = HashMap<&'a str, Vec<(&'a std::path::Path, SourceLocation)>>;

type EnumValueDefMap<'a> = HashMap<&'a str, Vec<(&'a std::path::Path, SourceLocation)>>;

type FieldDefMap<'a> = HashMap<&'a str, Vec<(&'a std::path::Path, SourceLocation, &'a str)>>;

fn collect_private_defs<'a>(
    ast_context: &'a AstContext,
) -> (TypeDefMap<'a>, FieldDefMap<'a>, EnumValueDefMap<'a>) {
    let mut type_defs: TypeDefMap = HashMap::new();
    let mut field_defs: FieldDefMap = HashMap::new();
    let mut enum_value_defs: EnumValueDefMap = HashMap::new();

    for (path, file) in ast_context.iter_private_files() {
        for item in file.iter_all_items() {
            match item {
                TopLevelItem::TypeDefinition(TypeDefItem::Struct { name, location, .. }) => {
                    type_defs
                        .entry(name.as_str())
                        .or_default()
                        .push((path, *location));
                }
                TopLevelItem::TypeDefinition(TypeDefItem::Typedef { name, location, .. }) => {
                    type_defs
                        .entry(name.as_str())
                        .or_default()
                        .push((path, *location));
                }
                _ => {}
            }

            match item {
                TopLevelItem::TypeDefinition(td @ TypeDefItem::Struct { name, fields, .. })
                    if !td.is_vtable_struct() =>
                {
                    collect_fields_into_defs(fields, name, path, &mut field_defs);
                }
                TopLevelItem::TypeDefinition(
                    td @ TypeDefItem::Typedef {
                        name,
                        struct_fields,
                        ..
                    },
                ) if !td.is_vtable_struct() => {
                    collect_fields_into_defs(struct_fields, name, path, &mut field_defs);
                }
                _ => {}
            }
        }

        for enum_info in file.iter_all_enums() {
            for value in &enum_info.values {
                if value.is_prop_0()
                    || value.is_prop_last()
                    || value.is_signal_last()
                    || (value.value == Some(0) && value.value_expr.is_some())
                {
                    continue;
                }
                enum_value_defs
                    .entry(value.name.as_str())
                    .or_default()
                    .push((path, value.location));
            }
        }
    }

    (type_defs, field_defs, enum_value_defs)
}

fn collect_fields_into_defs<'a>(
    fields: &'a [StructField],
    struct_name: &'a str,
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
        defs.entry(field_name.as_str())
            .or_default()
            .push((path, field.location, struct_name));
    }
}

fn collect_gobject_implicit_refs(
    file: &FileModel,
    func_refs: &mut HashSet<String>,
    type_refs: &mut HashSet<String>,
) {
    for gt in file.iter_all_gobject_types() {
        if gt.is_interface() {
            func_refs.insert(gt.default_init_function_name());
        } else if !matches!(
            gt.kind,
            GObjectTypeKind::DefineEnum { .. }
                | GObjectTypeKind::DefineFlags { .. }
                | GObjectTypeKind::DefineQuark { .. }
                | GObjectTypeKind::DefineCustom { .. }
        ) {
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
            type_refs.insert(format!("_{priv_name}"));
            type_refs.insert(priv_name);
        }

        let tn = &gt.type_name;
        type_refs.insert(format!("_{tn}"));
        if gt.is_interface() {
            type_refs.insert(format!("_{tn}Interface"));
        } else if !matches!(
            gt.kind,
            GObjectTypeKind::DefineBoxed { .. }
                | GObjectTypeKind::DefineEnum { .. }
                | GObjectTypeKind::DefineFlags { .. }
                | GObjectTypeKind::DefineQuark { .. }
                | GObjectTypeKind::DefineCustom { .. }
        ) {
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

fn collect_type_refs_from_stmt(stmt: &Statement, refs: &mut HashSet<String>) {
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
                    refs.insert(name.to_owned());
                }
            }
            _ => {}
        });
    });
}

fn collect_type_refs_from_top_level_item(item: &TopLevelItem, refs: &mut HashSet<String>) {
    match item {
        TopLevelItem::Declaration(decl) => collect_type_ref(&decl.type_info, refs),
        TopLevelItem::Expression(_) => {}
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

fn collect_type_refs_from_fields(fields: &[StructField], refs: &mut HashSet<String>) {
    for field in fields {
        field.walk(&mut |f| collect_type_ref(&f.field_type, refs));
    }
}

fn collect_func_refs_from_stmt(stmt: &Statement, refs: &mut HashSet<String>) {
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

fn collect_func_refs_from_decl(decl: &VariableDecl, refs: &mut HashSet<String>) {
    if let Some(init) = &decl.initializer {
        init.walk(&mut |e| refs.extend(e.collect_identifiers()));
    }
    if let Some(size) = &decl.array_size {
        size.walk(&mut |e| refs.extend(e.collect_identifiers()));
    }
}

fn collect_field_refs_from_decl(
    ast_context: &AstContext,
    decl: &VariableDecl,
    type_map: &HashMap<&str, String>,
    qualified: &mut HashMap<String, HashSet<String>>,
    unqualified: &mut HashSet<String>,
) {
    if let Some(init) = &decl.initializer {
        collect_field_reads_impl(ast_context, init, false, type_map, qualified, unqualified);
    }
    if let Some(size) = &decl.array_size {
        collect_field_reads_impl(ast_context, size, false, type_map, qualified, unqualified);
    }
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
    ast_context: &AstContext,
    stmt: &Statement,
    type_map: &HashMap<&str, String>,
    qualified: &mut HashMap<String, HashSet<String>>,
    unqualified: &mut HashSet<String>,
) {
    stmt.walk_expressions(&mut |expr| {
        collect_field_reads_impl(ast_context, expr, false, type_map, qualified, unqualified);
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

/// `is_write_lhs` is true when `expr` is the direct LHS of a plain `=`
/// assignment
fn collect_field_reads_impl(
    ast_context: &AstContext,
    expr: &Expression,
    is_write_lhs: bool,
    type_map: &HashMap<&str, String>,
    qualified: &mut HashMap<String, HashSet<String>>,
    unqualified: &mut HashSet<String>,
) {
    match expr {
        Expression::FieldAccess(f) => {
            if !is_write_lhs {
                if let Expression::Identifier(id) = f.base.as_ref()
                    && let Some(type_name) = type_map.get(id.name.as_str())
                {
                    ast_context
                        .type_aliases()
                        .insert_qualified(type_name, &f.field, qualified);
                } else {
                    unqualified.insert(f.field.clone());
                }
            }
            collect_field_reads_impl(
                ast_context,
                &f.base,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Assignment(assign) => {
            let lhs_is_write = assign.operator == AssignmentOp::Assign;
            collect_field_reads_impl(
                ast_context,
                &assign.lhs,
                lhs_is_write,
                type_map,
                qualified,
                unqualified,
            );
            collect_field_reads_impl(
                ast_context,
                &assign.rhs,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::InitializerList(init) => {
            for item in &init.items {
                if let Some(Designator::Field(name)) = &item.designator {
                    unqualified.insert(name.clone());
                }
                if let Some(Designator::Subscript(idx)) = &item.designator {
                    collect_field_reads_impl(
                        ast_context,
                        idx,
                        false,
                        type_map,
                        qualified,
                        unqualified,
                    );
                }
                collect_field_reads_impl(
                    ast_context,
                    &item.value,
                    false,
                    type_map,
                    qualified,
                    unqualified,
                );
            }
        }
        Expression::Call(call) => {
            collect_field_reads_impl(
                ast_context,
                &call.function,
                false,
                type_map,
                qualified,
                unqualified,
            );
            for arg in &call.arguments {
                let Argument::Expression(e) = arg;
                collect_field_reads_impl(ast_context, e, false, type_map, qualified, unqualified);
            }
        }
        Expression::Binary(b) => {
            collect_field_reads_impl(
                ast_context,
                &b.left,
                false,
                type_map,
                qualified,
                unqualified,
            );
            collect_field_reads_impl(
                ast_context,
                &b.right,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Unary(u) => {
            collect_field_reads_impl(
                ast_context,
                &u.operand,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Update(u) => {
            collect_field_reads_impl(
                ast_context,
                &u.operand,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Cast(c) => {
            collect_field_reads_impl(
                ast_context,
                &c.operand,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Conditional(c) => {
            collect_field_reads_impl(
                ast_context,
                &c.condition,
                false,
                type_map,
                qualified,
                unqualified,
            );
            collect_field_reads_impl(
                ast_context,
                &c.then_expr,
                false,
                type_map,
                qualified,
                unqualified,
            );
            collect_field_reads_impl(
                ast_context,
                &c.else_expr,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Subscript(s) => {
            collect_field_reads_impl(
                ast_context,
                &s.array,
                false,
                type_map,
                qualified,
                unqualified,
            );
            collect_field_reads_impl(
                ast_context,
                &s.index,
                false,
                type_map,
                qualified,
                unqualified,
            );
        }
        Expression::Identifier(_)
        | Expression::StringLiteral(_)
        | Expression::NumberLiteral(_)
        | Expression::Null(_)
        | Expression::Boolean(_)
        | Expression::Sizeof(_)
        | Expression::CharLiteral(_)
        | Expression::Comment(_)
        | Expression::OffsetOf(_)
        | Expression::Generic(_) => {}
    }
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
        for (&func_name, defs) in func_defs {
            if func_refs.contains(func_name) {
                continue;
            }
            for (def_path, is_static, location) in defs {
                if *is_static {
                    violations.push(self.violation_at(
                        def_path,
                        location,
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
                        violations.push(self.violation_at(
                            decl_path,
                            decl_location,
                            format!(
                                "Internal function '{}' is never used (declared in private header)",
                                func_name
                            ),
                        ));
                    }
                }
            }
        }

        for (&func_name, decls) in func_decls {
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
                violations.push(self.violation_at(
                    decl_path,
                    decl_location,
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
        ast_context: &AstContext,
        type_defs: &TypeDefMap,
        type_refs: &HashSet<String>,
        violations: &mut Vec<Violation>,
    ) {
        for (&type_name, defs) in type_defs {
            if ast_context
                .type_aliases()
                .is_referenced(type_name, type_refs)
            {
                continue;
            }
            for (def_path, location) in defs {
                violations.push(self.violation_at(
                    def_path,
                    location,
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
        for (&value_name, defs) in enum_value_defs {
            if func_refs.contains(value_name) {
                continue;
            }
            for (def_path, location) in defs {
                violations.push(self.violation_at(
                    def_path,
                    location,
                    format!("Enum value '{}' is defined but never used", value_name),
                ));
            }
        }
    }

    fn report_field_violations(
        &self,
        ast_context: &AstContext,
        field_defs: &FieldDefMap,
        field_refs_qualified: &HashMap<String, HashSet<String>>,
        field_refs_unqualified: &HashSet<String>,
        violations: &mut Vec<Violation>,
    ) {
        for (&field_name, defs) in field_defs {
            if field_refs_unqualified.contains(field_name) {
                continue;
            }
            for (def_path, location, struct_name) in defs {
                if ast_context.type_aliases().field_is_referenced(
                    struct_name,
                    field_name,
                    field_refs_qualified,
                ) {
                    continue;
                }
                violations.push(self.violation_at(
                    def_path,
                    location,
                    format!("Field '{}' in '{}' is never read", field_name, struct_name),
                ));
            }
        }
    }
}
