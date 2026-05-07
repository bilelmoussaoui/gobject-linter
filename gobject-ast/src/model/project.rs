use std::{collections::HashMap, path::PathBuf};

use serde::Serialize;

use crate::{
    Comment, GObjectType, SourceLocation, Statement, TypeInfo, VariableDecl,
    model::{
        doc::TypeDoc,
        expression::Expression,
        top_level::{FunctionDefItem, TopLevelItem},
        types::EnumInfo,
    },
    top_level::{FunctionDeclItem, PreprocessorDirective, TypeDefItem, TypedefTarget},
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
    pub source: Vec<u8>,
}

impl FileModel {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            top_level_items: Vec::new(),
            source: Vec::new(),
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
                }) => Some((path.as_str(), *is_system, *location)),
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
                a.get_installed_enum_value(&self.source)
                    .is_some_and(|ev| property_names.contains(&ev.as_str()))
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
                            .and_then(|arg| arg.to_source_string(&self.source))
                            .is_some_and(|name| name == sentinel)
                    });
                }
            }

            false
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
        Self::resolve_items(&mut self.top_level_items, &self.source);
    }

    fn resolve_items(items: &mut [TopLevelItem], source: &[u8]) {
        for i in 0..items.len() {
            match &items[i] {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) => {
                    let class_init_name = gt.class_init_function_name();
                    let type_name = gt.type_name.clone();
                    let func_idx = items.iter().position(|item| {
                        matches!(item, TopLevelItem::FunctionDefinition(f) if f.name == class_init_name)
                    });
                    if let Some(j) = func_idx {
                        let func = match &items[j] {
                            TopLevelItem::FunctionDefinition(f) => f,
                            _ => unreachable!(),
                        };

                        let properties = func.find_param_spec_assignments(source);
                        let signals = func.find_signal_registrations(source);
                        if let TopLevelItem::Preprocessor(PreprocessorDirective::GObjectType(gt)) =
                            &mut items[i]
                        {
                            gt.properties = properties;
                            gt.signals = signals;
                        }
                    }

                    let doc = items.iter().find_map(|item| {
                        if let TopLevelItem::Comment(c) = item {
                            let td = TypeDoc::from_comment(c)?;
                            if td.symbol.as_deref() == Some(type_name.as_str()) {
                                return Some(td);
                            }
                        }
                        None
                    });
                    if let Some(doc) = doc
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

        for item in items.iter_mut() {
            match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { body, .. })
                | TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                    body,
                    ..
                }) => {
                    Self::resolve_items(body, source);
                }
                _ => {}
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
}
