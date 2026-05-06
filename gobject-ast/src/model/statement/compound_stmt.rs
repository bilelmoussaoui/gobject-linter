use serde::Serialize;

use crate::model::{SourceLocation, Statement};

#[derive(Debug, Clone, Serialize)]
pub struct CompoundStatement {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub statements: Vec<Statement>,
    pub location: SourceLocation,
}
