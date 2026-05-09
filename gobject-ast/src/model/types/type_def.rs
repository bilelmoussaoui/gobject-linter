use serde::Serialize;

use crate::model::{EnumInfo, Parameter, SourceLocation, TypeDoc, TypeInfo, VirtualFunction};

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
