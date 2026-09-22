Flags unnecessary NULL checks before GLib functions like `g_free`, `g_strfreev`,
and the `g_clear_*` family that already handle `NULL` gracefully.

## Why?

- **Redundant logic**: `g_free(NULL)` is documented as a no-op, so the `if`
  never prevents anything
- **Noise**: The extra indentation obscures the actual cleanup intent
- **Consistency**: Idiomatic GLib/GObject code relies on NULL-safe free
  functions without guards

## When the check is NOT flagged

The rule skips cases where the NULL check is actually meaningful:

- **`if`/`else` blocks** — removing the `if` would drop the `else` branch
- **Multi-statement bodies** — the guard may protect more than the free call
- **Different variables** — `if (a) g_free(b)` is not a redundant NULL check
- **Dereferences** — `if (node) g_free(node->field)` guards against
  dereferencing `node`, not against passing NULL to `g_free`
- **`g_clear_*` without address-of** — `if (ptr) g_clear_pointer(ptr, g_free)`
  dereferences `ptr`, so the guard is needed

## Covered functions

`g_free`, `g_free_size`, `g_strfreev`, and any `g_clear_*` variant.

## Examples

**Bad** (unnecessary guard):
```c
if (ptr)
  g_free (ptr);

if (ptr != NULL)
  g_free (ptr);

if (strv)
  g_strfreev (strv);

if (ptr)
  g_clear_pointer (&ptr, g_free);
```

**Good** (direct call):
```c
g_free (ptr);

g_strfreev (strv);

g_clear_pointer (&ptr, g_free);
```

This rule supports `--fix` to automatically remove the redundant NULL check.
