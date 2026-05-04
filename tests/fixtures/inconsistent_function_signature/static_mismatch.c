#include <glib.h>

static gboolean baz_cb (gpointer data);

static gint
baz_cb (gpointer data)
{
  return TRUE;
}
