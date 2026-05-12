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
  PROP_SESSION_NAME,
  PROP_ID,
  N_PROPS
};

static GParamSpec *props[N_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  props[PROP_SESSION_NAME] = g_param_spec_string ("session-name", NULL, NULL,
                                                  NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME);

  props[PROP_ID] = g_param_spec_string ("id", NULL, NULL,
                                        NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, N_PROPS, props);
}
