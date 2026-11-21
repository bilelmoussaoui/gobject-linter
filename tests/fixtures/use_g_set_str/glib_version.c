#include <glib.h>

/* With min_glib_version = "2.80", only the g_set_str() suggestion
 * (available since 2.76) is kept; the 2.90 setters are not suggested */

static void
set_values (char **name, const char *new_name, char *owned)
{
  g_free (*name);
  *name = g_strdup (new_name);

  g_free (*name);
  *name = g_steal_pointer (&owned);
}
