Suggests replacing `g_idle_add()` / `g_timeout_add()` with their `_once`
variants when the callback always returns `G_SOURCE_REMOVE`.

## Why?

- **Intent clarity**: `g_idle_add_once()` makes it explicit that the callback
  runs exactly once, without needing to inspect the callback's return value
- **Simpler callback**: the `_once` variants use `GSourceOnceFunc` which returns
  `void` — no need to return `G_SOURCE_REMOVE` or `FALSE`
- **Less error-prone**: eliminates the risk of accidentally returning
  `G_SOURCE_CONTINUE` from a callback that should only fire once

## Supported replacements

| Original                    | Replacement                        | Min GLib |
|-----------------------------|------------------------------------|----------|
| `g_idle_add`                | `g_idle_add_once`                  | 2.74     |
| `g_timeout_add`             | `g_timeout_add_once`               | 2.74     |
| `g_timeout_add_seconds`     | `g_timeout_add_seconds_once`       | 2.78     |

## Examples

**Bad** (callback returns G_SOURCE_REMOVE):
```c
static gboolean
do_work (gpointer user_data)
{
  /* ... */
  return G_SOURCE_REMOVE;
}

g_idle_add (do_work, data);
```

**Good** (once variant):
```c
static void
do_work (gpointer user_data)
{
  /* ... */
}

g_idle_add_once (do_work, data);
```

## Notes

This rule only triggers when the callback function is defined in the same file
and all its return paths return `G_SOURCE_REMOVE` or `FALSE`. It also checks
that the callback is not used elsewhere (e.g., passed to both `g_idle_add` and
`g_signal_connect`).

This rule supports `--fix` to automatically replace the call with the `_once` variant.
