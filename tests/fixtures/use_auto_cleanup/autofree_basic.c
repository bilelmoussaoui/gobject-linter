#include <glib.h>

static void
my_func (const char *input)
{
  char *copy = g_strdup (input);
  g_print ("%s\n", copy);
  g_free (copy);
}
