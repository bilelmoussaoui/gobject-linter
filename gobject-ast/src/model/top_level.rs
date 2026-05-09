use clap::ValueEnum;
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    Comment, EnumInfo, GObjectType, TypeInfo, VariableDecl, VirtualFunction,
    model::{SourceLocation, Statement, doc::TypeDoc, expression::Expression, types::Parameter},
    types::{FunctionDeclItem, FunctionDefItem},
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

/// A parsed field from a struct body (e.g. `GObject parent` → field_type =
/// "GObject")
#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub field_type: TypeInfo,
    /// Field name, if present (anonymous bitfields have none)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    pub location: SourceLocation,
    /// Bit-width for bitfield members (`unsigned flags : 1` → `Some(1)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_width: Option<u32>,
    /// Non-empty for anonymous struct/union fields: the members of the
    /// embedded aggregate (e.g. `union { A a; B b; } d` → inner_fields = [a,
    /// b]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_fields: Vec<Self>,
}

impl StructField {
    /// True for future-use padding fields that should never be flagged as dead
    /// code: names starting with `rfu`, `reserved`, `padding`, or `_padding`.
    pub fn is_reserved(&self) -> bool {
        self.field_name.as_deref().is_some_and(|n| {
            n.starts_with("rfu")
                || n.starts_with("reserved")
                || n.starts_with("padding")
                || n.starts_with("_padding")
        })
    }

    /// Visit this field and all nested fields (anonymous struct/union members)
    /// in pre-order, matching the pattern used by `Statement::walk`.
    pub fn walk<F>(&self, f: &mut F)
    where
        F: FnMut(&Self),
    {
        f(self);
        for inner in &self.inner_fields {
            inner.walk(f);
        }
    }
}

/// The right-hand side of a typedef declaration.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedefTarget {
    /// Plain type alias: `typedef struct _Foo Foo`, `typedef gint MyInt`.
    Type(TypeInfo),
    /// Function-pointer alias: `typedef void (*FooCallback)(GObject *,
    /// gpointer)`.
    Callback {
        return_type: TypeInfo,
        parameters: Vec<Parameter>,
    },
}

impl TypedefTarget {
    /// Return the inner `TypeInfo` if this is a plain type alias.
    pub fn as_type(&self) -> Option<&TypeInfo> {
        match self {
            Self::Type(t) => Some(t),
            Self::Callback { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeDefItem {
    Typedef {
        name: String,
        target: TypedefTarget,
        /// Fields when the typedef wraps an inline struct body:
        /// `typedef struct { FieldType field; } Name;`
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        struct_fields: Vec<StructField>,
        location: SourceLocation,
        #[serde(skip_serializing_if = "Option::is_none")]
        doc: Option<TypeDoc>,
    },
    Struct {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fields: Vec<StructField>,
        /// Virtual functions (function pointer fields) extracted from class
        /// structs (structs whose name ends with `Class`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        vfuncs: Vec<VirtualFunction>,
        location: SourceLocation,
        #[serde(skip_serializing_if = "Option::is_none")]
        doc: Option<TypeDoc>,
    },
    Enum(Box<EnumInfo>),
}

impl TypeDefItem {
    /// True for GObject class/interface vtable structs whose fields should not
    /// be checked for dead code: any struct with vfuncs, or any type whose
    /// bare name ends with `Class` or `Interface`.
    pub fn is_vtable_struct(&self) -> bool {
        match self {
            Self::Struct { name, vfuncs, .. } => {
                let bare = name.trim_start_matches('_');
                bare.ends_with("Class") || bare.ends_with("Interface") || !vfuncs.is_empty()
            }
            Self::Typedef { name, .. } => name.ends_with("Class") || name.ends_with("Interface"),
            Self::Enum { .. } => false,
        }
    }
}
