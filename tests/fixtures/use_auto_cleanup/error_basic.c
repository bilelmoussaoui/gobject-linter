#include <glib.h>

static void
my_func (void)
{
  GError *error = NULL;
  do_something (&error);
  if (error)
    g_error_free (error);
}
