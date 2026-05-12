#include <glib-object.h>

typedef struct _BadSpacing BadSpacing;
G_DECLARE_FINAL_TYPE (BadSpacing, bad_spacing, BAD, SPACING, GObject)
struct _BadSpacing { GObject parent_instance; };
G_DEFINE_TYPE (BadSpacing, bad_spacing, G_TYPE_OBJECT)
static void bad_spacing_init (BadSpacing *self) { }


/* Case 1: First property already has = 0 with bad spacing */
typedef enum {
  PROP_BOUNDING_BOX = 1,
  PROP_CHILD,
} BadSpacingProperty;

static GParamSpec *bad_spacing_props[PROP_CHILD + 1] = { NULL, };

static void
bad_spacing_class_init (BadSpacingClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  bad_spacing_props[PROP_BOUNDING_BOX] = g_param_spec_int ("bounding-box", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  bad_spacing_props[PROP_CHILD] = g_param_spec_object ("child", NULL, NULL, G_TYPE_OBJECT, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (bad_spacing_props), bad_spacing_props);
}

typedef struct _Legacy Legacy;
G_DECLARE_FINAL_TYPE (Legacy, legacy, MY, LEGACY, GObject)
struct _Legacy { GObject parent_instance; };
G_DEFINE_TYPE (Legacy, legacy, G_TYPE_OBJECT)
static void legacy_init (Legacy *self) { }

/* Case 2: Very old code using NUM_PROPERTIES */
typedef enum {
  LEGACY_PROP_FOO = 1,
  LEGACY_PROP_BAR,
} LegacyProperty;

static GParamSpec *legacy_props[LEGACY_PROP_BAR + 1] = { NULL, };

static void
legacy_class_init (LegacyClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  legacy_props[LEGACY_PROP_FOO] = g_param_spec_string ("foo", NULL, NULL, NULL, G_PARAM_READWRITE);
  legacy_props[LEGACY_PROP_BAR] = g_param_spec_string ("bar", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (legacy_props), legacy_props);
}
