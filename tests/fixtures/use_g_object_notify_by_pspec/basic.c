#include <glib-object.h>

static void
foo_set_name (GObject *self, const char *name)
{
  g_object_notify (G_OBJECT (self), "name");
  g_object_notify (G_OBJECT (self), "display-name");
}
