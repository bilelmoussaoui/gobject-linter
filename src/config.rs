use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::ValueEnum;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, de};

use crate::{for_each_rule, rules::*, scanner::RuleName};

#[derive(Default, Debug, Clone, Copy, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    /// Human-readable colorized output (default)
    #[default]
    Text,
    /// JSON format
    Json,
    /// SARIF JSON format for GitHub Code Scanning, VS Code, etc.
    Sarif,
    /// GCC-compatible format for Emacs, Vim, and other tools
    Gcc,
    /// Gitlab specific Code Quality Report
    GitlabCodequality,
}

/// Parse a GLib version string like "2.76" into (major, minor)
pub fn parse_glib_version(version: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    Some((major, minor))
}

pub const CONFIG_FILENAMES: &[&str] = &["gobject-linter.toml", ".gobject-linter.toml"];
pub const LEGACY_CONFIG_FILENAMES: &[&str] = &["goblint.toml"];

pub fn has_config_file(dir: &Path) -> bool {
    CONFIG_FILENAMES
        .iter()
        .chain(LEGACY_CONFIG_FILENAMES)
        .any(|name| dir.join(name).exists())
}

pub fn resolve_config_in_dir(dir: &Path) -> Option<PathBuf> {
    CONFIG_FILENAMES
        .iter()
        .chain(LEGACY_CONFIG_FILENAMES)
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// Resolve which config file to use for CLI invocation.
///
/// When `explicit_config` is `Some`, that exact path is used (returns `Err` if
/// missing). Otherwise, searches `target_dir` then `base_dir` with canonical
/// names first, legacy names last.
pub fn resolve_config_path(
    target_dir: &Path,
    base_dir: &Path,
    explicit_config: Option<&Path>,
) -> Result<PathBuf, PathBuf> {
    if let Some(config) = explicit_config {
        if config.exists() {
            return Ok(config.to_path_buf());
        }
        return Err(config.to_path_buf());
    }

    for names in [CONFIG_FILENAMES, LEGACY_CONFIG_FILENAMES] {
        for dir in [target_dir, base_dir] {
            for name in names {
                let path = dir.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    Ok(base_dir.join(CONFIG_FILENAMES[0]))
}

/// Deserialize GLib version from string to (major, minor) tuple
fn deserialize_glib_version<'de, D>(deserializer: D) -> Result<Option<(u32, u32)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version_str: Option<String> = Option::deserialize(deserializer)?;

    match version_str {
        Some(s) => parse_glib_version(&s).map(Some).ok_or_else(|| {
            de::Error::custom(format!(
                "Invalid GLib version format: '{}'. Expected format: 'major.minor' (e.g., '2.76')",
                s
            ))
        }),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Style {
    #[serde(default = "default_true")]
    pub space_before_paren: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            space_before_paren: true,
        }
    }
}

impl Style {
    pub fn format_call(&self, func: &str, args: &[&str]) -> String {
        let sep = if self.space_before_paren { " (" } else { "(" };
        let mut s =
            String::with_capacity(func.len() + 2 + args.iter().map(|a| a.len() + 2).sum::<usize>());
        s.push_str(func);
        s.push_str(sep);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(arg);
        }
        s.push(')');
        s
    }

    pub fn format_call_stmt(&self, func: &str, args: &[&str]) -> String {
        let mut s = self.format_call(func, args);
        s.push(';');
        s
    }

    pub fn format_addr_call(&self, func: &str, var: &str, extra_args: &[&str]) -> String {
        let sep = if self.space_before_paren { " (" } else { "(" };
        let mut s = String::with_capacity(
            func.len() + 4 + var.len() + extra_args.iter().map(|a| a.len() + 2).sum::<usize>(),
        );
        s.push_str(func);
        s.push_str(sep);
        s.push('&');
        s.push_str(var);
        for arg in extra_args {
            s.push_str(", ");
            s.push_str(arg);
        }
        s.push(')');
        s
    }

    pub fn format_addr_call_stmt(&self, func: &str, var: &str, extra_args: &[&str]) -> String {
        let mut s = self.format_addr_call(func, var, extra_args);
        s.push(';');
        s
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub style: Style,

    #[serde(default)]
    pub rules: RulesConfig,

    #[serde(default)]
    pub ignore: Vec<String>,

    /// Minimum supported GLib version as (major, minor)
    /// Rules requiring newer GLib versions will be automatically disabled
    #[serde(default, deserialize_with = "deserialize_glib_version")]
    pub min_glib_version: Option<(u32, u32)>,

    /// Target MSVC-compatible code
    #[serde(default)]
    pub msvc_compatible: bool,

    /// Output format
    pub format: Option<OutputFormat>,

    /// Editor URL format for clickable links
    /// Available placeholders: {path}, {line}, {column}
    /// Examples:
    ///   VSCode: "vscode://file{path}:{line}:{column}"
    ///   IntelliJ: "idea://open?file={path}&line={line}"
    ///   Sublime: "subl://open?url=file://{path}&line={line}&column={column}"
    pub editor_url: Option<String>,

    /// Build directory for meson introspection (used for dead code analysis)
    /// If not specified, will search for common build directories (build/,
    /// builddir/, _build/)
    pub build_dir: Option<String>,

    /// Default severity level for all non-opt-in rules.
    /// Opt-in rules always default to "ignore" regardless of this setting.
    /// Per-rule `level` settings override this.
    pub default_level: Option<RuleLevel>,
}

/// Rule severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleLevel {
    /// Report as error and exit with failure code
    Error,
    /// Report as warning but don't fail
    Warn,
    /// Disabled/ignored
    Ignore,
}

impl RuleLevel {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Ignore)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    pub fn is_warn(&self) -> bool {
        matches!(self, Self::Warn)
    }
}

/// Per-rule configuration
#[derive(Debug, Default, Clone)]
pub struct RuleConfig {
    /// Explicitly configured level, or None if the user never set it.
    /// None is resolved at runtime: opt-in rules default to Ignore, others to
    /// Warn.
    pub level: Option<RuleLevel>,
    pub ignore: Vec<String>,
    /// Rule-specific options (e.g., config_header for include_order)
    pub options: HashMap<String, toml::Value>,
}

impl<'de> Deserialize<'de> for RuleConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RuleConfigVisitor;

        impl<'de> serde::de::Visitor<'de> for RuleConfigVisitor {
            type Value = RuleConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("a boolean, \"error\"/\"warn\"/\"ignore\", or a RuleConfig struct")
            }

            fn visit_bool<E>(self, value: bool) -> Result<RuleConfig, E>
            where
                E: de::Error,
            {
                Ok(RuleConfig {
                    level: Some(if value {
                        RuleLevel::Error
                    } else {
                        RuleLevel::Ignore
                    }),
                    ignore: Vec::new(),
                    options: HashMap::new(),
                })
            }

            fn visit_str<E>(self, value: &str) -> Result<RuleConfig, E>
            where
                E: de::Error,
            {
                let level = match value {
                    "error" => RuleLevel::Error,
                    "warn" => RuleLevel::Warn,
                    "ignore" => RuleLevel::Ignore,
                    _ => {
                        return Err(de::Error::unknown_variant(
                            value,
                            &["error", "warn", "ignore"],
                        ));
                    }
                };
                Ok(RuleConfig {
                    level: Some(level),
                    ignore: Vec::new(),
                    options: HashMap::new(),
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<RuleConfig, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut level: Option<RuleLevel> = None;
                let mut ignore = None;
                let mut options = HashMap::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "level" => {
                            if level.is_some() {
                                return Err(de::Error::duplicate_field("level"));
                            }
                            let level_str: String = map.next_value()?;
                            level = Some(match level_str.as_str() {
                                "error" => RuleLevel::Error,
                                "warn" => RuleLevel::Warn,
                                "ignore" => RuleLevel::Ignore,
                                _ => {
                                    return Err(de::Error::unknown_variant(
                                        &level_str,
                                        &["error", "warn", "ignore"],
                                    ));
                                }
                            });
                        }
                        "ignore" => {
                            if ignore.is_some() {
                                return Err(de::Error::duplicate_field("ignore"));
                            }
                            ignore = Some(map.next_value()?);
                        }
                        _ => {
                            // Capture unknown fields as rule-specific options
                            let value: toml::Value = map.next_value()?;
                            options.insert(key, value);
                        }
                    }
                }

                Ok(RuleConfig {
                    level, // None when not set
                    ignore: ignore.unwrap_or_default(),
                    options,
                })
            }
        }

        deserializer.deserialize_any(RuleConfigVisitor)
    }
}

macro_rules! impl_rules_config {
    ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
        #[derive(Debug, Clone, Deserialize, Default)]
        pub struct RulesConfig {
            $(
                #[serde(default)]
                pub $config_field: RuleConfig,
            )*
        }
    };
}

for_each_rule!(impl_rules_config);

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    pub fn build_ignore_matcher(&self) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();

        for pattern in &self.ignore {
            let glob = Glob::new(pattern)
                .with_context(|| format!("Invalid ignore pattern: {}", pattern))?;
            builder.add(glob);
        }

        builder.build().context("Failed to build ignore matcher")
    }

    /// Build an ignore matcher for a specific rule, combining global and
    /// per-rule ignores
    pub fn build_rule_ignore_matcher(&self, rule_config: &RuleConfig) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();

        // Add global ignore patterns
        for pattern in &self.ignore {
            let glob = Glob::new(pattern)
                .with_context(|| format!("Invalid ignore pattern: {}", pattern))?;
            builder.add(glob);
        }

        // Add per-rule ignore patterns
        for pattern in &rule_config.ignore {
            let glob = Glob::new(pattern)
                .with_context(|| format!("Invalid ignore pattern: {}", pattern))?;
            builder.add(glob);
        }

        builder.build().context("Failed to build ignore matcher")
    }

    /// Get reference to a rule config by field name
    pub fn get_rule_config(&self, field_name: &str) -> Option<&RuleConfig> {
        macro_rules! impl_get_rule_config {
            ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
                match field_name {
                    $(
                        stringify!($config_field) => Some(&self.rules.$config_field),
                    )*
                    _ => None,
                }
            };
        }

        for_each_rule!(impl_get_rule_config)
    }

    /// Get mutable reference to a rule config by field name
    pub fn get_rule_config_mut(&mut self, field_name: &str) -> Option<&mut RuleConfig> {
        macro_rules! impl_get_rule_config_mut {
            ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
                match field_name {
                    $(
                        stringify!($config_field) => Some(&mut self.rules.$config_field),
                    )*
                    _ => None,
                }
            };
        }

        for_each_rule!(impl_get_rule_config_mut)
    }

    #[cfg(test)]
    pub fn with_min_glib_version(mut self, version: (u32, u32)) -> Self {
        self.min_glib_version = Some(version);
        self
    }

    #[cfg(test)]
    pub fn with_msvc_compatible(mut self, msvc: bool) -> Self {
        self.msvc_compatible = msvc;
        self
    }

    pub fn get_string_list(&self, rule_name: &str, key: &str) -> Vec<String> {
        self.get_rule_config(rule_name)
            .and_then(|rc| rc.options.get(key))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Enable only specific rules, disabling all others
    pub fn enable_only_rules(&mut self, rule_names: &[RuleName]) {
        macro_rules! impl_enable_only_rules {
            ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
                {
                    $(
                        self.rules.$config_field.level = Some(if rule_names.iter().any(|r| r.as_str() == stringify!($config_field)) {
                            let default_level = match self.default_level {
                                None | Some(RuleLevel::Ignore) => RuleLevel::Warn,
                                Some(level) => level,
                            };
                            match self.rules.$config_field.level {
                                None | Some(RuleLevel::Ignore) => default_level,
                                Some(level) => level,
                            }
                        } else {
                            RuleLevel::Ignore
                        });
                    )*
                }
            };
        }

        for_each_rule!(impl_enable_only_rules);
    }

    /// Disable specific rules (all others remain enabled according to config)
    pub fn disable_rules(&mut self, rule_names: &[RuleName]) {
        macro_rules! impl_disable_rules {
            ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
                {
                    $(
                        if rule_names.iter().any(|r| r.as_str() == stringify!($config_field)) {
                            self.rules.$config_field.level = Some(RuleLevel::Ignore);
                        }
                    )*
                }
            };
        }

        for_each_rule!(impl_disable_rules);
    }

    /// Filter rules by category, disabling all others
    pub fn filter_by_category(&mut self, category: Category) -> Result<()> {
        macro_rules! impl_filter_by_category {
            ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
                {
                    $(
                        if $rule_type.category() != category {
                            self.rules.$config_field.level = Some(RuleLevel::Ignore);
                        }
                    )*
                }
            };
        }

        for_each_rule!(impl_filter_by_category);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_glib_version ---

    #[test]
    fn parse_glib_version_valid() {
        assert_eq!(parse_glib_version("2.76"), Some((2, 76)));
        assert_eq!(parse_glib_version("2.0"), Some((2, 0)));
        assert_eq!(parse_glib_version("3.10"), Some((3, 10)));
    }

    #[test]
    fn parse_glib_version_invalid() {
        assert_eq!(parse_glib_version("2"), None);
        assert_eq!(parse_glib_version("2.76.1"), None);
        assert_eq!(parse_glib_version("abc.def"), None);
        assert_eq!(parse_glib_version(""), None);
        assert_eq!(parse_glib_version("2."), None);
    }

    // --- Style ---

    #[test]
    fn format_call_with_space() {
        let style = Style {
            space_before_paren: true,
        };
        assert_eq!(style.format_call("g_free", &["ptr"]), "g_free (ptr)");
        assert_eq!(
            style.format_call("g_object_set", &["obj", "\"prop\"", "val", "NULL"]),
            "g_object_set (obj, \"prop\", val, NULL)"
        );
    }

    #[test]
    fn format_call_without_space() {
        let style = Style {
            space_before_paren: false,
        };
        assert_eq!(style.format_call("g_free", &["ptr"]), "g_free(ptr)");
    }

    #[test]
    fn format_call_no_args() {
        let style = Style::default();
        assert_eq!(style.format_call("func", &[]), "func ()");
    }

    #[test]
    fn format_call_stmt_appends_semicolon() {
        let style = Style::default();
        assert_eq!(style.format_call_stmt("g_free", &["ptr"]), "g_free (ptr);");
    }

    #[test]
    fn format_addr_call_prepends_ampersand() {
        let style = Style {
            space_before_paren: true,
        };
        assert_eq!(
            style.format_addr_call("g_clear_object", "obj", &[]),
            "g_clear_object (&obj)"
        );
        assert_eq!(
            style.format_addr_call("g_set_object", "dest", &["src"]),
            "g_set_object (&dest, src)"
        );
    }

    #[test]
    fn format_addr_call_without_space() {
        let style = Style {
            space_before_paren: false,
        };
        assert_eq!(
            style.format_addr_call("g_clear_object", "obj", &[]),
            "g_clear_object(&obj)"
        );
    }

    #[test]
    fn format_addr_call_stmt_appends_semicolon() {
        let style = Style::default();
        assert_eq!(
            style.format_addr_call_stmt("g_clear_object", "obj", &[]),
            "g_clear_object (&obj);"
        );
    }

    // --- RuleLevel ---

    #[test]
    fn rule_level_is_enabled() {
        assert!(RuleLevel::Error.is_enabled());
        assert!(RuleLevel::Warn.is_enabled());
        assert!(!RuleLevel::Ignore.is_enabled());
    }

    #[test]
    fn rule_level_is_error() {
        assert!(RuleLevel::Error.is_error());
        assert!(!RuleLevel::Warn.is_error());
        assert!(!RuleLevel::Ignore.is_error());
    }

    #[test]
    fn rule_level_is_warn() {
        assert!(!RuleLevel::Error.is_warn());
        assert!(RuleLevel::Warn.is_warn());
        assert!(!RuleLevel::Ignore.is_warn());
    }

    // --- RuleConfig deserialization ---

    fn parse_rule_config(toml_value: &str) -> Result<RuleConfig, toml::de::Error> {
        #[derive(Deserialize)]
        struct Wrapper {
            rule: RuleConfig,
        }
        let wrapper: Wrapper = toml::from_str(&format!("rule = {toml_value}"))?;
        Ok(wrapper.rule)
    }

    #[test]
    fn rule_config_from_bool_true() {
        let config = parse_rule_config("true").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Error));
    }

    #[test]
    fn rule_config_from_bool_false() {
        let config = parse_rule_config("false").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Ignore));
    }

    #[test]
    fn rule_config_from_string_error() {
        let config = parse_rule_config("\"error\"").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Error));
    }

    #[test]
    fn rule_config_from_string_warn() {
        let config = parse_rule_config("\"warn\"").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Warn));
    }

    #[test]
    fn rule_config_from_string_ignore() {
        let config = parse_rule_config("\"ignore\"").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Ignore));
    }

    #[test]
    fn rule_config_from_string_invalid() {
        assert!(parse_rule_config("\"bogus\"").is_err());
    }

    #[test]
    fn rule_config_from_table_with_level() {
        let config: RuleConfig = toml::from_str(r#"level = "error""#).unwrap();
        assert_eq!(config.level, Some(RuleLevel::Error));
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn rule_config_from_table_with_ignore() {
        let config: RuleConfig =
            toml::from_str("level = \"warn\"\nignore = [\"tests/**\"]").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Warn));
        assert_eq!(config.ignore, vec!["tests/**"]);
    }

    #[test]
    fn rule_config_from_table_with_options() {
        let config: RuleConfig =
            toml::from_str("level = \"warn\"\nconfig_header = \"config.h\"").unwrap();
        assert_eq!(config.level, Some(RuleLevel::Warn));
        assert!(config.options.contains_key("config_header"));
    }

    #[test]
    fn rule_config_from_table_level_only_omitted() {
        let config: RuleConfig = toml::from_str("ignore = [\"*.h\"]").unwrap();
        assert_eq!(config.level, None);
        assert_eq!(config.ignore, vec!["*.h"]);
    }

    // --- Config::load ---

    #[test]
    fn load_missing_file_returns_default() {
        let config = Config::load(Path::new("/nonexistent/path.toml")).unwrap();
        assert!(config.min_glib_version.is_none());
        assert!(!config.msvc_compatible);
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn load_empty_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "").unwrap();
        let config = Config::load(&path).unwrap();
        assert!(config.min_glib_version.is_none());
    }

    #[test]
    fn load_with_min_glib_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "min_glib_version = \"2.76\"").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.min_glib_version, Some((2, 76)));
    }

    #[test]
    fn load_with_invalid_glib_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "min_glib_version = \"abc\"").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn load_with_msvc_compatible() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "msvc_compatible = true").unwrap();
        let config = Config::load(&path).unwrap();
        assert!(config.msvc_compatible);
    }

    #[test]
    fn load_with_ignore_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "ignore = [\"tests/**\", \"build/**\"]").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.ignore, vec!["tests/**", "build/**"]);
    }

    #[test]
    fn load_with_default_level() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "default_level = \"error\"").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.default_level, Some(RuleLevel::Error));
    }

    #[test]
    fn load_with_style_no_space() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "[style]\nspace_before_paren = false").unwrap();
        let config = Config::load(&path).unwrap();
        assert!(!config.style.space_before_paren);
    }

    // --- Config ignore matchers ---

    #[test]
    fn build_ignore_matcher_empty() {
        let config = Config::default();
        let matcher = config.build_ignore_matcher().unwrap();
        assert!(!matcher.is_match("foo.c"));
    }

    #[test]
    fn build_ignore_matcher_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "ignore = [\"tests/**\"]").unwrap();
        let config = Config::load(&path).unwrap();
        let matcher = config.build_ignore_matcher().unwrap();
        assert!(matcher.is_match("tests/foo.c"));
        assert!(!matcher.is_match("src/foo.c"));
    }

    #[test]
    fn build_rule_ignore_matcher_combines_global_and_rule() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gobject-linter.toml");
        fs::write(&path, "ignore = [\"vendor/**\"]").unwrap();
        let config = Config::load(&path).unwrap();

        let rule_config = RuleConfig {
            level: Some(RuleLevel::Warn),
            ignore: vec!["tests/**".to_string()],
            options: HashMap::new(),
        };

        let matcher = config.build_rule_ignore_matcher(&rule_config).unwrap();
        assert!(matcher.is_match("vendor/lib.c"));
        assert!(matcher.is_match("tests/test.c"));
        assert!(!matcher.is_match("src/main.c"));
    }

    // --- Config::get_rule_config ---

    #[test]
    fn get_rule_config_existing() {
        let config = Config::default();
        assert!(config.get_rule_config("dead_code").is_some());
        assert!(config.get_rule_config("include_order").is_some());
    }

    #[test]
    fn get_rule_config_nonexistent() {
        let config = Config::default();
        assert!(config.get_rule_config("nonexistent_rule").is_none());
    }

    #[test]
    fn get_rule_config_mut_existing() {
        let mut config = Config::default();
        let rc = config.get_rule_config_mut("dead_code").unwrap();
        rc.level = Some(RuleLevel::Error);
        assert_eq!(config.rules.dead_code.level, Some(RuleLevel::Error));
    }

    // --- Config::get_string_list ---

    #[test]
    fn get_string_list_missing_rule() {
        let config = Config::default();
        assert!(config.get_string_list("nonexistent", "key").is_empty());
    }

    #[test]
    fn get_string_list_missing_key() {
        let config = Config::default();
        assert!(
            config
                .get_string_list("dead_code", "missing_key")
                .is_empty()
        );
    }

    // --- Config::enable_only_rules / disable_rules ---

    #[test]
    fn enable_only_rules_disables_others() {
        let mut config = Config::default();
        config.enable_only_rules(&[crate::scanner::RuleName::DeadCode]);
        assert!(config.rules.dead_code.level.unwrap().is_enabled());
        assert!(!config.rules.include_order.level.unwrap().is_enabled());
        assert!(!config.rules.use_g_new.level.unwrap().is_enabled());
    }

    #[test]
    fn disable_rules_disables_specified() {
        let mut config = Config::default();
        config.rules.dead_code.level = Some(RuleLevel::Error);
        config.disable_rules(&[crate::scanner::RuleName::DeadCode]);
        assert_eq!(config.rules.dead_code.level, Some(RuleLevel::Ignore));
    }
}
