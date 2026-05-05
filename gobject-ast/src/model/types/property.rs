use std::ffi::{c_long, c_ulong};

use serde::{Deserialize, Serialize};

use crate::model::{
    SourceLocation,
    expression::{Argument, CallExpression, Expression},
    operators::UnaryOp,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            "G_PARAM_READABLE" => ParamFlag::Readable,
            "G_PARAM_WRITABLE" => ParamFlag::Writable,
            "G_PARAM_READWRITE" => ParamFlag::ReadWrite,
            "G_PARAM_CONSTRUCT" => ParamFlag::Construct,
            "G_PARAM_CONSTRUCT_ONLY" => ParamFlag::ConstructOnly,
            "G_PARAM_LAX_VALIDATION" => ParamFlag::LaxValidation,
            "G_PARAM_STATIC_NAME" => ParamFlag::StaticName,
            "G_PARAM_PRIVATE" => ParamFlag::Private,
            "G_PARAM_STATIC_NICK" => ParamFlag::StaticNick,
            "G_PARAM_STATIC_BLURB" => ParamFlag::StaticBlurb,
            "G_PARAM_STATIC_STRINGS" => ParamFlag::StaticStrings,
            "G_PARAM_EXPLICIT_NOTIFY" => ParamFlag::ExplicitNotify,
            "G_PARAM_DEPRECATED" => ParamFlag::Deprecated,
            _ => ParamFlag::Unknown(name.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ParamFlag::Readable => "G_PARAM_READABLE",
            ParamFlag::Writable => "G_PARAM_WRITABLE",
            ParamFlag::ReadWrite => "G_PARAM_READWRITE",
            ParamFlag::Construct => "G_PARAM_CONSTRUCT",
            ParamFlag::ConstructOnly => "G_PARAM_CONSTRUCT_ONLY",
            ParamFlag::LaxValidation => "G_PARAM_LAX_VALIDATION",
            ParamFlag::StaticName => "G_PARAM_STATIC_NAME",
            ParamFlag::Private => "G_PARAM_PRIVATE",
            ParamFlag::StaticNick => "G_PARAM_STATIC_NICK",
            ParamFlag::StaticBlurb => "G_PARAM_STATIC_BLURB",
            ParamFlag::StaticStrings => "G_PARAM_STATIC_STRINGS",
            ParamFlag::ExplicitNotify => "G_PARAM_EXPLICIT_NOTIFY",
            ParamFlag::Deprecated => "G_PARAM_DEPRECATED",
            ParamFlag::Unknown(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub property_type: PropertyType,
    pub nick: Option<String>,
    pub blurb: Option<String>,
    pub flags: Vec<ParamFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        param_type: String,
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
        enum_type: String,
        default: i32,
    },
    Flags {
        flags_type: String,
        default: u32,
    },
    Object {
        object_type: String,
    },
    Boxed {
        boxed_type: String,
    },
    Pointer,
    GType {
        is_a_type: String,
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
    pub fn from_param_spec_call(call: &CallExpression) -> Option<Self> {
        let func_name = call.function_name_str()?;

        // Extract common arguments (name, nick, blurb)
        let args = &call.arguments;
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
                let enum_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
                let default = if args.len() > 4 {
                    extract_int_arg::<i32>(&args[4]).unwrap_or(0)
                } else {
                    0
                };
                PropertyType::Enum { enum_type, default }
            }
            "g_param_spec_flags" => {
                // (name, nick, blurb, flags_type, default, flags)
                let flags_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
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
                let object_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
                PropertyType::Object { object_type }
            }
            "g_param_spec_boxed" => {
                // (name, nick, blurb, boxed_type, flags)
                let boxed_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
                PropertyType::Boxed { boxed_type }
            }
            "g_param_spec_pointer" => PropertyType::Pointer,
            "g_param_spec_gtype" => {
                // (name, nick, blurb, is_a_type, flags)
                let is_a_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
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
                let param_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3]).unwrap_or_default()
                } else {
                    String::new()
                };
                PropertyType::Param { param_type }
            }
            "g_param_spec_variant" => {
                // (name, nick, blurb, type, default_value, flags)
                let variant_type = if args.len() > 3 {
                    extract_identifier_arg(&args[3])
                } else {
                    None
                };
                let default_value = args.get(4).and_then(|arg| {
                    let Argument::Expression(e) = arg;
                    if matches!(e.as_ref(), Expression::Null(_)) {
                        None
                    } else {
                        Some(*e.clone())
                    }
                });
                PropertyType::Variant {
                    variant_type,
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

        Some(Property {
            name,
            property_type,
            nick,
            blurb,
            flags,
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

        Some(Property {
            name,
            property_type: PropertyType::Override,
            nick: None,
            blurb: None,
            flags: Vec::new(),
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

fn extract_identifier_arg(arg: &Argument) -> Option<String> {
    match arg {
        Argument::Expression(boxed_expr) => match &**boxed_expr {
            Expression::Identifier(id) => Some(id.name.clone()),
            Expression::Call(call) => {
                // Handle macros like G_TYPE_STRING
                Some(call.function_name())
            }
            _ => None,
        },
    }
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
#[derive(Debug, Clone)]
pub enum ParamSpecAssignment {
    /// Array subscript pattern: props[PROP_X] = g_param_spec_*()
    ArraySubscript {
        array_name: String,
        enum_value: String,
        property_name: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
        install_call: Option<CallExpression>,
    },
    /// Variable pattern: param_spec = g_param_spec_*()
    Variable {
        variable_name: String,
        property_name: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
        install_call: Option<CallExpression>,
    },
    /// Override property pattern: g_object_class_override_property(class,
    /// PROP_X, "name")
    OverrideProperty {
        enum_value: String,
        property_name: String,
        statement_location: SourceLocation,
        call: CallExpression,
        property: Property,
    },
    /// Direct install: g_object_class_install_property(class, PROP_X,
    /// g_param_spec_xxx(...))
    DirectInstall {
        enum_value: String,
        property_name: String,
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
            ParamSpecAssignment::ArraySubscript { install_call, .. } => install_call.is_some(),
            ParamSpecAssignment::OverrideProperty { .. } => true,
            ParamSpecAssignment::Variable { install_call, .. } => install_call.is_some(),
            ParamSpecAssignment::DirectInstall { .. } => true,
        }
    }

    /// Get the enum value if this assignment is installed
    /// For Variable assignments, extracts enum value from install_property call
    pub fn get_installed_enum_value(&self, source: &[u8]) -> Option<String> {
        match self {
            ParamSpecAssignment::ArraySubscript {
                enum_value,
                install_call,
                ..
            } => install_call.as_ref().map(|_| enum_value.clone()),
            ParamSpecAssignment::OverrideProperty { enum_value, .. } => Some(enum_value.clone()),
            ParamSpecAssignment::Variable { install_call, .. } => install_call
                .as_ref()
                .and_then(|call| call.get_arg(1).and_then(|arg| arg.to_source_string(source))),
            ParamSpecAssignment::DirectInstall { enum_value, .. } => Some(enum_value.clone()),
        }
    }

    /// Get the property information
    pub fn property(&self) -> &Property {
        match self {
            ParamSpecAssignment::ArraySubscript { property, .. } => property,
            ParamSpecAssignment::Variable { property, .. } => property,
            ParamSpecAssignment::OverrideProperty { property, .. } => property,
            ParamSpecAssignment::DirectInstall { property, .. } => property,
        }
    }

    /// Get the enum value (for ArraySubscript, OverrideProperty, and
    /// DirectInstall)
    pub fn enum_value(&self) -> Option<&str> {
        match self {
            ParamSpecAssignment::ArraySubscript { enum_value, .. } => Some(enum_value.as_str()),
            ParamSpecAssignment::OverrideProperty { enum_value, .. } => Some(enum_value.as_str()),
            ParamSpecAssignment::DirectInstall { enum_value, .. } => Some(enum_value.as_str()),
            ParamSpecAssignment::Variable { .. } => None,
        }
    }
}
