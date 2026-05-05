use serde::{Deserialize, Serialize};

use crate::SourceLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: Option<String>,
    pub location: SourceLocation,
    pub values: Vec<EnumValue>,
    /// Location of the enum body for inserting fixes
    pub body_location: SourceLocation,
    /// Attributes between closing brace and type name (e.g., G_GNUC_FLAG_ENUM)
    pub attributes: Vec<String>,
}

impl EnumInfo {
    pub fn is_property_enum(&self) -> bool {
        self.values
            .iter()
            .any(|v| v.name.contains("_PROP_") || v.name.starts_with("PROP_"))
    }

    pub fn is_signal_enum(&self) -> bool {
        self.values
            .iter()
            .any(|v| v.name.contains("_SIGNAL_") || v.name.starts_with("SIGNAL_"))
    }

    /// Check if this appears to be a flags enum (bit flags pattern)
    /// based on bit shift operations or power-of-two values.
    pub fn is_flags_enum(&self) -> bool {
        if self.values.is_empty() {
            return false;
        }

        // Collect all explicitly-assigned numeric values.
        let explicit_values: Vec<i64> = self
            .values
            .iter()
            .filter(|v| v.value.is_some())
            .filter_map(|v| v.value)
            .collect();

        // If the values with explicit assignments form a consecutive integer
        // sequence (0,1,2,… or 1,2,3,…) they are almost certainly a plain
        // enumeration, not bit flags.  Reject early to avoid false positives
        // on e.g. { FORBID=0, ALLOW=1, IGNORE=2 }.
        if explicit_values.len() >= 2 {
            let mut sorted = explicit_values.clone();
            sorted.sort_unstable();
            let is_consecutive = sorted.windows(2).all(|w| w[1] == w[0] + 1);
            if is_consecutive {
                return false;
            }
        }

        let mut flag_like_count = 0;
        let mut total_count = 0;

        for value in &self.values {
            // Skip values without explicit values (auto-incremented)
            if value.value_expr.is_none() {
                continue;
            }

            total_count += 1;

            // Check if the value expression is a bit shift operation
            let is_bit_shift = value
                .value_expr
                .as_ref()
                .map(is_bit_shift_expr)
                .unwrap_or(false);

            // Check if the evaluated value is a power of 2 (or 0)
            let is_power_of_two = value
                .value
                .map(|num| num == 0 || (num > 0 && (num & (num - 1)) == 0))
                .unwrap_or(false);

            if is_bit_shift || is_power_of_two {
                flag_like_count += 1;
            }
        }

        // Consider it a flags enum if most values (>= 80%) are flag-like
        // and there are at least 2 explicit values
        total_count >= 2 && flag_like_count * 100 / total_count >= 80
    }

    /// Check if this enum has a specific attribute (e.g., G_GNUC_FLAG_ENUM)
    pub fn has_attribute(&self, attr_name: &str) -> bool {
        self.attributes.iter().any(|attr| attr == attr_name)
    }
}

/// Check if an expression is a bit shift operation (e.g., 1 << 0)
fn is_bit_shift_expr(expr: &super::super::Expression) -> bool {
    use super::super::operators::BinaryOp;

    match expr {
        super::super::Expression::Binary(bin) => matches!(bin.operator, BinaryOp::LeftShift),
        _ => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<i64>,
    /// The expression AST for the value (if present)
    pub value_expr: Option<super::super::Expression>,
    /// Location of this enumerator node
    pub location: SourceLocation,
    /// Location of just the name
    pub name_location: SourceLocation,
    /// Location of the value (if present)
    pub value_location: Option<SourceLocation>,
}

impl EnumValue {
    /// Check if this is a PROP_0 sentinel (PROP_0, *_PROP_0, etc.)
    pub fn is_prop_0(&self) -> bool {
        self.name.ends_with("_PROP_0")
            || self.name == "PROP_0"
            || (self.name.starts_with("PROP_") && self.name.ends_with("_0"))
            || self.name.ends_with("_PROP_ZERO")
            || self.name == "PROP_ZERO"
            || self.name.ends_with("_ROW_PROP_0")
            || self.name == "ROW_PROP_0"
            || self.name.ends_with("_CHILD_PROP_0")
            || self.name == "CHILD_PROP_0"
    }

    /// Check if this is a property count sentinel (N_PROPS, PROP_LAST,
    /// NUM_PROPERTIES, etc.)
    pub fn is_prop_last(&self) -> bool {
        // Sentinels ending with count/last indicators
        self.name.ends_with("_N_PROPS")
            || self.name == "N_PROPS"
            || self.name.ends_with("_PROP_LAST")
            || self.name == "PROP_LAST"
            || self.name.ends_with("_NUM_PROPERTIES")
            || self.name == "NUM_PROPERTIES"
            || self.name == "N_PROPERTIES"
            || self.name.ends_with("_NUM_PROPS")
            // Patterns like LAST_PROP, LAST_FOO_PROP, LAST_PROPERTY, LAST_ROW_PROPERTY
            || (self.name.starts_with("LAST_") && (self.name.ends_with("_PROP") || self.name.ends_with("_PROPERTY")))
            || self.name == "LAST_PROP"
            || self.name == "LAST_PROPERTY"
            // Patterns like N_FOO_PROPS or NUM_FOO_PROPS
            || (self.name.starts_with("N_") && self.name.ends_with("_PROPS"))
            || (self.name.starts_with("N_") && self.name.ends_with("_PROPERTIES"))
            || (self.name.starts_with("NUM_") && self.name.ends_with("_PROPS"))
    }

    /// Check if this is a signal count sentinel (N_SIGNALS, LAST_SIGNAL,
    /// NUM_SIGNALS, etc.)
    pub fn is_signal_last(&self) -> bool {
        self.name.ends_with("_N_SIGNALS")
            || self.name == "N_SIGNALS"
            || self.name.ends_with("_SIGNAL_LAST")
            || self.name == "SIGNAL_LAST"
            || self.name.ends_with("_LAST_SIGNAL")
            || self.name == "LAST_SIGNAL"
            || self.name.ends_with("_NUM_SIGNALS")
            || self.name == "NUM_SIGNALS"
            || (self.name.starts_with("LAST_") && self.name.ends_with("_SIGNAL"))
            || (self.name.starts_with("N_") && self.name.ends_with("_SIGNALS"))
            || (self.name.starts_with("NUM_") && self.name.ends_with("_SIGNALS"))
    }

    /// Extract the value text from source (e.g., for `N_PROPS =
    /// PROP_ORIENTATION`, returns "PROP_ORIENTATION")
    pub fn value_text<'a>(&self, source: &'a [u8]) -> Option<&'a str> {
        self.value_location
            .as_ref()
            .and_then(|loc| loc.as_str(source))
            .map(|s| s.trim())
    }
}
