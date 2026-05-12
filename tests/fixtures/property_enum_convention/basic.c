#include <glib-object.h>

typedef struct _MyObject MyObject;
G_DECLARE_FINAL_TYPE (MyObject, my_object, MY, OBJECT, GObject)
struct _MyObject { GObject parent_instance; };
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)
static void my_object_init (MyObject *self) { }


/* Case 1: Old pattern with PROP_0 and N_PROPS */
typedef enum {
  PROP_0,
  PROP_NAME,
  PROP_TITLE,
  PROP_DESCRIPTION,
  N_PROPS
} MyObjectProperty;

static GParamSpec *my_props[N_PROPS] = { NULL, };

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  my_props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_TITLE] = g_param_spec_string ("title", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_DESCRIPTION] = g_param_spec_string ("description", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, N_PROPS, my_props);
}

typedef struct _Widget Widget;
G_DECLARE_FINAL_TYPE (Widget, widget, MY, WIDGET, GObject)
struct _Widget { GObject parent_instance; };
G_DEFINE_TYPE (Widget, widget, G_TYPE_OBJECT)
static void widget_init (Widget *self) { }

/* Case 2: Old pattern with prefix */
typedef enum {
  WIDGET_PROP_0,
  WIDGET_PROP_WIDTH,
  WIDGET_PROP_HEIGHT,
  WIDGET_N_PROPS
} WidgetProperty;

static GParamSpec *widget_props[WIDGET_N_PROPS] = { NULL, };

static void
widget_class_init (WidgetClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  widget_props[WIDGET_PROP_WIDTH] = g_param_spec_int ("width", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  widget_props[WIDGET_PROP_HEIGHT] = g_param_spec_int ("height", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, WIDGET_N_PROPS, widget_props);
}

typedef struct _Modern Modern;
G_DECLARE_FINAL_TYPE (Modern, modern, MY, MODERN, GObject)
struct _Modern { GObject parent_instance; };
G_DEFINE_TYPE (Modern, modern, G_TYPE_OBJECT)
static void modern_init (Modern *self) { }

/* Case 3: Already using modern pattern - should NOT be flagged */
typedef enum {
  MODERN_PROP_FOO = 1,
  MODERN_PROP_BAR,
  MODERN_PROP_BAZ
} ModernProperty;

static GParamSpec *modern_props[MODERN_PROP_BAZ + 1] = { NULL, };

static void
modern_class_init (ModernClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  modern_props[MODERN_PROP_FOO] = g_param_spec_string ("foo", NULL, NULL, NULL, G_PARAM_READWRITE);
  modern_props[MODERN_PROP_BAR] = g_param_spec_string ("bar", NULL, NULL, NULL, G_PARAM_READWRITE);
  modern_props[MODERN_PROP_BAZ] = g_param_spec_string ("baz", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (modern_props), modern_props);
}

/* Case 5: GParamSpec array in conditional block */
#ifdef ENABLE_FEATURE
typedef struct _Feature Feature;
G_DECLARE_FINAL_TYPE (Feature, feature, MY, FEATURE, GObject)
struct _Feature { GObject parent_instance; };
G_DEFINE_TYPE (Feature, feature, G_TYPE_OBJECT)
static void feature_init (Feature *self) { }
typedef enum {
  FEATURE_PROP_0,
  FEATURE_PROP_ENABLED,
  FEATURE_PROP_VALUE,
  FEATURE_N_PROPS
} FeatureProperty;

static GParamSpec *feature_props[FEATURE_N_PROPS] = { NULL, };

static void
feature_class_init (FeatureClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  feature_props[FEATURE_PROP_ENABLED] = g_param_spec_boolean ("enabled", NULL, NULL, FALSE, G_PARAM_READWRITE);
  feature_props[FEATURE_PROP_VALUE] = g_param_spec_int ("value", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, FEATURE_N_PROPS, feature_props);
}
#endif

