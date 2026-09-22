Suggests using `g_strcmp0()` instead of `strcmp()` for NULL-safe string
comparison.

## Why?

- **NULL safety**: `strcmp()` causes undefined behavior when passed a `NULL`
  pointer, while `g_strcmp0()` treats `NULL` as less than any non-NULL string
  and two `NULL` values as equal
- **Defensive coding**: GLib/GObject code frequently deals with nullable strings
  from properties, signal parameters, and configuration — `g_strcmp0()` handles
  these without requiring explicit `NULL` guards

## Examples

**Bad** (crashes on NULL):
```c
if (strcmp (name, expected) == 0)
  do_something ();
```

**Good** (NULL-safe):
```c
if (g_strcmp0 (name, expected) == 0)
  do_something ();
```

## Notes

Requires GLib >= 2.16.

This rule supports `--fix` to automatically replace `strcmp()` with `g_strcmp0()`.
