#include <glib-object.h>

static void
my_func (void)
{
  GObject *obj = g_object_new (G_TYPE_OBJECT, NULL);

  if (!do_something ())
    goto cleanup;

  use_object (obj);

cleanup:
  g_object_unref (obj);
}
