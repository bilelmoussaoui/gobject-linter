Detects public API functions and GObject type declarations in installed headers
that lack an export visibility macro.

## Why?

- **Symbol visibility**: without an export macro, symbols default to hidden
  when built with `-fvisibility=hidden` (common in modern shared libraries),
  making them invisible to consumers
- **Shared library correctness**: missing exports cause linker errors
  (`undefined symbol`) for downstream users even though the function exists in
  the library
- **Consistent API surface**: export macros explicitly mark the public API
  boundary, making it clear which symbols are part of the stable interface

## What is checked

- **Function declarations** in public (installed) headers without an export
  macro such as `G_MODULE_EXPORT`, `*_EXPORT`, or similar
- **GObject type declarations** (`G_DECLARE_FINAL_TYPE`,
  `G_DECLARE_DERIVABLE_TYPE`, etc.) in public headers without an export macro
- Static functions and private (non-installed) headers are skipped

## Examples

**Flagged** (missing export macro):
```c
/* mylib.h (installed header) */
void my_lib_do_work (void);

G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)
```

**Good** (export macros present):
```c
/* mylib.h (installed header) */
MY_LIB_EXPORT
void my_lib_do_work (void);

MY_LIB_EXPORT
G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)
```

## Opt-in

This rule is **opt-in** because the parser may misidentify export macros in
some codebases. Enable it in your configuration:

```toml
[rules.missing_export_macro]
level = "warn"
```

The rule requires meson build information to distinguish public from private
headers.
