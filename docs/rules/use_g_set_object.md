Detects the pattern of `g_clear_object()`/`g_object_unref()` followed by an
assignment with `g_object_ref()`, and suggests replacing both statements with a
single `g_set_object()` call.

## Why?

- **Atomicity**: `g_set_object()` handles the unref-then-ref sequence in one
  call, avoiding a window where the pointer is cleared but not yet reassigned
- **NULL safety**: `g_set_object()` correctly handles NULL values for both the
  old and new object, without requiring extra NULL checks
- **Less code**: one function call replaces two, reducing boilerplate in property
  setters

## Examples

**Bad** (manual unref + ref):
```c
g_clear_object (&self->child);
self->child = g_object_ref (new_child);
```

**Bad** (variant with g_object_unref):
```c
g_object_unref (self->child);
self->child = g_object_ref (new_child);
```

**Good** (single call):
```c
g_set_object (&self->child, new_child);
```

## Notes

Requires GLib >= 2.44.

This rule supports `--fix` to automatically replace the manual pattern with `g_set_object()`.
