#include <glib-object.h>

static void
my_func (void)
{
  GObject *obj = g_object_new (G_TYPE_OBJECT, NULL);
  use_object (obj);
  g_object_unref (obj);
}
