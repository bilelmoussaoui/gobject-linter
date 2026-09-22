Detects the pattern of `g_free()` or `g_clear_pointer(&var, g_free)` followed by
an assignment with `g_strdup()`, and suggests replacing both statements with a
single `g_set_str()` call.

## Why?

- **Change detection**: `g_set_str()` returns `TRUE` if the value actually
  changed, which is useful for skipping redundant property notifications
- **NULL safety**: `g_set_str()` handles NULL values for both the old and new
  string correctly
- **Less code**: one function call replaces two, reducing boilerplate in string
  property setters

## Examples

**Bad** (manual free + dup):
```c
g_free (self->name);
self->name = g_strdup (new_name);
```

**Bad** (variant with g_clear_pointer):
```c
g_clear_pointer (&self->name, g_free);
self->name = g_strdup (new_name);
```

**Good** (single call):
```c
g_set_str (&self->name, new_name);
```

## Notes

Requires GLib >= 2.76.

This rule supports `--fix` to automatically replace the manual pattern with `g_set_str()`.
