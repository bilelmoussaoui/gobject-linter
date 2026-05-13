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
  PROP_TITLE,
  PROP_POSITION,
  N_PROPS
} FooProperty;

static GParamSpec *props[N_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);

  props[PROP_TITLE] = g_param_spec_override ("title",
      g_object_interface_find_property (g_type_default_interface_ref (MY_TYPE_IFACE), "title"));
  props[PROP_POSITION] = g_param_spec_override ("position",
      g_object_interface_find_property (g_type_default_interface_ref (MY_TYPE_IFACE), "position"));

  g_object_class_install_properties (object_class, N_PROPS, props);
}
