#include <glib-object.h>

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)


enum {
  PROP_0,
  PROP_SESSION_NAME,
  PROP_ID
};

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);
  GParamSpec *param_spec;

  param_spec = g_param_spec_string ("session-name", NULL, NULL,
                                    NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME);
  g_object_class_install_property (object_class, PROP_SESSION_NAME, param_spec);

  param_spec = g_param_spec_string ("id", NULL, NULL,
                                    NULL, G_PARAM_READWRITE);
  g_object_class_install_property (object_class, PROP_ID, param_spec);
}
