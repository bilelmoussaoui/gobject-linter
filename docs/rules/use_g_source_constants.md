Flags `return TRUE` / `return FALSE` in GSource callback functions and suggests
`G_SOURCE_CONTINUE` / `G_SOURCE_REMOVE` instead.

## Why?

- **Readability**: `G_SOURCE_CONTINUE` and `G_SOURCE_REMOVE` make the intent
  obvious — the reader immediately knows whether the source keeps firing or is
  removed
- **Self-documenting**: `TRUE` / `FALSE` require the reader to recall the
  GSource return-value convention; the named constants eliminate that mental
  lookup
- **Consistency**: this is the recommended style in modern GLib code

## How callbacks are detected

The rule identifies GSource callbacks by scanning for calls to:

- `g_idle_add`, `g_idle_add_full`
- `g_timeout_add`, `g_timeout_add_full`
- `g_timeout_add_seconds`, `g_timeout_add_seconds_full`
- `gtk_widget_add_tick_callback`

Only functions passed as callback arguments to these APIs are checked.
Functions that are never used as GSource callbacks are not flagged, even if they
return `TRUE` / `FALSE`.

## Examples

**Bad** (raw booleans):
```c
static gboolean
my_timeout_cb (gpointer user_data)
{
  g_print ("tick\n");
  return TRUE;
}

static gboolean
my_idle_cb (gpointer user_data)
{
  return FALSE;
}
```

**Good** (named constants):
```c
static gboolean
my_timeout_cb (gpointer user_data)
{
  g_print ("tick\n");
  return G_SOURCE_CONTINUE;
}

static gboolean
my_idle_cb (gpointer user_data)
{
  return G_SOURCE_REMOVE;
}
```

This rule supports `--fix` to automatically replace `TRUE`/`FALSE` with `G_SOURCE_CONTINUE`/`G_SOURCE_REMOVE`.
