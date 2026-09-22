Ensures that `g_task_set_source_tag()` is called after every `g_task_new()`
call.

## Why?

- **Debugging**: the source tag identifies which function created the task. When
  debugging async operations with many concurrent tasks, the tag is the primary
  way to tell them apart in logs and debugger output
- **Validation**: `g_task_is_valid()` uses the source tag to verify that a
  `GAsyncResult` passed to a `_finish` function was actually created by the
  matching `_async` function
- **Convention**: the GLib documentation recommends always setting the source tag
  to the calling function

## Examples

**Bad** (missing source tag):
```c
static void
my_obj_load_async (MyObj               *self,
                   GCancellable        *cancellable,
                   GAsyncReadyCallback  callback,
                   gpointer             user_data)
{
  GTask *task = g_task_new (self, cancellable, callback, user_data);
  /* missing g_task_set_source_tag */
  g_task_run_in_thread (task, load_thread);
}
```

**Good** (source tag set):
```c
static void
my_obj_load_async (MyObj               *self,
                   GCancellable        *cancellable,
                   GAsyncReadyCallback  callback,
                   gpointer             user_data)
{
  GTask *task = g_task_new (self, cancellable, callback, user_data);
  g_task_set_source_tag (task, my_obj_load_async);
  g_task_run_in_thread (task, load_thread);
}
```

## Notes

Requires GLib >= 2.36.

This rule supports `--fix` to automatically insert the `g_task_set_source_tag` call.
