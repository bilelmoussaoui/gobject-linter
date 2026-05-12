#include <glib-object.h>

typedef struct {
  GObjectClass parent_class;
} FooClass;

typedef struct {
  int dummy;
} FooPrivate;

static void
foo_class_init (FooClass *klass)
{
  g_type_class_add_private (klass, sizeof (FooPrivate));
}
