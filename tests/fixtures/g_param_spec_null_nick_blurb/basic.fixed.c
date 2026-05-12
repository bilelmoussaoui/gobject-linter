#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

enum { PROP_0, PROP_A, PROP_B, PROP_C, PROP_D, PROP_E, PROP_F, PROP_G };

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  /* Both nick and blurb are string literals, no static flags */
  g_object_class_install_property (object_class, PROP_A,
    g_param_spec_string ("prop-a", NULL, NULL, NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Both nick and blurb are string literals, has G_PARAM_STATIC_STRINGS */
  g_object_class_install_property (object_class, PROP_B,
    g_param_spec_string ("prop-b", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Both nick and blurb are string literals, has individual static flags */
  g_object_class_install_property (object_class, PROP_C,
    g_param_spec_string ("prop-c", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Only nick is a string literal, blurb is NULL, no static flags */
  g_object_class_install_property (object_class, PROP_D,
    g_param_spec_string ("prop-d", NULL, NULL, NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Only blurb is a string literal, nick is NULL, no static flags */
  g_object_class_install_property (object_class, PROP_E,
    g_param_spec_string ("prop-e", NULL, NULL, NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Only nick is a string literal, has G_PARAM_STATIC_NAME: flags already correct after fix */
  g_object_class_install_property (object_class, PROP_F,
    g_param_spec_string ("prop-f", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Both are already NULL: no violation */
  g_object_class_install_property (object_class, PROP_G,
    g_param_spec_string ("prop-g", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));
}
