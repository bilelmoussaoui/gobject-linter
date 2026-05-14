#include <glib.h>

static gboolean
handle_it_cb (gpointer data)
{
  if (already_handled ())
    return G_SOURCE_REMOVE;

  handle_it_now ();

  return G_SOURCE_REMOVE;
}

static void
setup (void)
{
  g_idle_add (handle_it_cb, NULL);
}
