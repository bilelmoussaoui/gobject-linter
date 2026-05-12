#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }


enum {
  PROP_0,
  PROP_NAME,
  PROP_VALUE
};

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  g_object_class_install_property (object_class, PROP_NAME,
    g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE));

  g_object_class_install_property (object_class, PROP_VALUE,
    g_param_spec_int ("value", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE));
}
