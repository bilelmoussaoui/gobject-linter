Detects public API that has `AVAILABLE_IN_*` or `DEPRECATED_IN_*` export macros
but is missing or has mismatched `Since:` or `Deprecated:` gtk-doc annotations.

## Why?

- **Introspection accuracy**: `g-ir-scanner` reads `Since:` annotations to
  record when each symbol was introduced. Missing annotations mean bindings
  (Python, JavaScript, Vala) cannot report version requirements
- **Documentation consistency**: a function marked `AVAILABLE_IN_2_74` but
  documented as `Since: 2.72` is confusing — the macro controls symbol
  visibility while the doc tells users when to expect it
- **Deprecation tracking**: `DEPRECATED_IN_*` macros should match `Deprecated:`
  annotations so both the compiler (via `G_DEPRECATED_FOR`) and documentation
  agree on when the API was deprecated

## What it checks

- Functions, types, and enum values in public headers
- Missing `Since:` annotation when a versioned export macro is present
- Missing versioned export macro when a `Since:` annotation is present
- Mismatched versions between export macro and `Since:` annotation
- Property getter/setter `Since:` consistency with the property's own `Since:`
- Enum values with inline `Since:` that `g-ir-scanner` cannot detect (must use
  a standalone `/** ENUM_VALUE:` doc block instead)

## Examples

**Bad** (missing Since annotation):
```c
/**
 * my_obj_get_name:
 *
 * Gets the name.
 */
MY_LIB_AVAILABLE_IN_2_74
const char *my_obj_get_name (MyObj *self);
```

**Good** (matching Since annotation):
```c
/**
 * my_obj_get_name:
 *
 * Gets the name.
 *
 * Since: 2.74
 */
MY_LIB_AVAILABLE_IN_2_74
const char *my_obj_get_name (MyObj *self);
```

## Notes

This rule is opt-in and requires meson project information. It is only relevant
to libraries maintaining GObject Introspection annotations.
