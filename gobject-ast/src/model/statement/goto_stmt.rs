use serde::Serialize;

use crate::model::SourceLocation;

#[derive(Debug, Clone, Serialize)]
pub struct GotoStatement {
    pub label: String,
    pub location: SourceLocation,
}
