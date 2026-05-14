#include <glib.h>

static void
handle_it_cb (gpointer data)
{
  if (already_handled ())
    return;

  handle_it_now ();
}

static void
setup (void)
{
  g_idle_add_once (handle_it_cb, NULL);
}
