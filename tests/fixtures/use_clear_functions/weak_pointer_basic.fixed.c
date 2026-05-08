#include <glib-object.h>

static void
my_func (GObject *obj)
{
  g_clear_weak_pointer (&obj);
}
