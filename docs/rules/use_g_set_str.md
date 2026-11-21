Detects a free call followed by an assignment of the same variable and
suggests replacing both statements with a single `g_set_*()` call.

## Why?

- **Change detection**: the setters return `TRUE` if the value actually
  changed, which is useful for skipping redundant property notifications
- **NULL safety**: the setters handle NULL values for both the old and new
  value correctly
- **Less code**: one function call replaces two, reducing boilerplate in
  property setters

## Patterns detected

| Free call | Assignment | Suggestion | Since |
|---|---|---|---|
| `g_free (var)` | `var = g_strdup (x)` | `g_set_str (&var, x)` | 2.76 |
| `g_free (var)` | `var = <owned string>` | `g_set_str_take (&var, …)` | 2.90 |
| `g_strfreev (var)` | `var = g_strdupv (x)` | `g_set_strv (&var, x)` | 2.90 |
| `g_strfreev (var)` | `var = <owned strv>` | `g_set_strv_take (&var, …)` | 2.90 |
| `g_date_time_unref (var)` | `var = g_date_time_ref (x)` | `g_set_date_time (&var, x)` | 2.90 |

Each free call also matches the `g_clear_pointer (&var, <free function>)`
variant. `<owned string>`/`<owned strv>` is an expression that transfers
ownership of its result, such as `g_strdup_printf()`, `g_strconcat()`,
`g_strsplit()` or `g_steal_pointer()`. Suggestions requiring GLib 2.90 are
only emitted when `min_glib_version` allows it.

## Examples

**Bad** (manual free + dup):
```c
g_free (self->name);
self->name = g_strdup (new_name);
```

**Good**:
```c
g_set_str (&self->name, new_name);
```

**Bad** (manual free + owned assignment):
```c
g_free (self->name);
self->name = g_strdup_printf ("%s!", name);
```

**Good**:
```c
g_set_str_take (&self->name, g_strdup_printf ("%s!", name));
```

## Notes

Requires GLib >= 2.76 (`g_set_str_take()`, `g_set_strv()`,
`g_set_strv_take()` and `g_set_date_time()` require >= 2.90).

This rule supports `--fix` to automatically replace the manual pattern.
