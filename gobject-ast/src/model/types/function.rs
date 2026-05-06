use serde::Serialize;

use crate::model::{SourceLocation, TypeInfo};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Parameter {
    Regular {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        type_info: TypeInfo,
        location: SourceLocation,
    },
    Variadic,
}
