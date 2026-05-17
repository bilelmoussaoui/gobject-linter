#include <glib.h>

static void
my_func (gpointer a, gpointer b, guint c)
{
  if (a != NULL)
    {
      g_free (a);
      a = NULL;
    }

  if (NULL != a)
    {
      g_free (a);
      a = NULL;
    }

  if (!a)
    {
      g_free (a);
      a = NULL;
    }

  if (c > 0)
    {
      g_source_remove (c);
      c = 0;
    }

  if (0 < c)
    {
      g_source_remove (c);
      c = 0;
    }

  /* Variables don't match */
  if (!b)
    {
      g_free (a);
      a = NULL;
    }

  /* Not a simple NULL check */
  if (a != b)
    {
      g_free (a);
      a = NULL;
    }

  /* Not a check against zero */
  if (c != 5)
    {
      g_source_remove (c);
      c = 0;
    }

  /* Not a non-equal check */
  if (a <= 0)
    {
      g_free (a);
      a = NULL;
    }

  /* Not a NOT */
  if (~c)
    {
      g_source_remove (c);
      c = 0;
    }

  /* Not something we want to touch */
  if ((a == b) == c)
    {
      g_free (a);
      a = NULL;
    }
}

