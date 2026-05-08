#include <glib.h>

static void
my_func (guint timeout_id, gulong handler_id)
{
  if (timeout_id) {
    g_source_remove (timeout_id);
    timeout_id = 0;
  }

  g_source_remove (handler_id);
  handler_id = 0;
}
