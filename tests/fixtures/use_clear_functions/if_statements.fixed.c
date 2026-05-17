#include <glib.h>

static void
my_func (gpointer a, gpointer b, guint c)
{
  g_clear_pointer (&a, g_free);

  g_clear_pointer (&a, g_free);

  g_clear_pointer (&a, g_free);

  g_clear_handle_id (&c, g_source_remove);

  g_clear_handle_id (&c, g_source_remove);

  /* Variables don't match */
  if (!b)
    {
      g_clear_pointer (&a, g_free);
    }

  /* Not a simple NULL check */
  if (a != b)
    {
      g_clear_pointer (&a, g_free);
    }

  /* Not a check against zero */
  if (c != 5)
    g_clear_handle_id (&c, g_source_remove);

  /* Not a non-equal check */
  if (a <= 0)
    {
      g_clear_pointer (&a, g_free);
    }

  /* Not a NOT */
  if (~c)
    g_clear_handle_id (&c, g_source_remove);

  /* Not something we want to touch */
  if ((a == b) == c)
    {
      g_clear_pointer (&a, g_free);
    }
}

