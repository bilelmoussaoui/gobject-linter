Detects the pattern of calling `g_file_load_contents()` (or its `_finish`
variant) followed by `g_bytes_new_take()` to wrap the result, and suggests using
`g_file_load_bytes()` / `g_file_load_bytes_async()` instead.

## Why?

- **Simpler code**: `g_file_load_bytes()` returns a `GBytes` directly,
  eliminating the manual wrap step
- **Fewer variables**: no need for separate `contents` and `length` variables
  just to pass them to `g_bytes_new_take()`
- **Less error-prone**: the manual pattern requires careful memory ownership
  tracking between `g_file_load_contents` output and `g_bytes_new_take` input

## Examples

**Bad** (manual wrap):
```c
g_file_load_contents (file, NULL, &contents, &length, NULL, &error);
bytes = g_bytes_new_take (contents, length);
```

**Good** (direct):
```c
bytes = g_file_load_bytes (file, NULL, NULL, &error);
```

## Notes

Requires GLib >= 2.56. This rule is not auto-fixable because the transformation
may require adjusting variable declarations and error handling.
