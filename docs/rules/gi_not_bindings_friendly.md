Detects public API patterns that are problematic for GObject Introspection
bindings (Python, JavaScript, Vala, etc.).

## Why?

- **Variadic functions**: `g-ir-scanner` cannot introspect C variadic arguments
  (`...`). Bindings simply cannot call these functions at all
- **Too many out parameters**: functions with more than 2 pointer-to-pointer
  (out) parameters are awkward in languages that return multiple values via
  tuples — consider returning a struct or using a builder pattern
- **Raw container types**: `GList *`, `GSList *`, `GHashTable *`, `GPtrArray *`,
  `GArray *`, and `GByteArray *` in public API are problematic because
  introspection cannot determine the element types. Prefer `GListModel` or
  typed alternatives

## What it flags

| Pattern                        | Problem                                    |
|--------------------------------|--------------------------------------------|
| Variadic parameters (`...`)    | Cannot be introspected at all              |
| >2 out (pointer-to-pointer) params | Clumsy in bindings, hard to annotate   |
| `GList *` / `GSList *` return or param | Element type unknown to introspection |
| `GHashTable *` return or param | Key/value types unknown to introspection   |
| `GPtrArray *` / `GArray *` / `GByteArray *` | Element type unknown         |

## Examples

**Flagged** (variadic function):
```c
MY_LIB_EXPORT
void my_obj_set_properties (MyObj *self, const char *first, ...);
```

**Good** (non-variadic alternative):
```c
MY_LIB_EXPORT
void my_obj_set_name (MyObj *self, const char *name);
```

**Flagged** (raw container in public API):
```c
MY_LIB_EXPORT
GList *my_obj_get_children (MyObj *self);
```

**Good** (typed model):
```c
MY_LIB_EXPORT
GListModel *my_obj_get_children (MyObj *self);
```

## Notes

This rule is opt-in and requires meson project information. Functions annotated
with `(skip)` are excluded from checks. Only relevant to libraries maintaining
GObject Introspection bindings.
