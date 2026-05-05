use serde::{Deserialize, Serialize};

use crate::model::{SourceLocation, TypeInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Parameter {
    Regular {
        name: Option<String>,
        type_info: TypeInfo,
        location: SourceLocation,
    },
    Variadic,
}
