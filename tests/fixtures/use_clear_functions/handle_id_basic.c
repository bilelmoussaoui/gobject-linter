#include <glib.h>

static void
my_func (guint timeout_id, gulong handler_id, guint owner_id, guint watcher_id)
{
  if (timeout_id) {
    g_source_remove (timeout_id);
    timeout_id = 0;
  }

  g_source_remove (handler_id);
  handler_id = 0;

  if (owner_id != 0) {
    g_bus_unown_name (owner_id);
    owner_id = 0;
  }

  g_bus_unown_name (handler_id);
  handler_id = 0;

  if (watcher_id != 0) {
    g_bus_unwatch_name (watcher_id);
    watcher_id = 0;
  }

  g_bus_unwatch_name (handler_id);
  handler_id = 0;
}
