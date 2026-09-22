Warns when `g_timeout_add`, `g_idle_add`, and related functions are called
without storing the returned source ID.

## Why?

- **Use-after-free**: if the object referenced by the callback's user data is
  destroyed before the source fires, the callback accesses freed memory. Storing
  the source ID lets you remove it in `dispose` with `g_clear_handle_id()`
- **Cannot cancel**: without the source ID there is no way to cancel a pending
  timeout or idle callback
- **Common crash**: this is one of the most frequent causes of
  use-after-free bugs in GLib applications

## Checked functions

`g_timeout_add`, `g_timeout_add_full`, `g_timeout_add_seconds`,
`g_timeout_add_seconds_full`, `g_idle_add`, `g_idle_add_full`,
`gtk_widget_add_tick_callback`, and the `_once` variants.

## Examples

**Bad** (source ID discarded):
```c
static void
my_obj_start (MyObj *self)
{
  g_timeout_add (100, on_timeout, self);
}
```

**Good** (source ID stored and cleared in dispose):
```c
static void
my_obj_start (MyObj *self)
{
  self->timeout_id = g_timeout_add (100, on_timeout, self);
}

static void
my_obj_dispose (GObject *object)
{
  MyObj *self = MY_OBJ (object);
  g_clear_handle_id (&self->timeout_id, g_source_remove);
  G_OBJECT_CLASS (my_obj_parent_class)->dispose (object);
}
```

## Configuration

- **`check_once_functions`** (bool, default `true`): whether to also check
  `_once` variants (`g_idle_add_once`, `g_timeout_add_once`, etc.)
