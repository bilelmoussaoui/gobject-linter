use clap::ValueEnum;
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::model::{
    Comment, Expression, FunctionDeclItem, FunctionDefItem, GObjectType, SourceLocation, Statement,
    TypeDefItem, VariableDecl,
};

/// Coarse kind of a top-level item, useful for filtering without
/// pattern-matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TopLevelItemKind {
    FunctionDefinition,
    FunctionDeclaration,
    Typedef,
    Struct,
    Enum,
    Include,
    Define,
    GObjectType,
    Conditional,
    GObjectDeclsBlock,
    Declaration,
    Expression,
    Other,
}

/// Represents a top-level item in a C file
#[derive(Debug, Clone)]
pub enum TopLevelItem {
    /// Preprocessor directive (#define, #include, etc.)
    Preprocessor(PreprocessorDirective),
    /// Type definition (typedef, enum, struct)
    TypeDefinition(TypeDefItem),
    /// Function declaration (forward declaration)
    FunctionDeclaration(FunctionDeclItem),
    /// Function definition (with body)
    FunctionDefinition(FunctionDefItem),
    /// Standalone variable declaration
    Declaration(Box<VariableDecl>),
    /// Standalone expression statement
    Expression(Box<Expression>),
    /// Standalone GTK-Doc comment not attached to any declaration
    Comment(Comment),
}

impl TopLevelItem {
    /// The name associated with this item, if any.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::FunctionDefinition(f) => Some(&f.name),
            Self::FunctionDeclaration(f) => Some(&f.name),
            Self::TypeDefinition(td) => match td {
                TypeDefItem::Typedef { name, .. } | TypeDefItem::Struct { name, .. } => Some(name),
                TypeDefItem::Enum(enum_info) => enum_info.name.as_deref(),
            },
            Self::Preprocessor(PreprocessorDirective::Include { path, .. }) => Some(path),
            Self::Preprocessor(PreprocessorDirective::Define { name, .. }) => Some(name),
            Self::Preprocessor(PreprocessorDirective::GObjectType(gobject_type)) => {
                Some(&gobject_type.type_name)
            }
            Self::Declaration(decl) => Some(&decl.name),
            Self::Comment(_) => None,
            _ => None,
        }
    }

    /// The coarse kind of this item.
    pub fn kind(&self) -> TopLevelItemKind {
        match self {
            Self::FunctionDefinition(_) => TopLevelItemKind::FunctionDefinition,
            Self::FunctionDeclaration(_) => TopLevelItemKind::FunctionDeclaration,
            Self::TypeDefinition(TypeDefItem::Typedef { .. }) => TopLevelItemKind::Typedef,
            Self::TypeDefinition(TypeDefItem::Struct { .. }) => TopLevelItemKind::Struct,
            Self::TypeDefinition(TypeDefItem::Enum(_)) => TopLevelItemKind::Enum,
            Self::Preprocessor(PreprocessorDirective::Include { .. }) => TopLevelItemKind::Include,
            Self::Preprocessor(PreprocessorDirective::Define { .. }) => TopLevelItemKind::Define,
            Self::Preprocessor(PreprocessorDirective::GObjectType { .. }) => {
                TopLevelItemKind::GObjectType
            }
            Self::Preprocessor(PreprocessorDirective::Conditional { .. }) => {
                TopLevelItemKind::Conditional
            }
            Self::Preprocessor(PreprocessorDirective::GObjectDeclsBlock { .. }) => {
                TopLevelItemKind::GObjectDeclsBlock
            }
            Self::Declaration(_) => TopLevelItemKind::Declaration,
            Self::Expression(_) => TopLevelItemKind::Expression,
            Self::Comment(_) => TopLevelItemKind::Other,
            Self::Preprocessor(_) => TopLevelItemKind::Other,
        }
    }

    pub fn has_doc(&self) -> bool {
        match self {
            Self::FunctionDefinition(f) => f.doc.is_some(),
            Self::FunctionDeclaration(f) => f.doc.is_some(),
            Self::TypeDefinition(td) => match td {
                TypeDefItem::Struct { doc, .. } | TypeDefItem::Typedef { doc, .. } => doc.is_some(),
                TypeDefItem::Enum(e) => e.doc.is_some(),
            },
            _ => false,
        }
    }

    pub(crate) fn is_comment_at_byte(&self, byte: usize) -> bool {
        matches!(self, Self::Comment(c) if c.location.start_byte == byte)
    }
}

impl Serialize for TopLevelItem {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Preprocessor(directive) => directive.serialize(s),
            Self::TypeDefinition(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("type_definition", v)?;
                m.end()
            }
            Self::FunctionDeclaration(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("function_declaration", v)?;
                m.end()
            }
            Self::FunctionDefinition(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("function_definition", v)?;
                m.end()
            }
            Self::Declaration(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("declaration", v)?;
                m.end()
            }
            Self::Expression(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("expression", v)?;
                m.end()
            }
            Self::Comment(v) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("comment", v)?;
                m.end()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PragmaKind {
    /// #pragma once
    Once,
    /// #pragma GCC/clang diagnostic push
    DiagnosticPush,
    /// #pragma GCC/clang diagnostic pop
    DiagnosticPop,
    /// #pragma GCC/clang diagnostic ignored "-Wwarning-name"
    DiagnosticIgnored { warning: String },
    /// Other pragma directive
    Other {
        name: String,
        arguments: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreprocessorDirective {
    Include {
        path: String,
        is_system: bool,
        location: SourceLocation,
    },
    Define {
        name: String,
        value: Option<String>,
        location: SourceLocation,
    },
    Call {
        directive: String,
        location: SourceLocation,
    },
    Pragma {
        kind: PragmaKind,
        location: SourceLocation,
    },
    /// GObject type declaration/definition (G_DECLARE_*, G_DEFINE_*)
    GObjectType(Box<GObjectType>),
    /// G_DEFINE_AUTOPTR_CLEANUP_FUNC (Type, cleanup_func)
    AutoptrCleanupFunc {
        type_name: String,
        cleanup_function: String,
        location: SourceLocation,
    },
    /// G_DEFINE_AUTO_CLEANUP_CLEAR_FUNC (Type, cleanup_func)
    AutoCleanupClearFunc {
        type_name: String,
        cleanup_function: String,
        location: SourceLocation,
    },
    /// Macro call with code block (e.g., G_DEFINE_BOXED_TYPE_WITH_CODE)
    /// Contains the macro name and parsed statements from the code block
    MacroWithCode {
        macro_name: String,
        arguments: Vec<String>,
        code_statements: Vec<Statement>,
        location: SourceLocation,
    },
    Conditional {
        kind: ConditionalKind,
        condition: Option<String>,
        body: Vec<TopLevelItem>,
        location: SourceLocation,
    },
    /// G_BEGIN_DECLS ... G_END_DECLS block
    GObjectDeclsBlock {
        body: Vec<TopLevelItem>,
        location: SourceLocation,
    },
}

impl PreprocessorDirective {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::Include { location, .. }
            | Self::Define { location, .. }
            | Self::Call { location, .. }
            | Self::Pragma { location, .. }
            | Self::AutoptrCleanupFunc { location, .. }
            | Self::AutoCleanupClearFunc { location, .. }
            | Self::MacroWithCode { location, .. }
            | Self::Conditional { location, .. }
            | Self::GObjectDeclsBlock { location, .. } => location,
            Self::GObjectType(gt) => &gt.location,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalKind {
    Ifdef,
    Ifndef,
    If,
    Elif,
    Else,
}
