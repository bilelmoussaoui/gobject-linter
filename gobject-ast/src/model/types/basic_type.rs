use serde::{Deserialize, Serialize};

/// A C/GLib primitive scalar type.
///
/// Variants marked with a `G_TYPE_*` have a counterpart in `GType::as_basic()`.
/// The rest are valid C types that appear in `TypeInfo` but have no GType
/// registration (e.g. `short`, `long long`, fixed-width integers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BasicType {
    Boolean, // gboolean                      / G_TYPE_BOOLEAN
    Char,    // gchar / char                  / G_TYPE_CHAR
    UChar,   // guchar / unsigned char        / G_TYPE_UCHAR
    Int,     // gint / int                    / G_TYPE_INT
    UInt,    // guint / unsigned int          / G_TYPE_UINT
    Long,    // glong / long                  / G_TYPE_LONG
    ULong,   // gulong / unsigned long        / G_TYPE_ULONG
    Int64,   // gint64 / int64_t              / G_TYPE_INT64
    UInt64,  // guint64 / uint64_t            / G_TYPE_UINT64
    Float,   // gfloat / float                / G_TYPE_FLOAT
    Double,  // gdouble / double              / G_TYPE_DOUBLE
    String,  // gchar * / char *              / G_TYPE_STRING
    Pointer, // gpointer / void *             / G_TYPE_POINTER

    Bool,       // _Bool / bool (C99)
    Short,      // gshort / short / signed short
    UShort,     // gushort / unsigned short
    LongLong,   // long long / signed long long (C99)
    ULongLong,  // unsigned long long (C99)
    LongDouble, // long double
    Int8,       // gint8 / int8_t / signed char
    UInt8,      // guint8 / uint8_t
    Int16,      // gint16 / int16_t
    UInt16,     // guint16 / uint16_t
    Int32,      // gint32 / int32_t
    UInt32,     // guint32 / uint32_t
    Size,       // gsize / size_t
    SSize,      // gssize / ssize_t
    Offset,     // goffset
    IntPtr,     // gintptr / intptr_t
    UIntPtr,    // guintptr / uintptr_t
}

impl BasicType {
    /// The canonical GLib base-type spelling, or `None` if this type has no
    /// GLib equivalent or the conversion would change the pointer depth
    /// (e.g. `gpointer` ↔ `void *`).
    pub fn canonical_glib(self) -> Option<&'static str> {
        match self {
            Self::Boolean => None, // gboolean ≠ bool
            Self::Bool => None,    // bool ≠ gboolean
            Self::Char => Some("gchar"),
            Self::UChar => Some("guchar"),
            Self::Int => Some("gint"),
            Self::UInt => Some("guint"),
            Self::Short => Some("gshort"),
            Self::UShort => Some("gushort"),
            Self::Long => Some("glong"),
            Self::ULong => Some("gulong"),
            Self::Int8 => Some("gint8"),
            Self::UInt8 => Some("guint8"),
            Self::Int16 => Some("gint16"),
            Self::UInt16 => Some("guint16"),
            Self::Int32 => Some("gint32"),
            Self::UInt32 => Some("guint32"),
            Self::Int64 => Some("gint64"),
            Self::UInt64 => Some("guint64"),
            Self::Float => Some("gfloat"),
            Self::Double => Some("gdouble"),
            Self::String => Some("gchar"),
            Self::Size => Some("gsize"),
            Self::SSize => Some("gssize"),
            Self::Offset => Some("goffset"),
            Self::IntPtr => Some("gintptr"),
            Self::UIntPtr => Some("guintptr"),
            // gpointer ↔ void * changes pointer_depth — skip
            Self::Pointer => None,
            // No GLib equivalents for these C types
            Self::LongLong | Self::ULongLong | Self::LongDouble => None,
        }
    }

    /// The canonical C/C99 base-type spelling, or `None` if this type has no
    /// clean C standard equivalent or the conversion would change the pointer
    /// depth.
    pub fn canonical_c(self) -> Option<&'static str> {
        match self {
            Self::Boolean => None, // gboolean ≠ bool
            Self::Bool => None,    // already canonical C
            Self::Char => Some("char"),
            Self::UChar => Some("unsigned char"),
            Self::Int => Some("int"),
            Self::UInt => Some("unsigned int"),
            Self::Short => Some("short"),
            Self::UShort => Some("unsigned short"),
            Self::Long => Some("long"),
            Self::ULong => Some("unsigned long"),
            Self::Int8 => Some("int8_t"),
            Self::UInt8 => Some("uint8_t"),
            Self::Int16 => Some("int16_t"),
            Self::UInt16 => Some("uint16_t"),
            Self::Int32 => Some("int32_t"),
            Self::UInt32 => Some("uint32_t"),
            Self::Int64 => Some("int64_t"),
            Self::UInt64 => Some("uint64_t"),
            Self::Float => Some("float"),
            Self::Double => Some("double"),
            Self::String => Some("char"),
            Self::Size => Some("size_t"),
            Self::SSize => Some("ssize_t"),
            Self::IntPtr => Some("intptr_t"),
            Self::UIntPtr => Some("uintptr_t"),
            // goffset has no portable C standard equivalent
            Self::Offset => None,
            // gpointer ↔ void * changes pointer_depth
            Self::Pointer => None,
            // Already canonical C
            Self::LongLong | Self::ULongLong | Self::LongDouble => None,
        }
    }
}
