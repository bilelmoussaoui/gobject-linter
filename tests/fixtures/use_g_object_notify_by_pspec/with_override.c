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

/* Class using g_param_spec_override with array (fixable) */
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

  g_object_class_install_properties (object_class, N_PROPS, props);
}

/* Class using g_object_class_override_property (not fixable) */
typedef struct _Bar Bar;
G_DECLARE_FINAL_TYPE (Bar, bar, BAR, BAR, GObject)

struct _Bar {
  GObject parent_instance;
};

static void bar_my_iface_init (MyIfaceInterface *iface) { }

G_DEFINE_TYPE_WITH_CODE (Bar, bar, G_TYPE_OBJECT,
                         G_IMPLEMENT_INTERFACE (MY_TYPE_IFACE, bar_my_iface_init))

static void bar_init (Bar *self) { }

typedef enum {
  BAR_PROP_0,
  BAR_PROP_POSITION,
  BAR_N_PROPS
} BarProperty;

static void
bar_class_init (BarClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  g_object_class_override_property (object_class, BAR_PROP_POSITION, "position");
}

static void
foo_set_title (Foo *self, const char *title)
{
  g_object_notify (G_OBJECT (self), "name");
  g_object_notify (G_OBJECT (self), "title");
}

static void
bar_set_position (Bar *self, int position)
{
  g_object_notify (G_OBJECT (self), "position");
}
