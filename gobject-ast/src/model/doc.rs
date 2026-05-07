use serde::Serialize;
use tree_sitter::Node;

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

/// An identifier-level annotation parsed from the symbol line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentifierAnnotation {
    Function(FunctionAnnotation),
    Type(TypeAnnotation),
    Property(PropertyAnnotation),
    Signal(SignalAnnotation),
    EnumValue(EnumValueAnnotation),
    Attributes(Vec<(String, String)>),
    Unknown(String),
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

fn format_annotation(name: &str, value: Option<&str>) -> String {
    match value {
        Some(v) => format!("{name} {v}"),
        None => name.to_owned(),
    }
}

fn parse_identifier_annotation(name: &str, value: Option<&str>) -> IdentifierAnnotation {
    match name {
        // Function
        "skip" => IdentifierAnnotation::Function(FunctionAnnotation::Skip),
        "constructor" => IdentifierAnnotation::Function(FunctionAnnotation::Constructor),
        "method" => IdentifierAnnotation::Function(FunctionAnnotation::Method),
        "virtual" => match value {
            Some(v) => IdentifierAnnotation::Function(FunctionAnnotation::Virtual(v.to_owned())),
            None => {
                tracing::warn!("doc: (virtual) requires a slot name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "set-property" => match value {
            Some(v) => {
                IdentifierAnnotation::Function(FunctionAnnotation::SetProperty(v.to_owned()))
            }
            None => {
                tracing::warn!("doc: (set-property) requires a property name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "get-property" => match value {
            Some(v) => {
                IdentifierAnnotation::Function(FunctionAnnotation::GetProperty(v.to_owned()))
            }
            None => {
                tracing::warn!("doc: (get-property) requires a property name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "rename-to" => match value {
            Some(v) => IdentifierAnnotation::Function(FunctionAnnotation::RenameTo(v.to_owned())),
            None => {
                tracing::warn!("doc: (rename-to) requires a symbol name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "sync-func" => match value {
            Some(v) => IdentifierAnnotation::Function(FunctionAnnotation::SyncFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (sync-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "async-func" => match value {
            Some(v) => IdentifierAnnotation::Function(FunctionAnnotation::AsyncFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (async-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "finish-func" => match value {
            Some(v) => IdentifierAnnotation::Function(FunctionAnnotation::FinishFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (finish-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        // Type
        "foreign" => IdentifierAnnotation::Type(TypeAnnotation::Foreign),
        "ref-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::RefFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (ref-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "unref-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::UnrefFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (unref-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "copy-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::CopyFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (copy-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "free-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::FreeFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (free-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "get-value-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::GetValueFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (get-value-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "set-value-func" => match value {
            Some(v) => IdentifierAnnotation::Type(TypeAnnotation::SetValueFunc(v.to_owned())),
            None => {
                tracing::warn!("doc: (set-value-func) requires a function name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        // Property
        "getter" => match value {
            Some(v) => IdentifierAnnotation::Property(PropertyAnnotation::Getter(v.to_owned())),
            None => {
                tracing::warn!("doc: (getter) requires a symbol name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "setter" => match value {
            Some(v) => IdentifierAnnotation::Property(PropertyAnnotation::Setter(v.to_owned())),
            None => {
                tracing::warn!("doc: (setter) requires a symbol name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        "default-value" => match value {
            Some(v) => {
                IdentifierAnnotation::Property(PropertyAnnotation::DefaultValue(v.to_owned()))
            }
            None => {
                tracing::warn!("doc: (default-value) requires a value");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        // Signal
        "emitter" => match value {
            Some(v) => IdentifierAnnotation::Signal(SignalAnnotation::Emitter(v.to_owned())),
            None => {
                tracing::warn!("doc: (emitter) requires a method name");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        // Enum value
        "value" => match value {
            Some(v) => IdentifierAnnotation::EnumValue(EnumValueAnnotation::Value(v.to_owned())),
            None => {
                tracing::warn!("doc: (value) requires a value");
                IdentifierAnnotation::Unknown(format_annotation(name, value))
            }
        },
        // Attributes (any context)
        "attributes" => {
            let pairs = value
                .map(|v| {
                    v.split_whitespace()
                        .filter_map(|pair| {
                            let (k, val) = pair.split_once('=')?;
                            Some((k.to_owned(), val.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            IdentifierAnnotation::Attributes(pairs)
        }
        _ => {
            tracing::warn!(
                "doc: unknown identifier annotation: ({})",
                format_annotation(name, value)
            );
            IdentifierAnnotation::Unknown(format_annotation(name, value))
        }
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

#[derive(Debug, Clone, Default)]
pub struct DocComment {
    pub symbol: Option<String>,
    pub annotations: Vec<IdentifierAnnotation>,
    pub params: Vec<DocParam>,
    pub returns: Option<DocReturns>,
    pub description: Vec<String>,
    pub since: Option<String>,
    pub deprecated: Option<String>,
}

impl DocComment {
    /// Parse the doc comment preceding `node` in the C tree-sitter AST.
    pub fn from_node(node: Node<'_>, source: &[u8]) -> Option<Self> {
        let prev = node.prev_named_sibling()?;
        if prev.kind() != "comment" {
            return None;
        }
        let text = std::str::from_utf8(&source[prev.byte_range()]).ok()?;
        if !text.starts_with("/**") {
            return None;
        }
        Self::from_comment_text(text)
    }

    /// Parse a raw `/** ... */` comment string.
    pub fn from_comment_text(text: &str) -> Option<Self> {
        let text = text.strip_prefix("/**")?.strip_suffix("*/")?.trim();

        let mut doc = Self::default();

        for raw_line in text.lines() {
            let line = raw_line.trim().strip_prefix('*').unwrap_or(raw_line.trim());
            let line = line.strip_prefix(' ').unwrap_or(line);

            if line.is_empty() {
                continue;
            }

            if let Some(rest) = line.strip_prefix('@') {
                if let Some((name, after_colon)) = rest.split_once(':') {
                    let (annotations, desc) =
                        parse_annotations_and_desc(after_colon.trim(), ParamAnnotation::parse);
                    doc.params.push(DocParam {
                        name: name.trim().to_owned(),
                        annotations,
                        description: desc,
                    });
                }
            } else if let Some(after) = line.strip_prefix("Returns:") {
                let (annotations, desc) =
                    parse_annotations_and_desc(after.trim(), ReturnAnnotation::parse);
                doc.returns = Some(DocReturns {
                    annotations,
                    description: desc,
                });
            } else if let Some(v) = line.strip_prefix("Since:") {
                doc.since = Some(v.trim().to_owned());
            } else if let Some(v) = line.strip_prefix("Deprecated:") {
                doc.deprecated = Some(v.trim().to_owned());
            } else if doc.symbol.is_none() && doc.params.is_empty() && doc.description.is_empty() {
                let (before_colon, after_colon) = match line.split_once(':') {
                    Some((b, a)) => (b, Some(a)),
                    None => (line, None),
                };
                if before_colon
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-')
                {
                    doc.symbol = Some(before_colon.to_owned());
                    if let Some(rest) = after_colon {
                        let (annotations, _) =
                            parse_annotations_and_desc(rest.trim(), parse_identifier_annotation);
                        doc.annotations = annotations;
                    }
                } else {
                    doc.description.push(line.to_owned());
                }
            } else {
                doc.description.push(line.to_owned());
            }
        }

        Some(doc)
    }

    /// Convert to a function doc, warning about misplaced annotations.
    pub fn into_function_doc(self) -> FunctionDoc {
        let mut annotations = Vec::new();
        for a in self.annotations {
            match a {
                IdentifierAnnotation::Function(f) => annotations.push(f),
                IdentifierAnnotation::Attributes(_) | IdentifierAnnotation::Unknown(_) => {}
                other => {
                    tracing::warn!("doc: annotation {other:?} is not valid on a function");
                }
            }
        }
        FunctionDoc {
            symbol: self.symbol,
            annotations,
            params: self.params,
            returns: self.returns,
            description: self.description,
            since: self.since,
            deprecated: self.deprecated,
        }
    }

    /// Convert to a type doc, warning about misplaced annotations.
    pub fn into_type_doc(self) -> TypeDoc {
        let mut annotations = Vec::new();
        for a in self.annotations {
            match a {
                IdentifierAnnotation::Type(t) => annotations.push(t),
                IdentifierAnnotation::Attributes(_) | IdentifierAnnotation::Unknown(_) => {}
                other => {
                    tracing::warn!("doc: annotation {other:?} is not valid on a type");
                }
            }
        }
        if !self.params.is_empty() {
            tracing::warn!("doc: @param annotations are not valid on a type");
        }
        if self.returns.is_some() {
            tracing::warn!("doc: Returns: is not valid on a type");
        }
        TypeDoc {
            symbol: self.symbol,
            annotations,
            description: self.description,
            since: self.since,
            deprecated: self.deprecated,
        }
    }

    /// Convert to a property doc, warning about misplaced annotations.
    pub fn into_property_doc(self) -> PropertyDoc {
        let mut annotations = Vec::new();
        for a in self.annotations {
            match a {
                IdentifierAnnotation::Property(p) => annotations.push(p),
                IdentifierAnnotation::Attributes(_) | IdentifierAnnotation::Unknown(_) => {}
                other => {
                    tracing::warn!("doc: annotation {other:?} is not valid on a property");
                }
            }
        }
        if !self.params.is_empty() {
            tracing::warn!("doc: @param annotations are not valid on a property");
        }
        if self.returns.is_some() {
            tracing::warn!("doc: Returns: is not valid on a property");
        }
        PropertyDoc {
            symbol: self.symbol,
            annotations,
            description: self.description,
            since: self.since,
            deprecated: self.deprecated,
        }
    }

    /// Convert to a signal doc, warning about misplaced annotations.
    pub fn into_signal_doc(self) -> SignalDoc {
        let mut annotations = Vec::new();
        for a in self.annotations {
            match a {
                IdentifierAnnotation::Signal(s) => annotations.push(s),
                IdentifierAnnotation::Attributes(_) | IdentifierAnnotation::Unknown(_) => {}
                other => {
                    tracing::warn!("doc: annotation {other:?} is not valid on a signal");
                }
            }
        }
        if !self.params.is_empty() {
            tracing::warn!("doc: @param annotations are not valid on a signal");
        }
        if self.returns.is_some() {
            tracing::warn!("doc: Returns: is not valid on a signal");
        }
        SignalDoc {
            symbol: self.symbol,
            annotations,
            description: self.description,
            since: self.since,
            deprecated: self.deprecated,
        }
    }

    /// Convert to an enum value doc, warning about misplaced annotations.
    pub fn into_enum_value_doc(self) -> EnumValueDoc {
        let mut annotations = Vec::new();
        for a in self.annotations {
            match a {
                IdentifierAnnotation::EnumValue(e) => annotations.push(e),
                IdentifierAnnotation::Attributes(_) | IdentifierAnnotation::Unknown(_) => {}
                other => {
                    tracing::warn!("doc: annotation {other:?} is not valid on an enum value");
                }
            }
        }
        if !self.params.is_empty() {
            tracing::warn!("doc: @param annotations are not valid on an enum value");
        }
        if self.returns.is_some() {
            tracing::warn!("doc: Returns: is not valid on an enum value");
        }
        EnumValueDoc {
            symbol: self.symbol,
            annotations,
            description: self.description,
            since: self.since,
            deprecated: self.deprecated,
        }
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
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
}

impl FunctionDoc {
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
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
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
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<SignalAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub description: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
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
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
}

/// Parse `(annotation1) (annotation2): description text` using the
/// provided parse function for the annotation type.
fn parse_annotations_and_desc<T>(
    text: &str,
    parse_fn: fn(&str, Option<&str>) -> T,
) -> (Vec<T>, String) {
    let mut annotations = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find('(') {
        let Some(end) = rest[start..].find(')') else {
            break;
        };
        let end = start + end;
        let inner = &rest[start + 1..end];

        let (name, value) = match inner.split_once(' ') {
            Some((n, v)) => (n, Some(v)),
            None => (inner, None),
        };

        annotations.push(parse_fn(name, value));

        rest = rest[end + 1..].trim_start();
    }

    let desc = rest.strip_prefix(':').unwrap_or(rest).trim();
    (annotations, desc.to_owned())
}
