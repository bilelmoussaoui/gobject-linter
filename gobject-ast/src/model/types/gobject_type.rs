use serde::{Serialize, Serializer, ser::SerializeMap as _};

use crate::model::{
    ExportMacro, FunctionDefItem, FunctionDoc, GType, ParamSpecAssignment, Parameter, Signal,
    SourceLocation, Statement, TypeDoc, TypeInfo,
};

#[derive(Debug, Clone, Serialize)]
pub struct GObjectType {
    pub type_name: String, // e.g., "ClutterInputDeviceTool"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_macro: Option<GType>, // e.g., CLUTTER_TYPE_INPUT_DEVICE_TOOL; None for quarks
    pub function_prefix: String, // e.g., "clutter_input_device_tool"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_type: Option<String>, // e.g., "GObject"; None for boxed/pointer types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<String>, /* G_DEFINE_TYPE_EXTENDED flags arg, e.g.
                            * "G_TYPE_FLAG_ABSTRACT" */
    pub kind: GObjectTypeKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<InterfaceImplementation>, // G_IMPLEMENT_INTERFACE
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub has_private: bool, /* G_ADD_PRIVATE in *_WITH_CODE, or
                            * *_WITH_PRIVATE */
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub code_block_statements: Vec<Statement>, // Statements from *_WITH_CODE macros
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub export_macros: Vec<ExportMacro>, // e.g., [Other("CLUTTER_EXPORT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<TypeDoc>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<ParamSpecAssignment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signals: Vec<Signal>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterfaceImplementation {
    pub interface_type: GType, // e.g., GTK_TYPE_EDITABLE
    pub init_function: String, // e.g., "mask_entry_editable_init"
}

impl GObjectType {
    pub fn function_prefix(&self) -> &str {
        &self.function_prefix
    }

    /// Get the expected instance init function name based on the
    /// function_prefix
    pub fn init_function_name(&self) -> String {
        format!("{}_init", self.function_prefix)
    }

    /// Get the expected class_init function name based on the function_prefix
    pub fn class_init_function_name(&self) -> String {
        format!("{}_class_init", self.function_prefix)
    }

    /// Get the expected default_init function name for interfaces
    pub fn default_init_function_name(&self) -> String {
        format!("{}_default_init", self.function_prefix)
    }

    /// Returns the expected name of the class or interface struct for this
    /// type, without the leading underscore. Returns `None` for final types,
    /// which have no separate class struct.
    ///
    /// - Derivable types → `"{TypeName}Class"`
    /// - Interface types → `"{TypeName}Interface"`
    pub fn class_struct_name(&self) -> Option<String> {
        match &self.kind {
            GObjectTypeKind::Declare {
                kind: DeclareKind::Final,
                ..
            }
            | GObjectTypeKind::DefineEnum { .. }
            | GObjectTypeKind::DefineFlags { .. } => None,
            GObjectTypeKind::Declare {
                kind: DeclareKind::Interface,
                ..
            }
            | GObjectTypeKind::Define(DefineKind::Interface | DefineKind::InterfaceWithCode) => {
                Some(format!("{}Interface", self.type_name))
            }
            _ => Some(format!("{}Class", self.type_name)),
        }
    }

    /// Check if this is an interface type
    pub fn is_interface(&self) -> bool {
        matches!(
            self.kind,
            GObjectTypeKind::Declare {
                kind: DeclareKind::Interface,
                ..
            } | GObjectTypeKind::Define(DefineKind::Interface | DefineKind::InterfaceWithCode)
        )
    }

    /// Extract signals from a class_init function
    pub fn extract_signals(&self, class_init_func: &FunctionDefItem, source: &[u8]) -> Vec<Signal> {
        class_init_func
            .find_calls_matching(|name| name.starts_with("g_signal_new"))
            .iter()
            .filter_map(|call| Signal::from_g_signal_new_call(call, source))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VirtualFunction {
    pub name: String,
    pub return_type: TypeInfo,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<FunctionDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumValueDef {
    pub name: String,
    pub nick: String,
}

/// Which G_DECLARE_* variant was used
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclareKind {
    Final,
    Derivable,
    Interface,
}

/// Which G_DEFINE_* (non-boxed, non-pointer, non-extended) variant was used
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefineKind {
    Type,
    TypeWithCode,
    TypeWithPrivate,
    FinalType,
    FinalTypeWithCode,
    FinalTypeWithPrivate,
    AbstractType,
    AbstractTypeWithCode,
    AbstractTypeWithPrivate,
    Interface,
    InterfaceWithCode,
    /// G_DEFINE_TYPE_EXTENDED
    TypeExtended,
    /// G_DEFINE_POINTER_TYPE
    Pointer,
}

#[derive(Debug, Clone)]
pub enum GObjectTypeKind {
    /// G_DECLARE_FINAL_TYPE / G_DECLARE_DERIVABLE_TYPE / G_DECLARE_INTERFACE
    Declare {
        kind: DeclareKind,
        module_prefix: String, // e.g., "CLUTTER"
        type_prefix: String,   // e.g., "INPUT_DEVICE_TOOL"
    },
    /// All G_DEFINE_TYPE* / G_DEFINE_INTERFACE* variants (not
    /// boxed/pointer/extended)
    Define(DefineKind),
    /// G_DEFINE_BOXED_TYPE / G_DEFINE_BOXED_TYPE_WITH_CODE
    DefineBoxed {
        copy_func: String,
        free_func: String,
    },
    /// G_DEFINE_QUARK(quark-name, func_prefix) → generates func_prefix_quark()
    DefineQuark {
        quark_name: String,
        func_prefix: String,
    },
    /// G_DEFINE_ENUM_TYPE(TypeName, func_prefix, G_DEFINE_ENUM_VALUE(…), …)
    DefineEnum { values: Vec<EnumValueDef> },
    /// G_DEFINE_FLAGS_TYPE(TypeName, func_prefix, G_DEFINE_ENUM_VALUE(…), …)
    DefineFlags { values: Vec<EnumValueDef> },
}

impl Serialize for GObjectTypeKind {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Define(_) => s.serialize_str(self.macro_name()),
            Self::Declare {
                module_prefix,
                type_prefix,
                ..
            } => {
                let mut m = s.serialize_map(Some(3))?;
                m.serialize_entry("macro", self.macro_name())?;
                m.serialize_entry("module_prefix", module_prefix)?;
                m.serialize_entry("type_prefix", type_prefix)?;
                m.end()
            }
            Self::DefineBoxed {
                copy_func,
                free_func,
            } => {
                let mut m = s.serialize_map(Some(3))?;
                m.serialize_entry("macro", self.macro_name())?;
                m.serialize_entry("copy_func", copy_func)?;
                m.serialize_entry("free_func", free_func)?;
                m.end()
            }
            Self::DefineQuark {
                quark_name,
                func_prefix,
            } => {
                let mut m = s.serialize_map(Some(3))?;
                m.serialize_entry("macro", self.macro_name())?;
                m.serialize_entry("quark_name", quark_name)?;
                m.serialize_entry("func_prefix", func_prefix)?;
                m.end()
            }
            Self::DefineEnum { values } | Self::DefineFlags { values } => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("macro", self.macro_name())?;
                m.serialize_entry("values", values)?;
                m.end()
            }
        }
    }
}

impl GObjectTypeKind {
    /// Returns the macro name for this type declaration/definition
    pub fn macro_name(&self) -> &'static str {
        match self {
            Self::Declare { kind, .. } => match kind {
                DeclareKind::Final => "G_DECLARE_FINAL_TYPE",
                DeclareKind::Derivable => "G_DECLARE_DERIVABLE_TYPE",
                DeclareKind::Interface => "G_DECLARE_INTERFACE",
            },
            Self::Define(kind) => match kind {
                DefineKind::Type => "G_DEFINE_TYPE",
                DefineKind::TypeWithCode => "G_DEFINE_TYPE_WITH_CODE",
                DefineKind::TypeWithPrivate => "G_DEFINE_TYPE_WITH_PRIVATE",
                DefineKind::FinalType => "G_DEFINE_FINAL_TYPE",
                DefineKind::FinalTypeWithCode => "G_DEFINE_FINAL_TYPE_WITH_CODE",
                DefineKind::FinalTypeWithPrivate => "G_DEFINE_FINAL_TYPE_WITH_PRIVATE",
                DefineKind::AbstractType => "G_DEFINE_ABSTRACT_TYPE",
                DefineKind::AbstractTypeWithCode => "G_DEFINE_ABSTRACT_TYPE_WITH_CODE",
                DefineKind::AbstractTypeWithPrivate => "G_DEFINE_ABSTRACT_TYPE_WITH_PRIVATE",
                DefineKind::Interface => "G_DEFINE_INTERFACE",
                DefineKind::InterfaceWithCode => "G_DEFINE_INTERFACE_WITH_CODE",
                DefineKind::TypeExtended => "G_DEFINE_TYPE_EXTENDED",
                DefineKind::Pointer => "G_DEFINE_POINTER_TYPE",
            },
            Self::DefineBoxed { .. } => "G_DEFINE_BOXED_TYPE",
            Self::DefineQuark { .. } => "G_DEFINE_QUARK",
            Self::DefineEnum { .. } => "G_DEFINE_ENUM_TYPE",
            Self::DefineFlags { .. } => "G_DEFINE_FLAGS_TYPE",
        }
    }

    /// Returns true if this is a G_DECLARE_* macro
    pub fn is_declare(&self) -> bool {
        matches!(self, Self::Declare { .. })
    }

    /// Returns true if this is a G_DEFINE_* macro
    pub fn is_define(&self) -> bool {
        matches!(
            self,
            Self::Define(_)
                | Self::DefineBoxed { .. }
                | Self::DefineQuark { .. }
                | Self::DefineEnum { .. }
                | Self::DefineFlags { .. }
        )
    }

    /// For `DefineQuark`, returns the generated quark function name
    /// (`func_prefix_quark`).
    pub fn quark_function_name(&self) -> Option<String> {
        if let Self::DefineQuark { func_prefix, .. } = self {
            Some(format!("{func_prefix}_quark"))
        } else {
            None
        }
    }

    /// Check if a declare kind is compatible with a define kind
    pub fn is_compatible_with(&self, define: &Self) -> bool {
        let Self::Declare { kind, .. } = self else {
            return false;
        };
        match kind {
            // G_DECLARE_FINAL_TYPE requires a final define
            DeclareKind::Final => matches!(
                define,
                Self::Define(
                    DefineKind::FinalType
                        | DefineKind::FinalTypeWithCode
                        | DefineKind::FinalTypeWithPrivate
                )
            ),
            // G_DECLARE_DERIVABLE_TYPE covers concrete and abstract types
            DeclareKind::Derivable => matches!(
                define,
                Self::Define(
                    DefineKind::Type
                        | DefineKind::TypeWithCode
                        | DefineKind::TypeWithPrivate
                        | DefineKind::AbstractType
                        | DefineKind::AbstractTypeWithCode
                        | DefineKind::AbstractTypeWithPrivate
                        | DefineKind::TypeExtended
                )
            ),
            // G_DECLARE_INTERFACE requires G_DEFINE_INTERFACE
            DeclareKind::Interface => matches!(
                define,
                Self::Define(DefineKind::Interface | DefineKind::InterfaceWithCode)
            ),
        }
    }
}
