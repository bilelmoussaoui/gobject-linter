Ensures that `g_param_spec_*` calls include the `G_PARAM_STATIC_STRINGS` flag
(or the individual `G_PARAM_STATIC_NAME`, `G_PARAM_STATIC_NICK`,
`G_PARAM_STATIC_BLURB` flags) when the name, nick, or blurb are string
literals.

## Why?

- **Unnecessary copies**: without the static flags, GLib calls `g_strdup()` on
  each string parameter. For string literals that live in read-only memory, this
  wastes heap allocations
- **Performance**: a typical GObject class installs 5–20 properties. Without
  static flags, that means 15–60 unnecessary string copies at type registration
  time
- **Convention**: virtually all modern GLib/GTK code uses
  `G_PARAM_STATIC_STRINGS` — omitting it is almost always an oversight

## Examples

**Bad** (missing static strings flag):
```c
g_param_spec_int ("width",
                   "Width",
                   "The width in pixels",
                   0, G_MAXINT, 0,
                   G_PARAM_READWRITE);
```

**Good** (with G_PARAM_STATIC_STRINGS):
```c
g_param_spec_int ("width",
                   "Width",
                   "The width in pixels",
                   0, G_MAXINT, 0,
                   G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
```

## Configuration

- **`static_flags`**: list of custom flag constants that already include
  `G_PARAM_STATIC_STRINGS` (e.g., `["MY_PARAM_READWRITE"]`). Properties using
  these flags are not flagged.

This rule supports `--fix` to automatically add `G_PARAM_STATIC_STRINGS` to the flags.
