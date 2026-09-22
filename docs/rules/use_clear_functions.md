Suggests replacing manual null-check-then-free patterns with the corresponding
`g_clear_*` function: `g_clear_pointer`, `g_clear_object`, `g_clear_handle_id`,
`g_clear_error`, `g_clear_fd`, `g_clear_signal_handler`, `g_clear_weak_pointer`,
`g_clear_list`, or `g_clear_slist`.

## Why?

- **Atomicity**: `g_clear_pointer` and friends set the variable to
  `NULL`/`0`/`-1` *before* calling the free function, preventing use-after-free
  in re-entrant or concurrent code (e.g. a `dispose` method that can run twice)
- **Brevity**: a single `g_clear_object (&self->child)` replaces a three-line
  `if (self->child) { g_object_unref (self->child); self->child = NULL; }`
  pattern
- **Correctness**: the manual pattern is easy to get wrong — forgetting the
  NULL assignment, freeing without checking, or assigning NULL before freeing

## Examples

**Bad** (manual cleanup):
```c
if (self->child != NULL)
  {
    g_object_unref (self->child);
    self->child = NULL;
  }

if (self->name)
  {
    g_free (self->name);
    self->name = NULL;
  }

if (self->source_id != 0)
  {
    g_source_remove (self->source_id);
    self->source_id = 0;
  }
```

**Good** (clear functions):
```c
g_clear_object (&self->child);
g_clear_pointer (&self->name, g_free);
g_clear_handle_id (&self->source_id, g_source_remove);
```

This rule supports `--fix` to automatically replace manual patterns with the
corresponding `g_clear_*` call.
