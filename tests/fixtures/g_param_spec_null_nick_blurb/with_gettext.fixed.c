#include <glib-object.h>
#define _(s) (s)

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

enum { PROP_0, PROP_USERNAME, PROP_HOSTNAME };

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  /* Translation macros should be detected as violations */
  g_object_class_install_property (object_class, PROP_USERNAME,
    g_param_spec_string ("username", NULL, NULL,
                         NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  g_object_class_install_property (object_class, PROP_HOSTNAME,
    g_param_spec_string ("hostname", NULL, NULL,
                         NULL, G_PARAM_READWRITE | G_PARAM_STATIC_NAME));
}
