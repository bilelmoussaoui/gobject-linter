use std::{fmt, str::FromStr};

use serde::Serialize;
use tree_sitter::Node;

use super::Comment;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for Version {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (major, minor) = s.split_once('.').ok_or(())?;
        Ok(Self {
            major: major.parse().map_err(|_| ())?,
            minor: minor.parse().map_err(|_| ())?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportMacro {
    AvailableIn(Version),
    DeprecatedIn(Version),
    Other(String),
}

impl ExportMacro {
    pub fn parse(name: &str) -> Self {
        if let Some(suffix) = name.split("AVAILABLE_IN_").nth(1) {
            if let Some(ver) = Self::parse_version_suffix(suffix) {
                return Self::AvailableIn(ver);
            }
        } else if let Some(suffix) = name.split("DEPRECATED_IN_").nth(1)
            && let Some(ver) = Self::parse_version_suffix(suffix)
        {
            return Self::DeprecatedIn(ver);
        }
        Self::Other(name.to_owned())
    }

    fn parse_version_suffix(suffix: &str) -> Option<Version> {
        let (major, minor) = suffix.split_once('_')?;
        Some(Version {
            major: major.parse().ok()?,
            minor: minor.parse().ok()?,
        })
    }

    pub fn version(&self) -> Option<&Version> {
        match self {
            Self::AvailableIn(v) | Self::DeprecatedIn(v) => Some(v),
            Self::Other(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransferKind {
    None,
    Full,
    Container,
    Floating,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScopeKind {
    Call,
    Async,
    Notified,
    Forever,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArrayAnnotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zero_terminated: Option<bool>,
}

/// Annotations valid on function parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamAnnotation {
    Transfer(TransferKind),
    Nullable,
    NotNullable,
    Optional,
    AllowNone,
    NotOptional,
    In,
    Out,
    OutCallerAllocates,
    OutCalleeAllocates,
    Inout,
    Array,
    ArrayDetailed(ArrayAnnotation),
    ElementType(Vec<String>),
    Scope(ScopeKind),
    Closure,
    ClosureFor(String),
    Destroy(String),
    Type(String),
    Skip,
    Default(String),
    Attributes(Vec<(String, String)>),
    Unknown(String),
}

impl ParamAnnotation {
    pub fn parse(name: &str, value: Option<&str>) -> Self {
        match name {
            "transfer" => match parse_transfer(value) {
                Ok(t) => Self::Transfer(t),
                Err(e) => {
                    tracing::warn!("doc: {e}");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "nullable" => Self::Nullable,
            "not nullable" => Self::NotNullable,
            "optional" => Self::Optional,
            "allow-none" => Self::AllowNone,
            "not optional" => Self::NotOptional,
            "caller-allocates" => Self::OutCallerAllocates,
            "callee-allocates" => Self::OutCalleeAllocates,
            "in" => Self::In,
            "out" => match value {
                None => Self::Out,
                Some("caller-allocates") => Self::OutCallerAllocates,
                Some("callee-allocates") => Self::OutCalleeAllocates,
                Some(v) => {
                    tracing::warn!("doc: unknown out modifier: {v:?}");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "inout" | "in-out" => Self::Inout,
            "array" => match value {
                Some(v) => Self::ArrayDetailed(parse_array(v)),
                None => Self::Array,
            },
            "element-type" => match value {
                Some(v) => Self::ElementType(v.split_whitespace().map(String::from).collect()),
                None => {
                    tracing::warn!("doc: element-type requires at least one type");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "scope" => match parse_scope(value) {
                Ok(s) => Self::Scope(s),
                Err(e) => {
                    tracing::warn!("doc: {e}");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "closure" => match value {
                Some(v) => Self::ClosureFor(v.to_owned()),
                None => Self::Closure,
            },
            "destroy" => match value {
                Some(v) => Self::Destroy(v.to_owned()),
                None => {
                    tracing::warn!("doc: destroy requires a parameter name");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "type" => match value {
                Some(v) => Self::Type(v.to_owned()),
                None => {
                    tracing::warn!("doc: type requires a type name");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "skip" => Self::Skip,
            "attributes" => Self::Attributes(parse_attributes(value)),
            "default" => match value {
                Some(v) => Self::Default(v.to_owned()),
                None => {
                    tracing::warn!("doc: default requires a value");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            _ => {
                tracing::warn!(
                    "doc: unknown param annotation: ({})",
                    format_annotation(name, value)
                );
                Self::Unknown(format_annotation(name, value))
            }
        }
    }
}

/// Annotations valid on return values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReturnAnnotation {
    Transfer(TransferKind),
    Nullable,
    NotNullable,
    Optional,
    Skip,
    Array,
    ArrayDetailed(ArrayAnnotation),
    ElementType(Vec<String>),
    Type(String),
    Attributes(Vec<(String, String)>),
    Unknown(String),
}

impl ReturnAnnotation {
    pub fn parse(name: &str, value: Option<&str>) -> Self {
        match name {
            "transfer" => match parse_transfer(value) {
                Ok(t) => Self::Transfer(t),
                Err(e) => {
                    tracing::warn!("doc: {e}");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "nullable" => Self::Nullable,
            "not nullable" => Self::NotNullable,
            "optional" => Self::Optional,
            "skip" => Self::Skip,
            "array" => match value {
                Some(v) => Self::ArrayDetailed(parse_array(v)),
                None => Self::Array,
            },
            "element-type" => match value {
                Some(v) => Self::ElementType(v.split_whitespace().map(String::from).collect()),
                None => {
                    tracing::warn!("doc: element-type requires at least one type");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "type" => match value {
                Some(v) => Self::Type(v.to_owned()),
                None => {
                    tracing::warn!("doc: type requires a type name");
                    Self::Unknown(format_annotation(name, value))
                }
            },
            "attributes" => Self::Attributes(parse_attributes(value)),
            _ => {
                tracing::warn!(
                    "doc: unknown return annotation: ({})",
                    format_annotation(name, value)
                );
                Self::Unknown(format_annotation(name, value))
            }
        }
    }

    pub fn transfer(&self) -> Option<&TransferKind> {
        if let Self::Transfer(k) = self {
            Some(k)
        } else {
            None
        }
    }
}

/// Annotations valid on functions/methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FunctionAnnotation {
    Skip,
    Constructor,
    Method,
    Virtual(String),
    SetProperty(String),
    GetProperty(String),
    RenameTo(String),
    SyncFunc(String),
    AsyncFunc(String),
    FinishFunc(String),
}

/// Annotations valid on type declarations (structs, boxed, fundamental).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeAnnotation {
    Skip,
    Foreign,
    RenameTo(String),
    RefFunc(String),
    UnrefFunc(String),
    CopyFunc(String),
    FreeFunc(String),
    GetValueFunc(String),
    SetValueFunc(String),
}

/// Annotations valid on properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropertyAnnotation {
    Getter(String),
    Setter(String),
    DefaultValue(String),
}

/// Annotations valid on signals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalAnnotation {
    Emitter(String),
}

/// Annotations valid on enum/flag values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnumValueAnnotation {
    Value(String),
}

fn parse_transfer(value: Option<&str>) -> Result<TransferKind, String> {
    match value {
        Some("none") => Ok(TransferKind::None),
        Some("full") => Ok(TransferKind::Full),
        Some("container") => Ok(TransferKind::Container),
        Some("floating") => Ok(TransferKind::Floating),
        _ => Err(format!("unknown transfer kind: {:?}", value)),
    }
}

fn parse_scope(value: Option<&str>) -> Result<ScopeKind, String> {
    match value {
        Some("call") => Ok(ScopeKind::Call),
        Some("async") => Ok(ScopeKind::Async),
        Some("notified") => Ok(ScopeKind::Notified),
        Some("forever") => Ok(ScopeKind::Forever),
        _ => Err(format!("unknown scope kind: {:?}", value)),
    }
}

fn parse_array(value: &str) -> ArrayAnnotation {
    let mut length = None;
    let mut fixed_size = None;
    let mut zero_terminated = None;

    for part in value.split_whitespace() {
        if let Some(v) = part.strip_prefix("length=") {
            length = Some(v.to_owned());
        } else if let Some(v) = part.strip_prefix("fixed-size=") {
            fixed_size = v.parse().ok();
        } else if let Some(v) = part.strip_prefix("zero-terminated=") {
            zero_terminated = match v {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            };
        }
    }

    ArrayAnnotation {
        length,
        fixed_size,
        zero_terminated,
    }
}

fn parse_attributes(value: Option<&str>) -> Vec<(String, String)> {
    value
        .unwrap_or("")
        .split_whitespace()
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((k.to_owned(), v.to_owned()))
        })
        .collect()
}

fn format_annotation(name: &str, value: Option<&str>) -> String {
    match value {
        Some(v) => format!("{name} {v}"),
        None => name.to_owned(),
    }
}

fn parse_value_annotation<A>(
    name: &str,
    value: Option<&str>,
    label: &str,
    f: fn(String) -> A,
) -> Option<A> {
    match value {
        Some(v) => Some(f(v.to_owned())),
        None => {
            tracing::warn!("doc: ({name}) requires {label}");
            None
        }
    }
}

fn parse_function_annotation(name: &str, value: Option<&str>) -> Option<FunctionAnnotation> {
    match name {
        "skip" => Some(FunctionAnnotation::Skip),
        "constructor" => Some(FunctionAnnotation::Constructor),
        "method" => Some(FunctionAnnotation::Method),
        "virtual" => {
            parse_value_annotation(name, value, "a slot name", FunctionAnnotation::Virtual)
        }
        "set-property" => parse_value_annotation(
            name,
            value,
            "a property name",
            FunctionAnnotation::SetProperty,
        ),
        "get-property" => parse_value_annotation(
            name,
            value,
            "a property name",
            FunctionAnnotation::GetProperty,
        ),
        "rename-to" => {
            parse_value_annotation(name, value, "a symbol name", FunctionAnnotation::RenameTo)
        }
        "sync-func" => {
            parse_value_annotation(name, value, "a function name", FunctionAnnotation::SyncFunc)
        }
        "async-func" => parse_value_annotation(
            name,
            value,
            "a function name",
            FunctionAnnotation::AsyncFunc,
        ),
        "finish-func" => parse_value_annotation(
            name,
            value,
            "a function name",
            FunctionAnnotation::FinishFunc,
        ),
        _ => None,
    }
}

fn parse_type_annotation(name: &str, value: Option<&str>) -> Option<TypeAnnotation> {
    match name {
        "skip" => Some(TypeAnnotation::Skip),
        "foreign" => Some(TypeAnnotation::Foreign),
        "rename-to" => {
            parse_value_annotation(name, value, "a symbol name", TypeAnnotation::RenameTo)
        }
        "ref-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::RefFunc)
        }
        "unref-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::UnrefFunc)
        }
        "copy-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::CopyFunc)
        }
        "free-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::FreeFunc)
        }
        "get-value-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::GetValueFunc)
        }
        "set-value-func" => {
            parse_value_annotation(name, value, "a function name", TypeAnnotation::SetValueFunc)
        }
        _ => None,
    }
}

fn parse_property_annotation(name: &str, value: Option<&str>) -> Option<PropertyAnnotation> {
    match name {
        "getter" => {
            parse_value_annotation(name, value, "a symbol name", PropertyAnnotation::Getter)
        }
        "setter" => {
            parse_value_annotation(name, value, "a symbol name", PropertyAnnotation::Setter)
        }
        "default-value" => {
            parse_value_annotation(name, value, "a value", PropertyAnnotation::DefaultValue)
        }
        _ => None,
    }
}

fn parse_signal_annotation(name: &str, value: Option<&str>) -> Option<SignalAnnotation> {
    match name {
        "emitter" => {
            parse_value_annotation(name, value, "a method name", SignalAnnotation::Emitter)
        }
        _ => None,
    }
}

fn parse_enum_value_annotation(name: &str, value: Option<&str>) -> Option<EnumValueAnnotation> {
    match name {
        "value" => parse_value_annotation(name, value, "a value", EnumValueAnnotation::Value),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DocParam {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<ParamAnnotation>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocReturns {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<ReturnAnnotation>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

struct RawDoc<A> {
    symbol: Option<String>,
    annotations: Vec<A>,
    params: Vec<DocParam>,
    returns: Option<DocReturns>,
    description: Vec<String>,
    since: Option<Version>,
    deprecated: Option<(Version, Option<String>)>,
}

impl<A> RawDoc<A> {
    fn from_node(
        node: Node<'_>,
        source: &[u8],
        parse_annotation: fn(&str, Option<&str>) -> Option<A>,
    ) -> Option<Self> {
        let prev = node.prev_named_sibling()?;
        if prev.kind() != "comment" {
            return None;
        }
        let text = std::str::from_utf8(&source[prev.byte_range()]).ok()?;
        if !text.starts_with("/**") {
            return None;
        }
        Self::from_text(text, parse_annotation)
    }

    fn from_comment(
        comment: &Comment,
        parse_annotation: fn(&str, Option<&str>) -> Option<A>,
    ) -> Option<Self> {
        if !comment.is_gtk_doc() {
            return None;
        }
        Self::from_text(&comment.text, parse_annotation)
    }

    fn from_text(
        text: &str,
        parse_annotation: fn(&str, Option<&str>) -> Option<A>,
    ) -> Option<Self> {
        let text = text.strip_prefix("/**")?.strip_suffix("*/")?.trim();

        let mut symbol = None;
        let mut annotations = Vec::new();
        let mut params = Vec::new();
        let mut returns = None;
        let mut description = Vec::new();
        let mut since = None;
        let mut deprecated = None;
        let mut past_symbol = false;
        let mut in_param = false;

        for raw_line in text.lines() {
            let line = raw_line.trim().strip_prefix('*').unwrap_or(raw_line.trim());
            let line = line.strip_prefix(' ').unwrap_or(line);

            if line.is_empty() {
                if in_param {
                    in_param = false;
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix('@')
                && let Some((name, after_colon)) = rest.split_once(':')
            {
                let (anns, desc) =
                    parse_annotations_and_desc(after_colon.trim(), ParamAnnotation::parse);
                params.push(DocParam {
                    name: name.trim().to_owned(),
                    annotations: anns,
                    description: desc,
                });
                in_param = true;
            } else if in_param && line.starts_with(' ') {
                // Continuation of previous param description
                if let Some(last) = params.last_mut() {
                    if !last.description.is_empty() {
                        last.description.push(' ');
                    }
                    last.description.push_str(line.trim());
                }
            } else if let Some(after) = line.strip_prefix("Returns:") {
                in_param = false;
                let (anns, desc) =
                    parse_annotations_and_desc(after.trim(), ReturnAnnotation::parse);
                returns = Some(DocReturns {
                    annotations: anns,
                    description: desc,
                });
            } else if let Some(v) = line.strip_prefix("Since:") {
                in_param = false;
                since = v.trim().parse().ok();
            } else if let Some(v) = line.strip_prefix("Deprecated:") {
                in_param = false;
                let v = v.trim();
                let ver_end = v
                    .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .unwrap_or(v.len());
                let ver_str = v[..ver_end].trim_end_matches('.');
                if let Ok(version) = ver_str.parse::<Version>() {
                    let rest = v[ver_end..].trim().trim_start_matches(':').trim();
                    let message = if rest.is_empty() {
                        None
                    } else {
                        Some(rest.to_owned())
                    };
                    deprecated = Some((version, message));
                }
            } else if !past_symbol {
                in_param = false;
                let symbol_end = line
                    .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '-'))
                    .unwrap_or(line.len());
                let candidate = &line[..symbol_end];
                let rest = line[symbol_end..].trim();

                let sym = candidate.trim_end_matches(':');
                if !sym.is_empty()
                    && candidate.ends_with(':')
                    && sym
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-')
                {
                    symbol = Some(sym.to_owned());
                    past_symbol = true;
                    if !rest.is_empty() {
                        annotations = parse_symbol_annotations(rest, parse_annotation);
                    }
                } else {
                    past_symbol = true;
                    description.push(line.to_owned());
                }
            } else {
                in_param = false;
                description.push(line.to_owned());
            }
        }

        Some(Self {
            symbol,
            annotations,
            params,
            returns,
            description,
            since,
            deprecated,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<FunctionAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DocParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns: Option<DocReturns>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<(Version, Option<String>)>,
}

impl FunctionDoc {
    pub fn from_node(node: Node<'_>, source: &[u8]) -> Option<Self> {
        RawDoc::from_node(node, source, parse_function_annotation).map(Self::from_raw)
    }

    pub fn from_node_for(node: Node<'_>, source: &[u8], expected_name: &str) -> Option<Self> {
        let doc = Self::from_node(node, source)?;
        match &doc.symbol {
            Some(sym) if sym != expected_name => None,
            _ => Some(doc),
        }
    }

    fn from_raw(raw: RawDoc<FunctionAnnotation>) -> Self {
        Self {
            symbol: raw.symbol,
            annotations: raw.annotations,
            params: raw.params,
            returns: raw.returns,
            description: raw.description,
            since: raw.since,
            deprecated: raw.deprecated,
        }
    }

    pub fn param(&self, name: &str) -> Option<&DocParam> {
        self.params.iter().find(|p| p.name == name)
    }

    pub fn param_has_annotation(&self, param: &str, annotation: &ParamAnnotation) -> bool {
        self.param(param)
            .is_some_and(|p| p.annotations.contains(annotation))
    }

    pub fn return_transfer(&self) -> Option<&TransferKind> {
        self.returns
            .as_ref()?
            .annotations
            .iter()
            .find_map(|a| a.transfer())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<TypeAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<(Version, Option<String>)>,
}

impl TypeDoc {
    pub fn from_node(node: Node<'_>, source: &[u8]) -> Option<Self> {
        RawDoc::from_node(node, source, parse_type_annotation).map(Self::from_raw)
    }

    pub fn from_node_for(node: Node<'_>, source: &[u8], expected_name: &str) -> Option<Self> {
        let doc = Self::from_node(node, source)?;
        match &doc.symbol {
            Some(sym) => {
                let bare_sym = sym.trim_start_matches('_');
                let bare_name = expected_name.trim_start_matches('_');
                if bare_sym == bare_name {
                    Some(doc)
                } else {
                    None
                }
            }
            None => Some(doc),
        }
    }

    pub fn from_comment(comment: &Comment) -> Option<Self> {
        RawDoc::from_comment(comment, parse_type_annotation).map(Self::from_raw)
    }

    fn from_raw(raw: RawDoc<TypeAnnotation>) -> Self {
        Self {
            symbol: raw.symbol,
            annotations: raw.annotations,
            description: raw.description,
            since: raw.since,
            deprecated: raw.deprecated,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertyDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<PropertyAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<(Version, Option<String>)>,
}

impl PropertyDoc {
    pub fn from_comment(comment: &Comment) -> Option<Self> {
        RawDoc::from_comment(comment, parse_property_annotation).map(Self::from_raw)
    }

    /// Attach only if the doc symbol matches `TypeName:property-name`.
    pub fn from_comment_for(
        comment: &Comment,
        type_name: &str,
        property_name: &str,
    ) -> Option<Self> {
        let doc = Self::from_comment(comment)?;
        let expected = format!("{type_name}:{property_name}");
        match &doc.symbol {
            Some(sym) if sym != &expected => None,
            _ => Some(doc),
        }
    }

    fn from_raw(raw: RawDoc<PropertyAnnotation>) -> Self {
        Self {
            symbol: raw.symbol,
            annotations: raw.annotations,
            description: raw.description,
            since: raw.since,
            deprecated: raw.deprecated,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<SignalAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DocParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns: Option<DocReturns>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<(Version, Option<String>)>,
}

impl SignalDoc {
    pub fn from_comment(comment: &Comment) -> Option<Self> {
        RawDoc::from_comment(comment, parse_signal_annotation).map(Self::from_raw)
    }

    /// Attach only if the doc symbol matches `TypeName::signal-name`.
    pub fn from_comment_for(comment: &Comment, type_name: &str, signal_name: &str) -> Option<Self> {
        let doc = Self::from_comment(comment)?;
        let expected = format!("{type_name}::{signal_name}");
        match &doc.symbol {
            Some(sym) if sym != &expected => None,
            _ => Some(doc),
        }
    }

    fn from_raw(raw: RawDoc<SignalAnnotation>) -> Self {
        Self {
            symbol: raw.symbol,
            annotations: raw.annotations,
            params: raw.params,
            returns: raw.returns,
            description: raw.description,
            since: raw.since,
            deprecated: raw.deprecated,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumValueDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<EnumValueAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<(Version, Option<String>)>,
}

impl EnumValueDoc {
    pub fn from_comment(comment: &Comment) -> Option<Self> {
        RawDoc::from_comment(comment, parse_enum_value_annotation).map(Self::from_raw)
    }

    /// Extract inline `@VALUE: description` entries from a parent type doc
    /// comment (e.g. `/** GtkAlign: @GTK_ALIGN_FILL: ... */`).
    pub fn extract_inline_from_comment(comment: &Comment) -> Vec<(String, Self)> {
        let Some(raw) = RawDoc::from_comment(comment, parse_type_annotation) else {
            return Vec::new();
        };
        raw.params
            .into_iter()
            .map(|p| {
                let doc = Self {
                    symbol: Some(p.name.clone()),
                    annotations: Vec::new(),
                    description: if p.description.is_empty() {
                        Vec::new()
                    } else {
                        vec![p.description]
                    },
                    since: None,
                    deprecated: None,
                };
                (p.name, doc)
            })
            .collect()
    }

    fn from_raw(raw: RawDoc<EnumValueAnnotation>) -> Self {
        Self {
            symbol: raw.symbol,
            annotations: raw.annotations,
            description: raw.description,
            since: raw.since,
            deprecated: raw.deprecated,
        }
    }
}

/// Parse `(annotation1) (annotation2): description text` using the
/// provided parse function for the annotation type.
fn is_annotation_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c == ' ')
}

fn parse_annotations_and_desc<T>(
    text: &str,
    parse_fn: fn(&str, Option<&str>) -> T,
) -> (Vec<T>, String) {
    let mut annotations = Vec::new();
    let mut rest = text;

    // Annotations must appear consecutively at the start: (nullable)(transfer full)
    while rest.starts_with('(') {
        let Some(end) = rest.find(')') else {
            break;
        };
        let inner = &rest[1..end];

        let (name, value) = if let Some(rest_after) = inner.strip_prefix("not ") {
            if let Some((_, v)) = rest_after.split_once(' ') {
                (&inner[..inner.len() - v.len() - 1], Some(v))
            } else {
                (inner, None)
            }
        } else {
            match inner.split_once(' ') {
                Some((n, v)) => (n, Some(v)),
                None => (inner, None),
            }
        };

        if !is_annotation_name(name) {
            break;
        }

        annotations.push(parse_fn(name, value));

        rest = rest[end + 1..].trim_start();
    }

    let desc = rest.strip_prefix(':').unwrap_or(rest).trim();
    (annotations, desc.to_owned())
}

fn parse_symbol_annotations<A>(
    text: &str,
    parse_fn: fn(&str, Option<&str>) -> Option<A>,
) -> Vec<A> {
    let mut annotations = Vec::new();
    let mut rest = text;

    while rest.starts_with('(') {
        let Some(end) = rest.find(')') else {
            break;
        };
        let inner = &rest[1..end];

        let (name, value) = if let Some(rest_after) = inner.strip_prefix("not ") {
            if let Some((_, v)) = rest_after.split_once(' ') {
                (&inner[..inner.len() - v.len() - 1], Some(v))
            } else {
                (inner, None)
            }
        } else {
            match inner.split_once(' ') {
                Some((n, v)) => (n, Some(v)),
                None => (inner, None),
            }
        };

        if !is_annotation_name(name) {
            break;
        }

        if let Some(a) = parse_fn(name, value) {
            annotations.push(a);
        }

        rest = rest[end + 1..].trim_start();
    }

    annotations
}
