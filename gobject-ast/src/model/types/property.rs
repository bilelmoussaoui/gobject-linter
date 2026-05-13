use std::ffi::{c_long, c_ulong};

use serde::Serialize;

use crate::model::{
    SourceLocation,
    doc::PropertyDoc,
    expression::{Argument, CallExpression, Expression},
    operators::UnaryOp,
    types::GType,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamFlag {
    /// The parameter is readable (value: 1)
    Readable,
    /// The parameter is writable (value: 2)
    Writable,
    /// Alias for READABLE | WRITABLE (value: 3)
    ReadWrite,
    /// The parameter will be set upon object construction (value: 4)
    Construct,
    /// The parameter can only be set upon object construction (value: 8)
    ConstructOnly,
    /// Strict validation not required upon parameter conversion (value: 16)
    LaxValidation,
    /// String used as name is guaranteed to remain valid (value: 32)
    StaticName,
    /// Internal flag (value: 32)
    Private,
    /// String used as nick is guaranteed to remain valid (value: 64)
    StaticNick,
    /// String used as blurb is guaranteed to remain valid (value: 128)
    StaticBlurb,
    /// Alias for STATIC_NAME | STATIC_NICK | STATIC_BLURB
    StaticStrings,
    /// No automatic notify signal emission (value: 1073741824)
    ExplicitNotify,
    /// The parameter is deprecated (value: 2147483648)
    Deprecated,
    /// Custom or unrecognized flag
    Unknown(String),
}

impl ParamFlag {
    pub fn from_identifier(name: &str) -> Self {
        match name {
            "G_PARAM_READABLE" => Self::Readable,
            "G_PARAM_WRITABLE" => Self::Writable,
            "G_PARAM_READWRITE" => Self::ReadWrite,
            "G_PARAM_CONSTRUCT" => Self::Construct,
            "G_PARAM_CONSTRUCT_ONLY" => Self::ConstructOnly,
            "G_PARAM_LAX_VALIDATION" => Self::LaxValidation,
            "G_PARAM_STATIC_NAME" => Self::StaticName,
            "G_PARAM_PRIVATE" => Self::Private,
            "G_PARAM_STATIC_NICK" => Self::StaticNick,
            "G_PARAM_STATIC_BLURB" => Self::StaticBlurb,
            "G_PARAM_STATIC_STRINGS" => Self::StaticStrings,
            "G_PARAM_EXPLICIT_NOTIFY" => Self::ExplicitNotify,
            "G_PARAM_DEPRECATED" => Self::Deprecated,
            _ => Self::Unknown(name.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Readable => "G_PARAM_READABLE",
            Self::Writable => "G_PARAM_WRITABLE",
            Self::ReadWrite => "G_PARAM_READWRITE",
            Self::Construct => "G_PARAM_CONSTRUCT",
            Self::ConstructOnly => "G_PARAM_CONSTRUCT_ONLY",
            Self::LaxValidation => "G_PARAM_LAX_VALIDATION",
            Self::StaticName => "G_PARAM_STATIC_NAME",
            Self::Private => "G_PARAM_PRIVATE",
            Self::StaticNick => "G_PARAM_STATIC_NICK",
            Self::StaticBlurb => "G_PARAM_STATIC_BLURB",
            Self::StaticStrings => "G_PARAM_STATIC_STRINGS",
            Self::ExplicitNotify => "G_PARAM_EXPLICIT_NOTIFY",
            Self::Deprecated => "G_PARAM_DEPRECATED",
            Self::Unknown(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Property {
    pub name: String,
    pub property_type: PropertyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<ParamFlag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<PropertyDoc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    String,
    Int {
        min: i32,
        max: i32,
        default: i32,
    },
    UInt {
        min: u32,
        max: u32,
        default: u32,
    },
    Int64 {
        min: i64,
        max: i64,
        default: i64,
    },
    UInt64 {
        min: u64,
        max: u64,
        default: u64,
    },
    Long {
        min: c_long,
        max: c_long,
        default: c_long,
    },
    ULong {
        min: c_ulong,
        max: c_ulong,
        default: c_ulong,
    },
    Char {
        min: i8,
        max: i8,
        default: i8,
    },
    UChar {
        min: u8,
        max: u8,
        default: u8,
    },
    Unichar {
        default: u32,
    },
    Param {
        param_type: GType,
    },
    Boolean {
        default: bool,
    },
    Float {
        min: f32,
        max: f32,
        default: f32,
    },
    Double {
        min: f64,
        max: f64,
        default: f64,
    },
    Enum {
        enum_type: GType,
        default: i32,
    },
    Flags {
        flags_type: GType,
        default: u32,
    },
    Object {
        object_type: GType,
    },
    Boxed {
        boxed_type: GType,
    },
    Pointer,
    GType {
        is_a_type: GType,
    },
    Variant {
        variant_type: Option<String>,
        default_value: Option<Expression>,
    },
    Override,
    Unknown {
        spec_function: String,
    },
}

impl Property {
    /// Extract property from g_param_spec_* call
    /// Call signature varies by type:
    /// - g_param_spec_string(name, nick, blurb, default, flags)
    /// - g_param_spec_int(name, nick, blurb, min, max, default, flags)
    /// - g_param_spec_object(name, nick, blurb, object_type, flags)
    pub(crate) fn from_param_spec_call(call: &CallExpression, source: &[u8]) -> Option<Self> {
        let func_name = call.function_name_str()?;

        let args = &call.arguments;

        // g_param_spec_override(name, overridden)
        if func_name == "g_param_spec_override" {
            let name = extract_string_arg(args.first()?)?;
            return Some(Self {
                name,
                nick: None,
                blurb: None,
                property_type: PropertyType::Override,
                flags: Vec::new(),
                doc: None,
            });
        }

        // Extract common arguments (name, nick, blurb)
        if args.len() < 3 {
            return None;
        }

        let name = extract_string_arg(&args[0])?;
        let nick = extract_string_arg(&args[1]);
        let blurb = extract_string_arg(&args[2]);

        let property_type = match func_name {
            "g_param_spec_string" => {
                // (name, nick, blurb, default, flags)
                PropertyType::String
            }
            "g_param_spec_boolean" => {
                // (name, nick, blurb, default, flags)
                let default = if args.len() > 3 {
                    extract_boolean_arg(&args[3]).unwrap_or(false)
                } else {
                    false
                };
                PropertyType::Boolean { default }
            }
            "g_param_spec_int" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_int_arg::<i32>(&args[3]).unwrap_or(i32::MIN)
                } else {
                    i32::MIN
                };
                let max = if args.len() > 4 {
                    extract_int_arg::<i32>(&args[4]).unwrap_or(i32::MAX)
                } else {
                    i32::MAX
                };
                let default = if args.len() > 5 {
                    extract_int_arg::<i32>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Int { min, max, default }
            }
            "g_param_spec_uint" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_uint_arg::<u32>(&args[3]).unwrap_or(0)
                } else {
                    0
                };
                let max = if args.len() > 4 {
                    extract_uint_arg::<u32>(&args[4]).unwrap_or(u32::MAX)
                } else {
                    u32::MAX
                };
                let default = if args.len() > 5 {
                    extract_uint_arg::<u32>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::UInt { min, max, default }
            }
            "g_param_spec_float" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_float_arg::<f32>(&args[3]).unwrap_or(f32::MIN)
                } else {
                    f32::MIN
                };
                let max = if args.len() > 4 {
                    extract_float_arg::<f32>(&args[4]).unwrap_or(f32::MAX)
                } else {
                    f32::MAX
                };
                let default = if args.len() > 5 {
                    extract_float_arg::<f32>(&args[5]).unwrap_or(0.0)
                } else {
                    0.0
                };
                PropertyType::Float { min, max, default }
            }
            "g_param_spec_double" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_float_arg::<f64>(&args[3]).unwrap_or(f64::MIN)
                } else {
                    f64::MIN
                };
                let max = if args.len() > 4 {
                    extract_float_arg::<f64>(&args[4]).unwrap_or(f64::MAX)
                } else {
                    f64::MAX
                };
                let default = if args.len() > 5 {
                    extract_float_arg::<f64>(&args[5]).unwrap_or(0.0)
                } else {
                    0.0
                };
                PropertyType::Double { min, max, default }
            }
            "g_param_spec_enum" => {
                // (name, nick, blurb, enum_type, default, flags)
                let enum_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                let default = if args.len() > 4 {
                    extract_int_arg::<i32>(&args[4]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Enum { enum_type, default }
            }
            "g_param_spec_flags" => {
                // (name, nick, blurb, flags_type, default, flags)
                let flags_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                let default = if args.len() > 4 {
                    extract_uint_arg::<u32>(&args[4]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Flags {
                    flags_type,
                    default,
                }
            }
            "g_param_spec_object" => {
                // (name, nick, blurb, object_type, flags)
                let object_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                PropertyType::Object { object_type }
            }
            "g_param_spec_boxed" => {
                // (name, nick, blurb, boxed_type, flags)
                let boxed_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                PropertyType::Boxed { boxed_type }
            }
            "g_param_spec_pointer" => PropertyType::Pointer,
            "g_param_spec_gtype" => {
                // (name, nick, blurb, is_a_type, flags)
                let is_a_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                PropertyType::GType { is_a_type }
            }
            "g_param_spec_int64" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_int_arg::<i64>(&args[3]).unwrap_or(i64::MIN)
                } else {
                    i64::MIN
                };
                let max = if args.len() > 4 {
                    extract_int_arg::<i64>(&args[4]).unwrap_or(i64::MAX)
                } else {
                    i64::MAX
                };
                let default = if args.len() > 5 {
                    extract_int_arg::<i64>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Int64 { min, max, default }
            }
            "g_param_spec_uint64" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_uint_arg::<u64>(&args[3]).unwrap_or(0)
                } else {
                    0
                };
                let max = if args.len() > 4 {
                    extract_uint_arg::<u64>(&args[4]).unwrap_or(u64::MAX)
                } else {
                    u64::MAX
                };
                let default = if args.len() > 5 {
                    extract_uint_arg::<u64>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::UInt64 { min, max, default }
            }
            "g_param_spec_long" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_int_arg::<c_long>(&args[3]).unwrap_or(c_long::MIN)
                } else {
                    c_long::MIN
                };
                let max = if args.len() > 4 {
                    extract_int_arg::<c_long>(&args[4]).unwrap_or(c_long::MAX)
                } else {
                    c_long::MAX
                };
                let default = if args.len() > 5 {
                    extract_int_arg::<c_long>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Long { min, max, default }
            }
            "g_param_spec_ulong" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_uint_arg::<c_ulong>(&args[3]).unwrap_or(0)
                } else {
                    0
                };
                let max = if args.len() > 4 {
                    extract_uint_arg::<c_ulong>(&args[4]).unwrap_or(c_ulong::MAX)
                } else {
                    c_ulong::MAX
                };
                let default = if args.len() > 5 {
                    extract_uint_arg::<c_ulong>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::ULong { min, max, default }
            }
            "g_param_spec_char" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_int_arg::<i8>(&args[3]).unwrap_or(i8::MIN)
                } else {
                    i8::MIN
                };
                let max = if args.len() > 4 {
                    extract_int_arg::<i8>(&args[4]).unwrap_or(i8::MAX)
                } else {
                    i8::MAX
                };
                let default = if args.len() > 5 {
                    extract_int_arg::<i8>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Char { min, max, default }
            }
            "g_param_spec_uchar" => {
                // (name, nick, blurb, min, max, default, flags)
                let min = if args.len() > 3 {
                    extract_uint_arg::<u8>(&args[3]).unwrap_or(0)
                } else {
                    0
                };
                let max = if args.len() > 4 {
                    extract_uint_arg::<u8>(&args[4]).unwrap_or(u8::MAX)
                } else {
                    u8::MAX
                };
                let default = if args.len() > 5 {
                    extract_uint_arg::<u8>(&args[5]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::UChar { min, max, default }
            }
            "g_param_spec_unichar" => {
                // (name, nick, blurb, default, flags)
                let default = if args.len() > 3 {
                    extract_uint_arg::<u32>(&args[3]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Unichar { default }
            }
            "g_param_spec_param" => {
                // (name, nick, blurb, param_type, flags)
                let param_type = args.get(3).and_then(|a| extract_gtype_arg(a, source))?;
                PropertyType::Param { param_type }
            }
            "g_param_spec_variant" => {
                // (name, nick, blurb, type, default_value, flags)
                let variant_type = args.get(3).and_then(|arg| {
                    let Argument::Expression(expr) = arg;
                    match expr.as_ref() {
                        Expression::Identifier(id) => Some(id.name.as_str()),
                        Expression::Call(call) => Some(call.function_name(source)),
                        _ => None,
                    }
                });
                let default_value = args.get(4).and_then(|arg| {
                    let Argument::Expression(e) = arg;
                    if matches!(e.as_ref(), Expression::Null(_)) {
                        None
                    } else {
                        Some(*e.clone())
                    }
                });
                PropertyType::Variant {
                    variant_type: variant_type.map(ToOwned::to_owned),
                    default_value,
                }
            }
            _ => PropertyType::Unknown {
                spec_function: func_name.to_string(),
            },
        };

        // Extract flags (usually last argument)
        let flags = if let Some(last_arg) = args.last() {
            extract_flags_arg(last_arg)
        } else {
            Vec::new()
        };

        Some(Self {
            name,
            property_type,
            nick,
            blurb,
            flags,
            doc: None,
        })
    }

    /// Extract property from g_object_class_override_property call
    /// Call signature: g_object_class_override_property(oclass, property_id,
    /// name)
    pub fn from_override_property_call(call: &CallExpression) -> Option<Self> {
        let func_name = call.function_name_str()?;
        if func_name != "g_object_class_override_property" {
            return None;
        }

        let args = &call.arguments;
        if args.len() < 3 {
            return None;
        }

        // Third argument is the property name
        let name = extract_string_arg(&args[2])?;

        Some(Self {
            name,
            property_type: PropertyType::Override,
            nick: None,
            blurb: None,
            flags: Vec::new(),
            doc: None,
        })
    }
}

// Helper functions to extract values from expression arguments

fn extract_string_arg(arg: &Argument) -> Option<String> {
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::StringLiteral(s) => {
                // Remove quotes
                let text = s.value.trim_matches('"');
                Some(text.to_owned())
            }
            Expression::Null(_) => None,
            _ => None,
        },
    }
}

fn extract_int_arg<T>(arg: &Argument) -> Option<T>
where
    T: std::str::FromStr + std::ops::Neg<Output = T>,
{
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::NumberLiteral(num) => num.value.parse().ok(),
            Expression::Unary(unary) => {
                if matches!(unary.operator, UnaryOp::Negate)
                    && let Expression::NumberLiteral(num) = &*unary.operand
                {
                    return num.value.parse::<T>().ok().map(|v| -v);
                }
                None
            }
            _ => None,
        },
    }
}

fn extract_uint_arg<T: std::str::FromStr>(arg: &Argument) -> Option<T> {
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::NumberLiteral(num) => num.value.parse().ok(),
            _ => None,
        },
    }
}

fn extract_float_arg<T>(arg: &Argument) -> Option<T>
where
    T: std::str::FromStr + std::ops::Neg<Output = T>,
{
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::NumberLiteral(num) => num.value.parse().ok(),
            Expression::Unary(unary) => {
                if matches!(unary.operator, UnaryOp::Negate)
                    && let Expression::NumberLiteral(num) = &*unary.operand
                {
                    return num.value.parse::<T>().ok().map(|v| -v);
                }
                None
            }
            _ => None,
        },
    }
}

fn extract_boolean_arg(arg: &Argument) -> Option<bool> {
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::Boolean(b) => Some(b.value),
            Expression::Identifier(id) => {
                // TRUE/FALSE macros
                match id.name.as_str() {
                    "TRUE" | "true" => Some(true),
                    "FALSE" | "false" => Some(false),
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

fn extract_gtype_arg(arg: &Argument, source: &[u8]) -> Option<GType> {
    let Argument::Expression(expr) = arg;
    GType::from_expression(expr, source)
}

fn extract_flags_arg(arg: &Argument) -> Vec<ParamFlag> {
    let Argument::Expression(expr) = arg;
    let mut flags = Vec::new();
    expr.walk(&mut |e| {
        if let Expression::Identifier(id) = e {
            flags.push(ParamFlag::from_identifier(&id.name));
        }
    });
    flags
}

/// Information about a param_spec assignment found in a class_init function
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamSpecAssignment {
    /// Array subscript pattern: props[PROP_X] = g_param_spec_*()
    ArraySubscript {
        array_name: String,
        enum_value: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
        install_call: Option<CallExpression>,
    },
    /// Variable pattern: param_spec = g_param_spec_*()
    Variable {
        variable_name: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
        install_call: Option<CallExpression>,
    },
    /// Override property pattern: g_object_class_override_property(class,
    /// PROP_X, "name")
    OverrideProperty {
        enum_value: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
    },
    /// Direct install: g_object_class_install_property(class, PROP_X,
    /// g_param_spec_xxx(...))
    DirectInstall {
        enum_value: String,
        statement_location: SourceLocation,
        /// The inline g_param_spec_* call
        call: CallExpression,
        property: Property,
        /// The enclosing g_object_class_install_property call
        install_call: CallExpression,
    },
}

impl ParamSpecAssignment {
    /// Check if this param_spec assignment is installed (has an install call or
    /// is an override)
    pub fn is_installed(&self) -> bool {
        match self {
            Self::ArraySubscript { install_call, .. } => install_call.is_some(),
            Self::OverrideProperty { .. } => true,
            Self::Variable { install_call, .. } => install_call.is_some(),
            Self::DirectInstall { .. } => true,
        }
    }

    /// Get the enum value if this assignment is installed
    /// For Variable assignments, extracts enum value from install_property call
    pub fn get_installed_enum_value<'a>(&'a self, source: &'a [u8]) -> Option<&'a str> {
        match self {
            Self::ArraySubscript {
                enum_value,
                install_call,
                ..
            } => install_call.as_ref().map(|_| enum_value.as_str()),
            Self::OverrideProperty { enum_value, .. } => Some(enum_value),
            Self::Variable { install_call, .. } => install_call
                .as_ref()
                .and_then(|call| call.get_arg(1).and_then(|arg| arg.to_source_string(source))),
            Self::DirectInstall { enum_value, .. } => Some(enum_value),
        }
    }

    /// Get the property information
    pub fn property(&self) -> &Property {
        match self {
            Self::ArraySubscript { property, .. } => property,
            Self::Variable { property, .. } => property,
            Self::OverrideProperty { property, .. } => property,
            Self::DirectInstall { property, .. } => property,
        }
    }

    /// Get the g_param_spec_* call (None for OverrideProperty)
    pub fn param_spec_call(&self) -> Option<&CallExpression> {
        match self {
            Self::ArraySubscript { call, .. }
            | Self::Variable { call, .. }
            | Self::DirectInstall { call, .. } => Some(call),
            Self::OverrideProperty { .. } => None,
        }
    }

    /// Get the enum value (for ArraySubscript, OverrideProperty, and
    /// DirectInstall)
    pub fn enum_value(&self) -> Option<&str> {
        match self {
            Self::ArraySubscript { enum_value, .. } => Some(enum_value.as_str()),
            Self::OverrideProperty { enum_value, .. } => Some(enum_value.as_str()),
            Self::DirectInstall { enum_value, .. } => Some(enum_value.as_str()),
            Self::Variable { .. } => None,
        }
    }
}
