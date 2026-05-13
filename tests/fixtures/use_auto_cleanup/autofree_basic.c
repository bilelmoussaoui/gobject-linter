#include <glib.h>

static void
my_func (const char *input)
{
  char *copy = g_strdup (input);
  g_print ("%s\n", copy);
  g_free (copy);

  /* Double pointer: g_autoptr(gchar) would give gchar*, not gchar** */
  gchar **tokens = g_strsplit (input, ":", -1);
  g_print ("%s\n", tokens[0]);
  g_strfreev (tokens);
}
