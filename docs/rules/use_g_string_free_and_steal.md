Suggests using `g_string_free_and_steal()` instead of
`g_string_free(str, FALSE)` when you want to free the `GString` container but
keep the underlying character data.

## Why?

- **Readability**: `g_string_free_and_steal(str)` clearly communicates the intent
  to take ownership of the string data, while the meaning of the `FALSE`
  parameter in `g_string_free(str, FALSE)` is not obvious without checking the
  documentation
- **Self-documenting**: the function name describes exactly what happens — the
  `GString` is freed and the `char *` is stolen

## Examples

**Bad** (opaque boolean parameter):
```c
gchar *result = g_string_free (str, FALSE);
```

**Good** (self-documenting):
```c
gchar *result = g_string_free_and_steal (str);
```

## Notes

Requires GLib >= 2.76.

This rule supports `--fix` to automatically replace `g_string_free(str, FALSE)` with `g_string_free_and_steal()`.
