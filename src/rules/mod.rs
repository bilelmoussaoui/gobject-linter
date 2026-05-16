use std::path::PathBuf;

use gobject_ast::model::{
    EnumInfo, FileModel, FunctionDeclItem, FunctionDefItem, GObjectType, SourceLocation,
};
use serde::{Deserialize, Serialize};

use crate::{
    ast_context::AstContext,
    config::{Config, RuleLevel},
};

/// Rule category (similar to Clippy's lint categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
pub enum Category {
    /// Code that is outright wrong or very useless
    Correctness,
    /// Code that is most likely wrong or useless
    Suspicious,
    /// Code that should be written in a more idiomatic way
    Style,
    /// Code that does something simple but in a complex way
    Complexity,
    /// Code that can be written to run faster
    Perf,
    /// Lints which are rather strict or have occasional false positives
    Pedantic,
    /// Lints which prevent the use of language/library features
    Restriction,
    /// Code that may cause portability issues across platforms/compilers
    Portability,
    /// GObject Introspection annotation issues
    Introspection,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Style => "style",
            Self::Complexity => "complexity",
            Self::Perf => "perf",
            Self::Pedantic => "pedantic",
            Self::Restriction => "restriction",
            Self::Portability => "portability",
            Self::Introspection => "introspection",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Correctness => write!(f, "Correctness"),
            Self::Suspicious => write!(f, "Suspicious"),
            Self::Style => write!(f, "Style"),
            Self::Complexity => write!(f, "Complexity"),
            Self::Perf => write!(f, "Performance"),
            Self::Pedantic => write!(f, "Pedantic"),
            Self::Restriction => write!(f, "Restriction"),
            Self::Portability => write!(f, "Portability"),
            Self::Introspection => write!(f, "Introspection"),
        }
    }
}

/// Represents an automated fix for a violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Byte offset where the fix starts
    pub start_byte: usize,
    /// Byte offset where the fix ends (exclusive)
    pub end_byte: usize,
    /// Replacement text (`None` = pure deletion)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

impl Fix {
    /// Create a fix that replaces a byte range with new text
    pub fn new(start_byte: usize, end_byte: usize, replacement: impl Into<String>) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: Some(replacement.into()),
        }
    }

    /// Create a fix that deletes a byte range
    pub fn delete(start_byte: usize, end_byte: usize) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: None,
        }
    }

    /// Delete an entire line (including indentation and newline)
    pub fn delete_line(location: &SourceLocation) -> Self {
        let (start, end) = location.find_line_range();
        Self::delete(start, end)
    }

    /// Delete a line and any preceding blank line
    pub fn delete_line_and_leading_blank(location: &SourceLocation) -> Self {
        let (start, end) = location.find_line_bounds();
        Self::delete(start, end)
    }

    /// Delete a line and any following blank line
    pub fn delete_line_and_trailing_blank(location: &SourceLocation) -> Self {
        let (start, end) = location.find_line_bounds_with_following_blank();
        Self::delete(start, end)
    }
}

/// Describes a simple function rename: find calls to `from`, suggest `to`.
pub struct FunctionRename {
    /// Function name to search for
    pub from: &'static str,
    /// Replacement function name, or `None` for warn-only (no fix)
    pub to: Option<&'static str>,
    /// Violation message
    pub message: &'static str,
}

/// Configuration option metadata for a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOption {
    /// Option name (e.g., "config_header")
    pub name: &'static str,
    /// Option type (e.g., "string", "array<string>", "boolean")
    pub option_type: &'static str,
    /// Default value as a string representation (e.g., "\"config.h\"", "[]")
    pub default_value: &'static str,
    /// Example value for documentation (e.g., "[\"cairo_*\", \"Pango*\"]")
    pub example_value: &'static str,
    /// Description of what this option does
    pub description: &'static str,
}

pub mod dead_code;
pub mod deprecated_add_private;
pub mod g_error_init;
pub mod g_error_leak;
pub mod g_object_virtual_methods_chain_up;
pub mod g_param_spec_null_nick_blurb;
pub mod g_param_spec_static_strings;
pub mod g_source_id_not_stored;
pub mod g_task_source_tag;
pub mod gi_missing_since;
pub mod gi_not_bindings_friendly;
pub mod include_order;
pub mod inconsistent_function_signature;
pub mod matching_declare_define;
pub mod missing_autoptr_cleanup;
pub mod missing_export_macro;
pub mod missing_g_begin_decls;
pub mod missing_implementation;
pub mod no_g_auto_macros;
pub mod property_canonical_name;
pub mod property_enum_convention;
pub mod property_enum_coverage;
pub mod property_switch_exhaustiveness;
pub mod signal_canonical_name;
pub mod signal_enum_coverage;
pub mod strcmp_explicit_comparison;
pub mod type_style;
pub mod unnecessary_null_check;
pub mod untranslated_string;
pub mod use_auto_cleanup;
pub mod use_clear_functions;
pub mod use_explicit_default_flags;
pub mod use_g_ascii_functions;
pub mod use_g_bytes_unref_to_data;
pub mod use_g_file_load_bytes;
pub mod use_g_gnuc_flag_enum;
pub mod use_g_new;
pub mod use_g_object_class_install_properties;
pub mod use_g_object_new_with_properties;
pub mod use_g_object_notify_by_pspec;
pub mod use_g_set_object;
pub mod use_g_set_str;
pub mod use_g_settings_typed;
pub mod use_g_source_constants;
pub mod use_g_source_once;
pub mod use_g_steal_pointer;
pub mod use_g_str_has_prefix_suffix;
pub mod use_g_strcmp0;
pub mod use_g_string_free_and_steal;
pub mod use_g_strlcpy;
pub mod use_g_value_set_static_string;
pub mod use_g_variant_new_typed;
pub mod use_pragma_once;

pub use dead_code::DeadCode;
pub use deprecated_add_private::DeprecatedAddPrivate;
pub use g_error_init::GErrorInit;
pub use g_error_leak::GErrorLeak;
pub use g_object_virtual_methods_chain_up::GObjectVirtualMethodsChainUp;
pub use g_param_spec_null_nick_blurb::GParamSpecNullNickBlurb;
pub use g_param_spec_static_strings::GParamSpecStaticStrings;
pub use g_source_id_not_stored::GSourceIdNotStored;
pub use g_task_source_tag::GTaskSourceTag;
pub use gi_missing_since::GiMissingSince;
pub use gi_not_bindings_friendly::GiNotBindingsFriendly;
pub use include_order::IncludeOrder;
pub use inconsistent_function_signature::InconsistentFunctionSignature;
pub use matching_declare_define::MatchingDeclareDefine;
pub use missing_autoptr_cleanup::MissingAutoptrCleanup;
pub use missing_export_macro::MissingExportMacro;
pub use missing_g_begin_decls::MissingGBeginDecls;
pub use missing_implementation::MissingImplementation;
pub use no_g_auto_macros::NoGAutoMacros;
pub use property_canonical_name::PropertyCanonicalName;
pub use property_enum_convention::PropertyEnumConvention;
pub use property_enum_coverage::PropertyEnumCoverage;
pub use property_switch_exhaustiveness::PropertySwitchExhaustiveness;
pub use signal_canonical_name::SignalCanonicalName;
pub use signal_enum_coverage::SignalEnumCoverage;
pub use strcmp_explicit_comparison::StrcmpExplicitComparison;
pub use type_style::TypeStyle;
pub use unnecessary_null_check::UnnecessaryNullCheck;
pub use untranslated_string::UntranslatedString;
pub use use_auto_cleanup::UseAutoCleanup;
pub use use_clear_functions::UseClearFunctions;
pub use use_explicit_default_flags::UseExplicitDefaultFlags;
pub use use_g_ascii_functions::UseGAsciiFunctions;
pub use use_g_bytes_unref_to_data::UseGBytesUnrefToData;
pub use use_g_file_load_bytes::UseGFileLoadBytes;
pub use use_g_gnuc_flag_enum::UseGGnucFlagEnum;
pub use use_g_new::UseGNew;
pub use use_g_object_class_install_properties::UseGObjectClassInstallProperties;
pub use use_g_object_new_with_properties::UseGObjectNewWithProperties;
pub use use_g_object_notify_by_pspec::UseGObjectNotifyByPspec;
pub use use_g_set_object::UseGSetObject;
pub use use_g_set_str::UseGSetStr;
pub use use_g_settings_typed::UseGSettingsTyped;
pub use use_g_source_constants::UseGSourceConstants;
pub use use_g_source_once::UseGSourceOnce;
pub use use_g_steal_pointer::UseGStealPointer;
pub use use_g_str_has_prefix_suffix::UseGStrHasPrefixSuffix;
pub use use_g_strcmp0::UseGStrcmp0;
pub use use_g_string_free_and_steal::UseGStringFreeAndSteal;
pub use use_g_strlcpy::UseGStrlcpy;
pub use use_g_value_set_static_string::UseGValueSetStaticString;
pub use use_g_variant_new_typed::UseGVariantNewTyped;
pub use use_pragma_once::UsePragmaOnce;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Violation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rule: &'static str,
    pub category: Category,
    pub level: RuleLevel,
    pub snippet: Option<String>,
    /// Rule execution order - higher means more specific/later rules take
    /// precedence
    pub rule_index: usize,
    /// Optional automated fixes (multiple edits can be applied)
    pub fixes: Vec<Fix>,
}

/// Trait that all linting rules must implement
pub trait Rule: Send + Sync {
    /// The unique identifier for this rule (e.g., "missing_implementation")
    fn name(&self) -> &'static str;

    /// Human-readable description of what this rule checks
    fn description(&self) -> &'static str;

    /// Long-form markdown documentation (optional)
    fn long_description(&self) -> Option<&'static str> {
        None
    }

    /// Rule category
    fn category(&self) -> Category;

    /// Whether this rule supports automated fixes via --fix
    fn fixable(&self) -> bool {
        false
    }

    /// Whether this rule requires meson introspection to produce results.
    /// Rules returning true silently skip when no build directory is found.
    fn requires_meson(&self) -> bool {
        false
    }

    /// Configuration options supported by this rule
    fn config_options(&self) -> &'static [ConfigOption] {
        &[]
    }

    /// Minimum GLib version required by this rule.
    /// `None` means no version requirement (compatible with all versions).
    fn min_glib_version(&self) -> Option<(u32, u32)> {
        None
    }

    /// Whether this rule suggests g_auto* macros (disabled when
    /// msvc_compatible=true)
    fn requires_auto_cleanup(&self) -> bool {
        false
    }

    /// Whether this rule is disabled by default (user must explicitly enable
    /// it)
    fn opt_in(&self) -> bool {
        false
    }

    /// Why this rule is opt-in (only meaningful when opt_in() returns true)
    fn opt_in_reason(&self) -> Option<&'static str> {
        None
    }

    /// Check a function implementation (from C files)
    /// Override this to check function bodies and implementations
    #[allow(unused_variables)]
    fn check_func_impl(
        &self,
        ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Default: no-op
    }

    /// Check a function declaration (from header files)
    /// Override this to check function declarations and signatures
    #[allow(unused_variables)]
    fn check_func_decl(
        &self,
        ast_context: &AstContext,
        config: &Config,
        func: &FunctionDeclItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Default: no-op
    }

    /// Check an enum definition
    #[allow(unused_variables)]
    fn check_enum(
        &self,
        ast_context: &AstContext,
        config: &Config,
        enum_info: &EnumInfo,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Default: no-op
    }

    /// Check a GObject type declaration/definition
    /// Override this to check properties, signals, or other GObject-level
    /// concerns
    #[allow(unused_variables)]
    fn check_gobject_type(
        &self,
        ast_context: &AstContext,
        config: &Config,
        gobject_type: &GObjectType,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Default: no-op
    }

    /// Check the AST and add violations to the provided vector
    /// Default implementation calls check_func_impl for C files,
    /// check_func_decl for headers, and check_gobject_type for all files.
    /// Override this if you need custom iteration logic beyond per-item
    /// checking
    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_all_files() {
            let ext = path.extension().and_then(|e| e.to_str());
            if ext == Some("c") {
                for func in file.iter_function_definitions() {
                    self.check_func_impl(ast_context, config, func, file, violations);
                }
            }
            if ext == Some("h") {
                for func in file.iter_function_declarations() {
                    self.check_func_decl(ast_context, config, func, file, violations);
                }
            }
            for gt in file.iter_all_gobject_types() {
                self.check_gobject_type(ast_context, config, gt, file, violations);
            }
            for enum_info in file.iter_all_enums() {
                self.check_enum(ast_context, config, enum_info, file, violations);
            }
        }
    }

    /// Find calls to functions listed in `renames` and emit violations.
    /// When `to` is `Some`, generates a fix that reformats the entire call.
    fn check_function_renames(
        &self,
        func: &FunctionDefItem,
        file: &FileModel,
        config: &Config,
        violations: &mut Vec<Violation>,
        renames: &[FunctionRename],
    ) {
        let names: Vec<&str> = renames.iter().map(|r| r.from).collect();
        for call in func.find_calls(&names) {
            let Some(func_name) = call.function_name_str() else {
                continue;
            };
            let Some(rename) = renames.iter().find(|r| r.from == func_name) else {
                continue;
            };

            if let Some(new_name) = rename.to {
                let args: Vec<&str> = call
                    .arguments
                    .iter()
                    .filter_map(|arg| arg.location().as_str())
                    .collect();
                let replacement = config.style.format_call(new_name, &args);
                let fix = Fix::new(
                    call.location.start_byte,
                    call.location.end_byte,
                    replacement,
                );
                violations.push(self.violation_with_fix_at(
                    &file.path,
                    &call.location,
                    rename.message.to_string(),
                    fix,
                ));
            } else {
                violations.push(self.violation_at(
                    &file.path,
                    &call.location,
                    rename.message.to_string(),
                ));
            }
        }
    }

    /// Helper to create a violation with the rule name automatically filled in
    fn violation(
        &self,
        file: &std::path::Path,
        line: usize,
        column: usize,
        message: String,
    ) -> Violation {
        self.violation_with_fixes(file, line, column, message, vec![])
    }

    /// Helper to create a violation with an automated fix
    fn violation_with_fix(
        &self,
        file: &std::path::Path,
        line: usize,
        column: usize,
        message: String,
        fix: Fix,
    ) -> Violation {
        self.violation_with_fixes(file, line, column, message, vec![fix])
    }

    /// Helper to create a violation with multiple automated fixes
    fn violation_with_fixes(
        &self,
        file: &std::path::Path,
        line: usize,
        column: usize,
        message: String,
        fixes: Vec<Fix>,
    ) -> Violation {
        Violation {
            file: file.to_path_buf(),
            line,
            column,
            message,
            rule: self.name(),
            category: self.category(),
            level: RuleLevel::Error, // Will be overridden by scanner
            snippet: None,
            rule_index: 0,
            fixes,
        }
    }

    fn violation_at(
        &self,
        file: &std::path::Path,
        location: &SourceLocation,
        message: String,
    ) -> Violation {
        self.violation(file, location.line, location.column, message)
    }

    fn violation_with_fix_at(
        &self,
        file: &std::path::Path,
        location: &SourceLocation,
        message: String,
        fix: Fix,
    ) -> Violation {
        self.violation_with_fix(file, location.line, location.column, message, fix)
    }

    fn violation_with_fixes_at(
        &self,
        file: &std::path::Path,
        location: &SourceLocation,
        message: String,
        fixes: Vec<Fix>,
    ) -> Violation {
        self.violation_with_fixes(file, location.line, location.column, message, fixes)
    }
}
