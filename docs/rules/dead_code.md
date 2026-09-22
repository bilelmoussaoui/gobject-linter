Detects internal functions, types, enum values, and struct fields that are
defined but never referenced anywhere in the codebase.

## Why?

- **Maintainability**: dead code is a maintenance burden — it must be read,
  compiled, and updated even though it does nothing
- **Clarity**: removing unused symbols makes the codebase easier to navigate
- **Binary size**: unused static functions and types still contribute to
  compile time and may survive in debug builds

## What is checked

| Kind | Scope |
|---|---|
| **Static functions** | `.c` files — flagged if never called or used as a callback |
| **Internal functions** | declared in private (non-installed) headers but never referenced |
| **Types** (structs, typedefs) | defined in private headers or `.c` files and never used in declarations, casts, `sizeof`, or GObject macros |
| **Enum values** | defined in private files and never referenced (skips `PROP_0` / `*_LAST` sentinels) |
| **Struct fields** | defined in private structs and never read (write-only fields are flagged) |

Public API symbols — functions declared in installed headers — are never
flagged, since they may be used by downstream consumers.

## Examples

```c
// Flagged: static function never called or passed as a callback
static void
unused_helper (void)
{
  // ...
}
```

```c
// Flagged: type defined in private header but never used
typedef struct _OldState OldState;
```

```c
// NOT flagged: function declared in a public (installed) header
// (may be used by library consumers)
void my_lib_public_func (void);
```

## Opt-in

This rule is **opt-in** because static analysis without a preprocessor cannot
see through `#ifdef` blocks or resolve macro-generated references, which may
cause false positives. Enable it in your configuration:

```toml
[rules.dead_code]
level = "warn"
```

The rule also requires meson build information (`meson.build`) to distinguish
public from private headers.
