use serde::Serialize;

use crate::SourceLocation;

#[derive(Debug, Clone, Serialize)]
pub struct Include {
    pub path: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_system: bool, // <> vs ""
    pub location: SourceLocation,
}
