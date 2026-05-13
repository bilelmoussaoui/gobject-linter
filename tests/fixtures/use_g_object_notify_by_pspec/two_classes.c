#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

typedef struct _Bar Bar;
G_DECLARE_FINAL_TYPE (Bar, bar, BAR, BAR, GObject)

struct _Bar {
  GObject parent_instance;
};

typedef struct _BazStringObject BazStringObject;
G_DECLARE_FINAL_TYPE (BazStringObject, baz_string_object, BAZ, STRING_OBJECT, GObject)

struct _BazStringObject {
  GObject parent_instance;
};

typedef Foo FooObject;
typedef Bar BarObject;

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }

G_DEFINE_TYPE (Bar, bar, G_TYPE_OBJECT)

static void bar_init (Bar *self) { }

G_DEFINE_FINAL_TYPE (BazStringObject, baz_string_object, G_TYPE_OBJECT)

static void baz_string_object_init (BazStringObject *self) { }

// First class
enum {
  FOO_PROP_0,
  FOO_PROP_NAME,
  FOO_PROP_TITLE,
  FOO_N_PROPS
};

static GParamSpec *foo_props[FOO_N_PROPS] = { NULL, };

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  foo_props[FOO_PROP_NAME] = g_param_spec_string ("name", NULL, NULL,
                                                   NULL, G_PARAM_READWRITE);

  foo_props[FOO_PROP_TITLE] = g_param_spec_string ("title", NULL, NULL,
                                                    NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, FOO_N_PROPS, foo_props);
}

// Second class
enum {
  BAR_PROP_0,
  BAR_PROP_NAME,
  BAR_PROP_LABEL,
  BAR_N_PROPS
};

static GParamSpec *bar_props[BAR_N_PROPS] = { NULL, };

static void
bar_class_init (BarClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  bar_props[BAR_PROP_NAME] = g_param_spec_string ("name", NULL, NULL,
                                                   NULL, G_PARAM_READWRITE);

  bar_props[BAR_PROP_LABEL] = g_param_spec_string ("label", NULL, NULL,
                                                    NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, BAR_N_PROPS, bar_props);
}

// Third class - type name ends in "Object" and prefix includes "object"
enum {
  BAZ_PROP_0,
  BAZ_PROP_NAME,
  BAZ_N_PROPS
};

static GParamSpec *baz_props[BAZ_N_PROPS] = { NULL, };

static void
baz_string_object_class_init (BazStringObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  baz_props[BAZ_PROP_NAME] = g_param_spec_string ("name", NULL, NULL,
                                                   NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, BAZ_N_PROPS, baz_props);
}

static void
foo_set_name (FooObject *self, const char *name)
{
  g_object_notify (G_OBJECT (self), "name");
  g_object_notify (G_OBJECT (self), "title");
}

static void
bar_set_name (BarObject *self, const char *name)
{
  g_object_notify (G_OBJECT (self), "name");
  g_object_notify (G_OBJECT (self), "label");
}

static void
baz_string_object_set_name (BazStringObject *self, const char *name)
{
  g_object_notify (G_OBJECT (self), "name");
}
