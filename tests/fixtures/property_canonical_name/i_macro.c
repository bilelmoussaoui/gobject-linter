#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, MY, FOO, GObject)
struct _Foo { GObject parent_instance; };
G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)
static void foo_init (Foo *self) { }

enum { PROP_STACK = 1 };
static GParamSpec *obj_properties[2] = { NULL, };

#define I_(s) (s)

/* Property name via I_() with no underscores — should not be flagged. */
static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  obj_properties[PROP_STACK] =
    g_param_spec_object (I_("stack"), NULL, NULL,
                         G_TYPE_OBJECT,
                         G_PARAM_READWRITE|G_PARAM_STATIC_STRINGS|G_PARAM_EXPLICIT_NOTIFY);
}
