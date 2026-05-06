use serde::Serialize;

use crate::model::{SourceLocation, Statement};

#[derive(Debug, Clone, Serialize)]
pub struct LabeledStatement {
    pub label: String,
    pub statement: Box<Statement>,
    pub location: SourceLocation,
}
