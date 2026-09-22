Detects enums whose values are power-of-two constants (bit flags) but are
missing the `G_GNUC_FLAG_ENUM` attribute.

## Why?

- **Compiler warnings**: without `G_GNUC_FLAG_ENUM`, GCC's `-Wswitch` may warn
  about bitwise-OR combinations of flag values not being handled in switch
  statements, even though exhaustive matching on flags is impractical
- **Intent documentation**: the attribute clearly signals to readers and tools
  that the enum represents combinable bit flags, not mutually exclusive values
- **GObject Introspection**: the attribute helps GI tooling generate correct
  bindings for flag types

## Examples

**Bad** (missing attribute):
```c
typedef enum {
  MY_FLAG_NONE  = 0,
  MY_FLAG_READ  = 1 << 0,
  MY_FLAG_WRITE = 1 << 1,
  MY_FLAG_EXEC  = 1 << 2,
} MyFlags;
```

**Good** (with attribute):
```c
typedef enum {
  MY_FLAG_NONE  = 0,
  MY_FLAG_READ  = 1 << 0,
  MY_FLAG_WRITE = 1 << 1,
  MY_FLAG_EXEC  = 1 << 2,
} G_GNUC_FLAG_ENUM MyFlags;
```

## Notes

Requires GLib >= 2.87.

This rule supports `--fix` to automatically add the `G_GNUC_FLAG_ENUM` attribute.
