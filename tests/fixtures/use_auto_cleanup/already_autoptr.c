#include <glib-object.h>

/* Variable already uses g_autoptr — should not be flagged. */
static void
example (void)
{
  g_autoptr(GFile) dest_file = NULL;

  dest_file = g_file_new_for_uri ("foo");
  g_clear_object (&dest_file);
  dest_file = g_file_new_for_uri ("bar");
}
