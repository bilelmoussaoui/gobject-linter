use serde::Serialize;

use crate::model::SourceLocation;

#[derive(Debug, Clone, Serialize)]
pub struct BreakStatement {
    pub location: SourceLocation,
}
