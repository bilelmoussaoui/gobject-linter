#include <glib-object.h>

static void
my_func (GObject *obj)
{
  g_object_remove_weak_pointer (G_OBJECT (obj), (gpointer *) &obj);
  obj = NULL;
}
