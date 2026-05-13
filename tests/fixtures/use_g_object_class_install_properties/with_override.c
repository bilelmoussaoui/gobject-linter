#include <glib-object.h>

/* Interface definition */
typedef struct _MyIface MyIface;

#define MY_TYPE_IFACE (my_iface_get_type ())
G_DECLARE_INTERFACE (MyIface, my_iface, MY, IFACE, GObject)

struct _MyIfaceInterface {
  GTypeInterface parent_iface;
};

G_DEFINE_INTERFACE (MyIface, my_iface, G_TYPE_OBJECT)

static void
my_iface_default_init (MyIfaceInterface *iface)
{
  g_object_interface_install_property (iface,
    g_param_spec_string ("title", NULL, NULL, NULL, G_PARAM_READWRITE));
  g_object_interface_install_property (iface,
    g_param_spec_int ("position", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE));
}

/* Concrete class implementing MyIface */
typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

static void foo_my_iface_init (MyIfaceInterface *iface) { }

G_DEFINE_TYPE_WITH_CODE (Foo, foo, G_TYPE_OBJECT,
                         G_IMPLEMENT_INTERFACE (MY_TYPE_IFACE, foo_my_iface_init))

static void foo_init (Foo *self) { }

typedef enum {
  PROP_0,
  PROP_NAME,
  N_REAL_PROPS,

  PROP_TITLE = N_REAL_PROPS,
  PROP_POSITION,
  N_PROPS
} FooProperty;

static GParamSpec *props[N_REAL_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  g_object_class_install_property (object_class, PROP_NAME,
    g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE));

  g_object_class_override_property (object_class, PROP_TITLE, "title");
  g_object_class_override_property (object_class, PROP_POSITION, "position");
}
