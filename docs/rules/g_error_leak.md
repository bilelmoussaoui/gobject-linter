Checks for `GError *` variables that are passed to functions (via `&error`) but
are never freed or propagated, leaking memory.

## Why?

- **Memory leak**: each `GError` allocated by `g_set_error` / `g_error_new`
  contains a heap-allocated message string. Forgetting to free it is a leak
- **Ownership rules**: GLib error ownership is explicit — you must either free
  the error (`g_error_free`, `g_clear_error`) or transfer ownership
  (`g_propagate_error`, `g_propagate_prefixed_error`, `g_task_return_error`,
  `g_dbus_method_invocation_take_error`, `g_steal_pointer`). Functions whose
  name contains `_terminate_` and `error`, ends with `_set_error`, or contains
  `_set_g_error` are also recognized as taking ownership
- **Common mistake**: developers often check the error, log it, and forget to
  free it, especially in early-return paths

## Examples

**Bad** (GError leaked):
```c
void
my_func (void)
{
  GError *error = NULL;

  if (!some_operation (&error))
    {
      g_warning ("Failed: %s", error->message);
      return;  /* leak! */
    }
}
```

**Good** (GError freed):
```c
void
my_func (void)
{
  GError *error = NULL;

  if (!some_operation (&error))
    {
      g_warning ("Failed: %s", error->message);
      g_error_free (error);
      return;
    }
}
```

**Good** (GError propagated):
```c
gboolean
my_func (GError **error)
{
  GError *local_error = NULL;

  if (!some_operation (&local_error))
    {
      g_propagate_error (error, local_error);
      return FALSE;
    }

  return TRUE;
}
```

## Configuration

- **`extra_noreturn_functions`**: additional function names that terminate the
  program, suppressing leak warnings (e.g., `["my_app_abort"]`)
- **`extra_propagation_functions`**: additional function names that take
  ownership of the GError (e.g., `["dbus_reply_error"]`)
