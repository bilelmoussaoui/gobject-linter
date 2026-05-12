#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, MY, FOO, GObject)
struct _Foo { GObject parent_instance; };
G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)
static void foo_init (Foo *self) { }

enum { PROP_DISPLAY_NAME = 1 };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  g_object_class_install_property (object_class, PROP_DISPLAY_NAME,
    g_param_spec_string ("display_name", NULL, NULL, NULL, G_PARAM_READWRITE));
}
