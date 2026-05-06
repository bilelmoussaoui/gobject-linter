use serde::Serialize;

use crate::model::SourceLocation;

#[derive(Debug, Clone, Serialize)]
pub struct IdentifierExpression {
    pub name: String,
    pub location: SourceLocation,
}
