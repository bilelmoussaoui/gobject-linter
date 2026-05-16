use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Serialize;

use crate::model::{
    Comment, DeclareKind, DefineKind, EnumInfo, EnumValueDoc, Expression, FunctionDeclItem,
    FunctionDefItem, FunctionDoc, GObjectType, GObjectTypeKind, GType, InterfaceImplementation,
    Parameter, PreprocessorDirective, SourceLocation, Statement, TopLevelItem, TypeDefItem,
    TypeDoc, TypeInfo, TypedefTarget, UnaryOp, VariableDecl,
};

/// The complete project model - a map of files to their content
#[derive(Debug, Clone, Serialize, Default)]
pub struct Project {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub files: HashMap<PathBuf, FileModel>,
}

impl Project {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Get a file's model
    pub fn get_file(&self, path: &PathBuf) -> Option<&FileModel> {
        self.files.get(path)
    }

    /// Check if a function is declared in any header
    pub fn is_function_declared_in_header(&self, name: &str) -> bool {
        for file in self.files.values() {
            if file.path.extension().is_some_and(|ext| ext == "h")
                && file
                    .iter_function_declarations()
                    .any(|decl| decl.name == name)
            {
                return true;
            }
        }
        false
    }

    pub fn iter_all_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.files.iter().map(|(path, file)| (path.as_path(), file))
    }

    pub fn iter_c_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.files
            .iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "c"))
            .map(|(path, file)| (path.as_path(), file))
    }

    pub fn iter_header_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.files
            .iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "h"))
            .map(|(path, file)| (path.as_path(), file))
    }

    pub fn find_func_doc(&self, name: &str) -> Option<&FunctionDoc> {
        self.iter_c_files()
            .flat_map(|(_, f)| f.iter_function_definitions())
            .find(|f| f.name == name)
            .and_then(|f| f.doc.as_ref())
            .or_else(|| {
                self.iter_all_files()
                    .flat_map(|(_, f)| f.iter_function_declarations())
                    .find(|f| f.name == name)
                    .and_then(|f| f.doc.as_ref())
            })
    }

    pub fn find_type_doc(&self, type_name: &str) -> Option<&TypeDoc> {
        self.iter_c_files()
            .flat_map(|(_, f)| f.iter_all_gobject_types())
            .find(|gt| gt.type_name == type_name)
            .and_then(|gt| gt.doc.as_ref())
            .or_else(|| {
                self.iter_header_files()
                    .flat_map(|(_, f)| f.iter_all_gobject_types())
                    .find(|gt| gt.type_name == type_name)
                    .and_then(|gt| gt.doc.as_ref())
            })
    }

    pub fn find_gobject_type_by_gtype(&self, gtype: &GType) -> Option<&GObjectType> {
        let all_types: Vec<_> = self
            .iter_all_files()
            .flat_map(|(_, f)| f.iter_all_gobject_types())
            .collect();

        let declare = all_types
            .iter()
            .copied()
            .find(|gt| gt.type_macro.as_ref() == Some(gtype))?;

        all_types
            .iter()
            .copied()
            .find(|gt| gt.type_name == declare.type_name && gt.kind.is_define())
            .or(Some(declare))
    }

    /// Given a GObjectType and its overridden property names, find the
    /// implemented interface that defines the most of them. Returns `None`
    /// if no interface matches any property, or if there's a tie.
    pub fn find_interface_for_property<'a>(
        &self,
        gobject_type: &'a GObjectType,
        property_name: &str,
    ) -> Option<&'a GType> {
        for iface in &gobject_type.interfaces {
            let Some(iface_type) = self.find_gobject_type_by_gtype(&iface.interface_type) else {
                continue;
            };
            if iface_type
                .properties
                .iter()
                .any(|p| p.property().name == property_name)
            {
                return Some(&iface.interface_type);
            }
        }
        None
    }

    /// Check if a function has export macros (truly public API)
    pub fn is_function_exported(&self, name: &str) -> bool {
        for file in self.files.values() {
            if file
                .iter_function_declarations()
                .any(|decl| decl.name == name && !decl.export_macros.is_empty())
            {
                return true;
            }
        }
        false
    }
}

/// Resolved context for a property enum: the owning GObjectType, its class_init
/// function, and the get/set property function names.
pub struct PropertyEnumContext<'a> {
    pub gobject_type: &'a GObjectType,
    pub class_init: &'a FunctionDefItem,
    pub class_type_info: Option<&'a TypeInfo>,
    pub get_property_func: Option<&'a str>,
    pub set_property_func: Option<&'a str>,
}

/// Model of a single file (header or C file)
#[derive(Debug, Clone, Serialize)]
pub struct FileModel {
    pub path: PathBuf,
    /// Top-level items in source order (preserves structure like #ifdef blocks)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub top_level_items: Vec<TopLevelItem>,
    /// The raw source code of this file - available for detailed pattern
    /// matching
    #[serde(skip)]
    pub source: Arc<Vec<u8>>,
}

impl FileModel {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            top_level_items: Vec::new(),
            source: Arc::new(Vec::new()),
        }
    }

    /// Iterate through all includes in the file (including those in #ifdef
    /// blocks)
    pub fn iter_all_includes(&self) -> impl Iterator<Item = (&str, bool, SourceLocation)> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Include {
                    path,
                    is_system,
                    location,
                }) => Some((path.as_str(), *is_system, location.clone())),
                _ => None,
            })
    }

    /// Iterate through all function definitions in the file (including those in
    /// #ifdef blocks)
    pub fn iter_function_definitions(&self) -> impl Iterator<Item = &FunctionDefItem> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::FunctionDefinition(func) => Some(func),
                _ => None,
            })
    }

    /// Iterate through class_init functions
    pub fn iter_class_init_functions(&self) -> impl Iterator<Item = &FunctionDefItem> + '_ {
        self.iter_function_definitions()
            .filter(|f| f.name.ends_with("_class_init"))
    }

    /// Iterate through all function declarations in the file (including those
    /// in #ifdef blocks)
    pub fn iter_function_declarations(&self) -> impl Iterator<Item = &FunctionDeclItem> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::FunctionDeclaration(func) => Some(func),
                _ => None,
            })
    }

    /// Iterate through all functions (both declarations and definitions),
    /// returning function names
    pub fn iter_all_function_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::FunctionDefinition(func) => Some(func.name.as_str()),
                TopLevelItem::FunctionDeclaration(func) => Some(func.name.as_str()),
                _ => None,
            })
    }

    /// Iterate through all GObject type declarations (including those in #ifdef
    /// blocks)
    pub fn iter_all_gobject_types(&self) -> impl Iterator<Item = &GObjectType> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gobject_type)) => {
                    Some(gobject_type.as_ref())
                }
                _ => None,
            })
    }

    /// Find the class or interface struct for a given `GObjectType`.
    /// Returns `None` for final types (no class struct) and when the struct
    /// was not found in this file.
    pub fn find_class_struct_for(&self, gobject_type: &GObjectType) -> Option<&TypeDefItem> {
        let name = gobject_type.class_struct_name()?;
        self.iter_class_structs().find(|td| {
            if let TypeDefItem::Struct { name: n, .. } = td {
                n.trim_start_matches('_') == name
            } else {
                false
            }
        })
    }

    /// Iterate through all class structs (structs ending with `Class` or
    /// `Interface` that have at least one vfunc).
    pub fn iter_class_structs(&self) -> impl Iterator<Item = &TypeDefItem> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::TypeDefinition(td @ TypeDefItem::Struct { vfuncs, .. })
                    if !vfuncs.is_empty() =>
                {
                    Some(td)
                }
                _ => None,
            })
    }

    /// Iterate through all standalone comments (including those in #ifdef
    /// blocks)
    pub fn iter_comments(&self) -> impl Iterator<Item = &Comment> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::Comment(c) => Some(c),
                _ => None,
            })
    }

    /// Iterate through all enum definitions (including those in #ifdef blocks)
    pub fn iter_all_enums(&self) -> impl Iterator<Item = &EnumInfo> + '_ {
        self.iter_items_recursive(&self.top_level_items)
            .filter_map(|item| match item {
                TopLevelItem::TypeDefinition(TypeDefItem::Enum(enum_info)) => {
                    Some(enum_info.as_ref())
                }
                _ => None,
            })
    }

    /// Iterate through property enums (enums that appear to define GObject
    /// properties) Filters for enums where first member starts with PROP_
    /// or ends with _PROP_0
    pub fn iter_property_enums(&self) -> impl Iterator<Item = &EnumInfo> + '_ {
        self.iter_all_enums().filter(|e| e.is_property_enum())
    }

    /// Find the GObjectType whose class_init installs signals from the given
    /// signal enum.
    pub fn find_gobject_type_for_signal_enum(&self, enum_info: &EnumInfo) -> Option<&GObjectType> {
        let signal_names: Vec<&str> = enum_info
            .values
            .iter()
            .filter(|v| !v.is_signal_last())
            .map(|v| v.name.as_str())
            .collect();

        let n_signals_name = enum_info
            .values
            .last()
            .filter(|v| v.is_signal_last())
            .map(|v| v.name.as_str());

        let arrays = self.find_typed_arrays("guint", false, n_signals_name);
        let array_names: Vec<&str> = arrays.iter().map(|d| d.name.as_str()).collect();

        self.iter_all_gobject_types().find(|gt| {
            let class_init_name = gt.class_init_function_name();
            let Some(func) = self
                .iter_function_definitions()
                .find(|f| f.name == class_init_name)
            else {
                return false;
            };

            // Check if class_init assigns to the signal array
            if !array_names.is_empty() {
                let uses_array = func
                    .body_statements
                    .iter()
                    .flat_map(Statement::iter_assignments)
                    .any(|a| {
                        matches!(&*a.lhs, Expression::Subscript(sub)
                            if matches!(&*sub.array, Expression::Identifier(id)
                                if array_names.contains(&id.name.as_str())))
                    });
                if uses_array {
                    return true;
                }
            }

            // Check if class_init uses signal enum values in subscript assignments
            // with g_signal_new
            func.body_statements
                .iter()
                .flat_map(Statement::iter_assignments)
                .any(|a| {
                    if let Expression::Subscript(sub) = &*a.lhs
                        && let Expression::Identifier(index_id) = &*sub.index
                        && signal_names.contains(&index_id.name.as_str())
                        && let Expression::Call(call) = &*a.rhs
                        && call.function_contains("g_signal_new")
                    {
                        true
                    } else {
                        false
                    }
                })
        })
    }

    /// Find array declarations of a specific type, optionally filtered by
    /// sentinel
    ///
    /// If sentinel_name is None, returns ALL arrays of that type.
    ///
    /// Examples:
    /// - `find_typed_arrays("GParamSpec", true, Some("N_PROPS"))` finds
    ///   `GParamSpec *props[N_PROPS]`
    /// - `find_typed_arrays("GParamSpec", true, None)` finds ALL `GParamSpec
    ///   *[]` arrays
    /// - `find_typed_arrays("guint", false, Some("N_SIGNALS"))` finds `guint
    ///   signals[N_SIGNALS]`
    pub fn find_typed_arrays(
        &self,
        base_type: &str,
        is_pointer: bool,
        sentinel_name: Option<&str>,
    ) -> Vec<&VariableDecl> {
        self.iter_all_items()
            .filter_map(|item| {
                let TopLevelItem::Declaration(decl) = item else {
                    return None;
                };
                if !decl.type_info.is_base_type(base_type)
                    || decl.type_info.is_pointer() != is_pointer
                {
                    return None;
                }
                let matches = match &decl.array_size {
                    Some(Expression::Identifier(size_id)) => {
                        sentinel_name.is_none_or(|s| size_id.name == s)
                    }
                    Some(Expression::Binary(_)) => sentinel_name.is_none(),
                    Some(_) => sentinel_name.is_none(),
                    None => false,
                };
                matches.then_some(decl.as_ref())
            })
            .collect()
    }

    /// Find the GObjectType whose properties match the given property enum.
    pub fn find_gobject_type_for_property_enum(
        &self,
        enum_info: &EnumInfo,
    ) -> Option<&GObjectType> {
        let property_names: Vec<&str> = enum_info
            .values
            .iter()
            .filter(|v| !v.is_prop_0() && !v.is_prop_last())
            .map(|v| v.name.as_str())
            .collect();

        let n_props_name = enum_info
            .values
            .last()
            .filter(|v| v.is_prop_last())
            .map(|v| v.name.as_str());

        self.iter_all_gobject_types().find(|gt| {
            // Match by property enum values in assignments
            let has_matching_property = gt.properties.iter().any(|a| {
                a.get_installed_enum_value()
                    .is_some_and(|ev| property_names.contains(&ev))
            });
            if has_matching_property {
                return true;
            }

            // Match by N_PROPS sentinel used in GParamSpec arrays referenced
            // from this GObjectType's class_init
            if let Some(sentinel) = n_props_name {
                let class_init_name = gt.class_init_function_name();
                if let Some(func) = self
                    .iter_function_definitions()
                    .find(|f| f.name == class_init_name)
                {
                    let install_calls = func.find_install_properties_calls();
                    return install_calls.iter().any(|call| {
                        call.get_arg(1)
                            .and_then(|arg| arg.location().as_str())
                            .is_some_and(|name| name == sentinel)
                    });
                }
            }

            false
        })
    }

    pub fn resolve_property_enum_context(
        &self,
        enum_info: &EnumInfo,
    ) -> Option<PropertyEnumContext<'_>> {
        let gobject_type = self.find_gobject_type_for_property_enum(enum_info)?;
        let class_init_name = gobject_type.class_init_function_name();
        let class_init = self
            .iter_function_definitions()
            .find(|f| f.name == class_init_name)?;

        let class_type_info = class_init.parameters.first().and_then(|p| {
            if let Parameter::Regular { type_info, .. } = p {
                Some(type_info)
            } else {
                None
            }
        });

        let mut get_property_func = None;
        let mut set_property_func = None;

        for assignment in class_init
            .body_statements
            .iter()
            .flat_map(Statement::iter_assignments)
        {
            if let Expression::FieldAccess(field) = &*assignment.lhs
                && let Expression::Identifier(ident) = assignment.rhs.as_ref()
            {
                match field.field.as_str() {
                    "get_property" => get_property_func = Some(ident.name.as_str()),
                    "set_property" => set_property_func = Some(ident.name.as_str()),
                    _ => {}
                }
            }
        }

        Some(PropertyEnumContext {
            gobject_type,
            class_init,
            class_type_info,
            get_property_func,
            set_property_func,
        })
    }

    /// Iterate through all top-level items recursively (including items inside
    /// `#ifdef`/`#if` and `G_BEGIN_DECLS` blocks). Conditional container items
    /// themselves are also yielded before their children.
    pub fn iter_all_items(&self) -> impl Iterator<Item = &TopLevelItem> + '_ {
        self.iter_items_recursive(&self.top_level_items)
    }

    /// Iterate typedef forward-alias declarations of the form
    /// `typedef [struct|union] _Foo Foo` (i.e., typedefs that have no inline
    /// struct body). Yields `(typedef_name, target_TypeInfo)` so callers can
    /// inspect `target_type.base_type`, `.is_struct`, and `.is_union`.
    pub fn iter_typedef_pairs(&self) -> impl Iterator<Item = (&str, &TypeInfo)> + '_ {
        self.iter_all_items().filter_map(|item| match item {
            TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
                name,
                target: TypedefTarget::Type(target_type),
                struct_fields,
                ..
            }) if struct_fields.is_empty() && !target_type.base_type.is_empty() => {
                Some((name.as_str(), target_type))
            }
            _ => None,
        })
    }

    /// Populate `properties` and `signals` on each `GObjectType` by finding
    /// the matching `*_class_init` function and extracting param_spec
    /// assignments and signal registrations from it.
    pub fn resolve_gobject_types(&mut self) {
        self.extract_manual_declare_types();
        Self::extract_manual_gobject_types(&mut self.top_level_items);
        Self::resolve_items(&mut self.top_level_items);
    }

    /// Scan for manual `#define TYPE_MACRO (prefix_get_type ())` patterns
    /// and synthesize Declare-style `GObjectType` entries. This covers headers
    /// that don't use `G_DECLARE_*` macros.
    fn extract_manual_declare_types(&mut self) {
        let existing_macros: Vec<GType> = self
            .iter_all_items()
            .filter_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) => {
                    gt.type_macro.clone()
                }
                _ => None,
            })
            .collect();

        // Collect all defines for cross-referencing
        struct DefineInfo {
            name: String,
            value: String,
            location: SourceLocation,
        }
        let defines: Vec<DefineInfo> = self
            .iter_all_items()
            .filter_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                    name,
                    value: Some(value),
                    location,
                }) => Some(DefineInfo {
                    name: name.clone(),
                    value: value.clone(),
                    location: location.clone(),
                }),
                _ => None,
            })
            .collect();

        let mut new_types = Vec::new();

        for define in &defines {
            // Match: #define XXX_TYPE_YYY (zzz_get_type ())
            if !define.name.contains("_TYPE_") {
                continue;
            }
            let gtype = GType::Identifier(define.name.clone());
            if existing_macros.contains(&gtype) {
                continue;
            }

            // Extract function prefix from the value
            let trimmed = define.value.trim().trim_matches(|c| c == '(' || c == ')');
            let Some(func_name) = trimmed.split_whitespace().next() else {
                continue;
            };
            let Some(function_prefix) = func_name.strip_suffix("_get_type") else {
                continue;
            };

            // Find the cast macro to extract the type_name:
            // #define XXX_YYY(obj) (G_TYPE_CHECK_INSTANCE_CAST (..., TypeName))
            // The cast macro name is the type macro without _TYPE
            let Some((module_prefix, type_suffix)) = define.name.split_once("_TYPE_") else {
                continue;
            };
            let cast_macro_name = format!("{}_{}", module_prefix, type_suffix);

            let type_name = defines
                .iter()
                .find(|d| d.name == cast_macro_name)
                .and_then(|d| {
                    // Last identifier before the closing paren is the type name
                    // e.g. (G_TYPE_CHECK_INSTANCE_CAST ((obj), GTK_TYPE_APP_CHOOSER,
                    // GtkAppChooser))
                    d.value
                        .rsplit(',')
                        .next()
                        .map(|s| s.trim().trim_end_matches(')').trim().to_owned())
                });

            let Some(type_name) = type_name else {
                continue;
            };
            if type_name.is_empty() || !type_name.chars().next().unwrap().is_uppercase() {
                continue;
            }

            // Check this type_name isn't already registered
            let already_exists = self.iter_all_items().any(|item| {
                matches!(
                    item,
                    TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt))
                    if gt.type_name == type_name
                )
            });
            if already_exists {
                continue;
            }

            new_types.push(TopLevelItem::Preprocessor(
                PreprocessorDirective::GObjectType(Box::new(GObjectType {
                    type_name,
                    type_macro: Some(gtype),
                    function_prefix: function_prefix.to_owned(),
                    parent_type: None,
                    flags: None,
                    kind: GObjectTypeKind::Declare {
                        kind: DeclareKind::Derivable,
                        module_prefix: module_prefix.to_owned(),
                        type_prefix: type_suffix.to_owned(),
                    },
                    interfaces: Vec::new(),
                    has_private: false,
                    manually_registered: true,
                    code_block_statements: Vec::new(),
                    export_macros: Vec::new(),
                    doc: None,
                    properties: Vec::new(),
                    signals: Vec::new(),
                    location: define.location.clone(),
                })),
            ));
        }

        self.top_level_items.extend(new_types);
    }

    /// Scan for `*_get_type` functions that return `GType` and contain a
    /// `g_type_register_static` or `g_type_register_static_simple` call.
    /// Synthesize `GObjectType` entries for manual type registrations that
    /// don't use `G_DEFINE_*` macros.
    fn extract_manual_gobject_types(items: &mut Vec<TopLevelItem>) {
        let existing_type_names: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) => {
                    Some(gt.type_name.clone())
                }
                _ => None,
            })
            .collect();

        let mut new_types = Vec::new();

        for item in items.iter() {
            let TopLevelItem::FunctionDefinition(func) = item else {
                continue;
            };
            if !func.name.ends_with("_get_type") || func.return_type.base_type != "GType" {
                continue;
            }

            let function_prefix = func.name.strip_suffix("_get_type").unwrap();

            let register_call = func.find_calls_matching(|name| {
                name == "g_type_register_static" || name == "g_type_register_static_simple"
            });
            let Some(call) = register_call.first() else {
                continue;
            };

            // arg[0] = parent_type, arg[1] = "TypeName"
            let Some(parent_expr) = call.get_arg(0) else {
                continue;
            };
            let parent_type = match parent_expr {
                Expression::Identifier(id) => id.name.clone(),
                _ => continue,
            };

            let Some(type_name) = call.extract_string_from_arg(1) else {
                continue;
            };
            let type_name = type_name.trim_matches('"').to_owned();

            if existing_type_names.contains(&type_name) {
                continue;
            }

            let flags = call
                .get_arg(if call.arguments.len() > 4 { 6 } else { 3 })
                .and_then(|e| match e {
                    Expression::Identifier(id) => Some(id.name.as_str()),
                    _ => None,
                });

            let is_interface = parent_type == "G_TYPE_INTERFACE";
            let is_abstract = flags.is_some_and(|f| f.contains("ABSTRACT"));

            let has_private = !func.find_calls(&["g_type_add_instance_private"]).is_empty();

            let kind = if is_interface {
                GObjectTypeKind::Define(DefineKind::Interface)
            } else if is_abstract {
                GObjectTypeKind::Define(DefineKind::AbstractType)
            } else {
                GObjectTypeKind::Define(DefineKind::Type)
            };

            let iface_calls = func.find_calls(&["g_type_add_interface_static"]);
            let mut interfaces = Vec::new();
            for iface_call in &iface_calls {
                let Some(iface_type_expr) = iface_call.get_arg(1) else {
                    continue;
                };
                let Expression::Identifier(iface_id) = iface_type_expr else {
                    continue;
                };
                let interface_type = GType::Identifier(iface_id.name.clone());

                // arg[2] is &info_var — extract var name, find its
                // GInterfaceInfo decl, get init func from initializer[0]
                let init_function = iface_call
                    .get_arg(2)
                    .and_then(|e| match e {
                        Expression::Unary(u) if u.operator == UnaryOp::AddressOf => {
                            match u.operand.as_ref() {
                                Expression::Identifier(id) => Some(id.name.as_str()),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                    .and_then(|var_name| {
                        Self::find_init_func_from_iface_info(&func.body_statements, var_name)
                    });

                interfaces.push(InterfaceImplementation {
                    interface_type,
                    init_function,
                });
            }

            new_types.push(TopLevelItem::Preprocessor(
                PreprocessorDirective::GObjectType(Box::new(GObjectType {
                    type_name,
                    type_macro: None,
                    function_prefix: function_prefix.to_owned(),
                    parent_type: Some(parent_type),
                    flags: flags.map(std::borrow::ToOwned::to_owned),
                    kind,
                    interfaces,
                    has_private,
                    manually_registered: true,
                    code_block_statements: Vec::new(),
                    export_macros: Vec::new(),
                    doc: None,
                    properties: Vec::new(),
                    signals: Vec::new(),
                    location: func.location.clone(),
                })),
            ));
        }

        items.extend(new_types);
    }

    /// Find the init function name from a `GInterfaceInfo` variable
    /// declaration. The init function is the first non-comment item in the
    /// initializer list, possibly wrapped in a cast like
    /// `(GInterfaceInitFunc) func_name`.
    fn find_init_func_from_iface_info(stmts: &[Statement], var_name: &str) -> Option<String> {
        for stmt in stmts {
            match stmt {
                Statement::Declaration(decl) if decl.name == var_name => {
                    let Expression::InitializerList(init) = decl.initializer.as_ref()? else {
                        return None;
                    };
                    let first = init
                        .items
                        .iter()
                        .find(|item| !matches!(&*item.value, Expression::Comment(_)))?;
                    return Some(Self::unwrap_cast_to_identifier(&first.value)?.to_owned());
                }
                Statement::If(if_stmt) => {
                    if let Some(v) =
                        Self::find_init_func_from_iface_info(&if_stmt.then_body, var_name)
                    {
                        return Some(v);
                    }
                    if let Some(else_body) = &if_stmt.else_body
                        && let Some(v) = Self::find_init_func_from_iface_info(else_body, var_name)
                    {
                        return Some(v);
                    }
                }
                Statement::Compound(c) => {
                    if let Some(v) = Self::find_init_func_from_iface_info(&c.statements, var_name) {
                        return Some(v);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Unwrap `(SomeType) identifier` casts to get the inner identifier name.
    fn unwrap_cast_to_identifier(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Identifier(id) => Some(&id.name),
            Expression::Cast(c) => Self::unwrap_cast_to_identifier(&c.operand),
            _ => None,
        }
    }

    fn resolve_items(items: &mut [TopLevelItem]) {
        for i in 0..items.len() {
            match &items[i] {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) => {
                    let type_name = gt.type_name.clone();

                    let init_names = if gt.is_interface() {
                        vec![
                            gt.default_init_function_name(),
                            gt.class_init_function_name(),
                        ]
                    } else {
                        vec![gt.class_init_function_name()]
                    };

                    let func_idx = items.iter().position(|item| {
                        matches!(item, TopLevelItem::FunctionDefinition(f) if init_names.contains(&f.name))
                    });
                    if let Some(j) = func_idx {
                        let func = match &items[j] {
                            TopLevelItem::FunctionDefinition(f) => f,
                            _ => unreachable!(),
                        };

                        let properties = func.find_param_spec_assignments(&type_name);
                        let signals = func.find_signal_registrations(&type_name);
                        if let TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) =
                            &mut items[i]
                        {
                            gt.properties = properties;
                            gt.signals = signals;
                        }
                    }

                    if let Some(doc) = Self::find_type_doc_in_comments(items, &type_name)
                        && let TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) =
                            &mut items[i]
                    {
                        gt.doc = Some(doc);
                    }
                }
                TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { .. })
                | TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock { .. }) => {}
                _ => {}
            }
        }

        Self::resolve_type_docs(items);

        for item in items.iter_mut() {
            match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { body, .. })
                | TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                    body,
                    ..
                }) => {
                    Self::resolve_items(body);
                }
                _ => {}
            }
        }
    }

    fn find_type_doc_in_comments(items: &[TopLevelItem], type_name: &str) -> Option<TypeDoc> {
        items.iter().find_map(|item| {
            if let TopLevelItem::Comment(c) = item {
                let doc = TypeDoc::from_comment(c)?;
                let sym = doc.symbol.as_deref()?;
                if sym == type_name {
                    return Some(doc);
                }
            }
            None
        })
    }

    fn resolve_type_docs(items: &mut [TopLevelItem]) {
        for i in 0..items.len() {
            let name = match &items[i] {
                TopLevelItem::TypeDefinition(
                    TypeDefItem::Struct {
                        name, doc: None, ..
                    }
                    | TypeDefItem::Typedef {
                        name, doc: None, ..
                    },
                ) => Some(name.trim_start_matches('_').to_owned()),
                TopLevelItem::TypeDefinition(TypeDefItem::Enum(e)) if e.doc.is_none() => {
                    e.name.clone()
                }
                _ => None,
            };
            if let Some(name) = name
                && let Some(doc) = Self::find_type_doc_in_comments(items, &name)
            {
                match &mut items[i] {
                    TopLevelItem::TypeDefinition(
                        TypeDefItem::Struct { doc: d, .. } | TypeDefItem::Typedef { doc: d, .. },
                    ) => {
                        *d = Some(doc);
                    }
                    TopLevelItem::TypeDefinition(TypeDefItem::Enum(e)) => {
                        e.doc = Some(doc);
                    }
                    _ => {}
                }
            }
        }

        Self::resolve_enum_value_docs(items);
    }

    fn resolve_enum_value_docs(items: &mut [TopLevelItem]) {
        for i in 0..items.len() {
            let TopLevelItem::TypeDefinition(TypeDefItem::Enum(e)) = &mut items[i] else {
                continue;
            };

            // Extract inline @VALUE: entries from the parent enum's doc comment
            let enum_name = match &e.name {
                Some(n) => n.clone(),
                None => continue,
            };
            let inline_docs: Vec<(String, EnumValueDoc)> = items
                .iter()
                .find_map(|item| {
                    if let TopLevelItem::Comment(c) = item {
                        let doc = TypeDoc::from_comment(c)?;
                        if doc.symbol.as_deref() == Some(&enum_name) {
                            return Some(EnumValueDoc::extract_inline_from_comment(c));
                        }
                    }
                    None
                })
                .unwrap_or_default();

            // Re-borrow mutably after the immutable iteration
            let TopLevelItem::TypeDefinition(TypeDefItem::Enum(e)) = &mut items[i] else {
                continue;
            };

            for value in &mut e.values {
                // First try inline @VALUE: from parent doc
                if let Some((_, doc)) = inline_docs.iter().find(|(n, _)| *n == value.name) {
                    value.doc = Some(doc.clone());
                }
            }

            // Then for values still without docs, look for standalone comments
            let missing: Vec<String> = e
                .values
                .iter()
                .filter(|v| v.doc.is_none())
                .map(|v| v.name.clone())
                .collect();

            let standalone_docs: Vec<(String, EnumValueDoc)> = missing
                .iter()
                .filter_map(|name| {
                    let doc = items.iter().find_map(|item| {
                        if let TopLevelItem::Comment(c) = item {
                            let doc = EnumValueDoc::from_comment(c)?;
                            if doc.symbol.as_deref() == Some(name.as_str()) {
                                return Some(doc);
                            }
                        }
                        None
                    })?;
                    Some((name.clone(), doc))
                })
                .collect();

            let TopLevelItem::TypeDefinition(TypeDefItem::Enum(e)) = &mut items[i] else {
                continue;
            };
            for (name, doc) in standalone_docs {
                if let Some(value) = e.values.iter_mut().find(|v| v.name == name) {
                    value.doc = Some(doc);
                }
            }
        }
    }

    /// Recursively iterate through all items (including those in #ifdef blocks)
    fn iter_items_recursive<'a>(
        &'a self,
        items: &'a [TopLevelItem],
    ) -> Box<dyn Iterator<Item = &'a TopLevelItem> + 'a> {
        Box::new(items.iter().flat_map(move |item| match item {
            TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { body, .. })
            | TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                body, ..
            }) => Box::new(std::iter::once(item).chain(self.iter_items_recursive(body)))
                as Box<dyn Iterator<Item = &'a TopLevelItem>>,
            _ => Box::new(std::iter::once(item)) as Box<dyn Iterator<Item = &'a TopLevelItem>>,
        }))
    }

    fn iter_items_with_parent<'a>(
        &'a self,
        items: &'a [TopLevelItem],
        parent: Option<&'a TopLevelItem>,
    ) -> Box<dyn Iterator<Item = (&'a TopLevelItem, Option<&'a TopLevelItem>)> + 'a> {
        Box::new(items.iter().flat_map(move |item| {
            match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { body, .. })
                | TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                    body,
                    ..
                }) => Box::new(
                    std::iter::once((item, parent))
                        .chain(self.iter_items_with_parent(body, Some(item))),
                )
                    as Box<dyn Iterator<Item = (&'a TopLevelItem, Option<&'a TopLevelItem>)>>,
                _ => Box::new(std::iter::once((item, parent)))
                    as Box<dyn Iterator<Item = (&'a TopLevelItem, Option<&'a TopLevelItem>)>>,
            }
        }))
    }

    pub fn iter_all_items_with_parent(
        &self,
    ) -> impl Iterator<Item = (&TopLevelItem, Option<&TopLevelItem>)> + '_ {
        self.iter_items_with_parent(&self.top_level_items, None)
    }
}
