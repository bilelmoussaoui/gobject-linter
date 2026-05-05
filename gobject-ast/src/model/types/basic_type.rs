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
