#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

typedef Foo FooObject;

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }

enum {
  PROP_0,
  PROP_NAME,
  N_PROPS
};

static GParamSpec *props[N_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL,
                                          NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, N_PROPS, props);
}

static void
foo_update_child (FooObject *self, GObject *child)
{
  // This is our own property - should be detected
  g_object_notify_by_pspec (G_OBJECT (self), props[PROP_NAME]);

  // This is on a different object - should NOT be detected/fixed
  g_object_notify (child, "title");
}
