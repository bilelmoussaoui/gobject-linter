#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }


/* Signals enum appears first - must not be confused with the property enum */
enum {
  SYNC_MESSAGE,
  ASYNC_MESSAGE,
  LAST_SIGNAL
};

enum {
  PROP_0,
  PROP_ENABLE_ASYNC,
  N_PROPS
};

static GParamSpec *props[N_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  props[PROP_ENABLE_ASYNC] = g_param_spec_boolean ("enable-async", NULL, NULL, TRUE,
                                                   G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);

  g_object_class_install_properties (object_class, N_PROPS, props);
}
