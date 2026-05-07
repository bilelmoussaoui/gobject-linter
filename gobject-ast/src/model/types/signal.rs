use serde::Serialize;

use crate::model::{
    doc::SignalDoc,
    expression::{CallExpression, Expression, StructField},
    types::GType,
};

/// Represents a GObject signal registration
///
/// Parsed from g_signal_new/g_signal_newv calls:
/// ```c
/// g_signal_new (const gchar *signal_name,
///               GType itype,
///               GSignalFlags signal_flags,
///               guint class_offset,
///               GSignalAccumulator accumulator,
///               gpointer accu_data,
///               GSignalCMarshaller c_marshaller,
///               GType return_type,
///               guint n_params,
///               ...);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Signal {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<SignalDoc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itype: Option<String>, // G_TYPE_FROM_CLASS(klass), G_OBJECT_TYPE, etc.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<SignalFlag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_offset: Option<StructField>, // None when 0 (no default handler)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accumulator: Option<String>, // function name or NULL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accu_data: Option<String>, // data or NULL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_marshaller: Option<String>, // marshaller or NULL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<GType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_params: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub param_types: Vec<GType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalFlag {
    RunFirst,
    RunLast,
    RunCleanup,
    NoRecurse,
    Detailed,
    Action,
    NoHooks,
    MustCollect,
    Deprecated,
    AccumulatorFirstRun,
    Unknown(String),
}

impl SignalFlag {
    pub fn from_identifier(s: &str) -> Self {
        match s {
            "G_SIGNAL_RUN_FIRST" => Self::RunFirst,
            "G_SIGNAL_RUN_LAST" => Self::RunLast,
            "G_SIGNAL_RUN_CLEANUP" => Self::RunCleanup,
            "G_SIGNAL_NO_RECURSE" => Self::NoRecurse,
            "G_SIGNAL_DETAILED" => Self::Detailed,
            "G_SIGNAL_ACTION" => Self::Action,
            "G_SIGNAL_NO_HOOKS" => Self::NoHooks,
            "G_SIGNAL_MUST_COLLECT" => Self::MustCollect,
            "G_SIGNAL_DEPRECATED" => Self::Deprecated,
            "G_SIGNAL_ACCUMULATOR_FIRST_RUN" => Self::AccumulatorFirstRun,
            _ => Self::Unknown(s.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::RunFirst => "G_SIGNAL_RUN_FIRST",
            Self::RunLast => "G_SIGNAL_RUN_LAST",
            Self::RunCleanup => "G_SIGNAL_RUN_CLEANUP",
            Self::NoRecurse => "G_SIGNAL_NO_RECURSE",
            Self::Detailed => "G_SIGNAL_DETAILED",
            Self::Action => "G_SIGNAL_ACTION",
            Self::NoHooks => "G_SIGNAL_NO_HOOKS",
            Self::MustCollect => "G_SIGNAL_MUST_COLLECT",
            Self::Deprecated => "G_SIGNAL_DEPRECATED",
            Self::AccumulatorFirstRun => "G_SIGNAL_ACCUMULATOR_FIRST_RUN",
            Self::Unknown(s) => s,
        }
    }
}

impl Signal {
    /// Extract a signal from a g_signal_new* function call
    ///
    /// ```c
    /// g_signal_new ("changed",
    ///               G_TYPE_FROM_CLASS (klass),
    ///               G_SIGNAL_RUN_LAST,
    ///               G_STRUCT_OFFSET (MyObjectClass, changed),
    ///               NULL, NULL, NULL,
    ///               G_TYPE_NONE,
    ///               0);
    /// ```
    pub fn from_g_signal_new_call(call: &CallExpression, source: &[u8]) -> Option<Self> {
        if !call.function_name().starts_with("g_signal_new") {
            return None;
        }

        // Argument 0: signal_name (string literal)
        let name = call.extract_string_from_arg(0)?;

        // Argument 1: itype (GType expression) - use source text
        let itype = call.get_arg_text(1, source);

        // Argument 2: signal_flags (can be bitwise OR of multiple flags)
        let flags = call
            .get_arg(2)
            .map(extract_signal_flags)
            .unwrap_or_default();

        // Argument 3: class_offset — G_STRUCT_OFFSET(StructType, field) or 0
        let class_offset = call.get_arg(3).and_then(|expr| match expr {
            Expression::Call(offset_call) if offset_call.function_name() == "G_STRUCT_OFFSET" => {
                let struct_type = offset_call.get_arg(0).and_then(|e| {
                    if let Expression::Identifier(id) = e {
                        Some(id.name.clone())
                    } else {
                        None
                    }
                })?;
                let field = offset_call.get_arg(1).and_then(|e| {
                    if let Expression::Identifier(id) = e {
                        Some(id.name.clone())
                    } else {
                        None
                    }
                })?;
                Some(StructField { struct_type, field })
            }
            _ => None, // 0, NULL, or any other form → no vtable slot
        });

        // Argument 4: accumulator (function pointer or NULL)
        let accumulator = call.get_arg_text(4, source);

        // Argument 5: accu_data (gpointer or NULL)
        let accu_data = call.get_arg_text(5, source);

        // Argument 6: c_marshaller (function pointer or NULL)
        let c_marshaller = call.get_arg_text(6, source);

        // Argument 7: return_type (GType)
        let return_type = call.get_arg(7).and_then(GType::from_expression);

        // Argument 8: n_params (guint)
        let n_params = call.get_arg(8).and_then(|expr| match expr {
            Expression::NumberLiteral(n) => n.value.parse::<i64>().ok(),
            _ => None,
        });

        // Arguments 9+: parameter types (variadic)
        let param_types = (9..call.arguments.len())
            .filter_map(|i| call.get_arg(i).and_then(GType::from_expression))
            .collect();

        Some(Self {
            name,
            enum_value: None,
            doc: None,
            itype,
            flags,
            class_offset,
            accumulator,
            accu_data,
            c_marshaller,
            return_type,
            n_params,
            param_types,
        })
    }
}

/// Extract signal flags from an expression (handles bitwise OR)
fn extract_signal_flags(expr: &Expression) -> Vec<SignalFlag> {
    let mut flags = Vec::new();

    // Walk the expression tree to find all flag identifiers
    // This handles simple cases like G_SIGNAL_RUN_LAST
    // and complex cases like G_SIGNAL_RUN_FIRST | G_SIGNAL_ACTION
    expr.walk(&mut |e| {
        if let Expression::Identifier(id) = e
            && id.name.starts_with("G_SIGNAL_")
        {
            flags.push(SignalFlag::from_identifier(&id.name));
        }
    });

    flags
}
