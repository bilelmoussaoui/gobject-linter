#include <gio/gio.h>

static void my_func_thread (GTask *task, gpointer src, gpointer data, GCancellable *c) { }

static void
my_func_async (GObject *source, GCancellable *cancellable,
               GAsyncReadyCallback callback, gpointer user_data)
{
  GTask *task = g_task_new (source, cancellable, callback, user_data);
  g_task_set_source_tag (task, my_func_async);
  g_task_run_in_thread (task, my_func_thread);
}
