#include <glib.h>

static gchar **
collect_names (void)
{
  GPtrArray *array = g_ptr_array_new ();

  g_ptr_array_add (array, g_strdup ("Alice"));
  g_ptr_array_add (array, g_strdup ("Bob"));
  g_ptr_array_add (array, NULL);

  return (gchar **) g_ptr_array_free (array, FALSE);
}

static void
discard_names (void)
{
  GPtrArray *array = g_ptr_array_new ();

  g_ptr_array_add (array, g_strdup ("Alice"));

  g_ptr_array_free (array, TRUE);
}
