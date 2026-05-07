#include <glib-object.h>

G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)


/* Modern pattern with correct array size */
typedef enum {
  PROP_NAME = 1,
  PROP_TITLE,
  PROP_DESCRIPTION
} MyObjectProperty;

static GParamSpec *my_props[PROP_DESCRIPTION + 1] = { NULL, };

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  my_props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_TITLE] = g_param_spec_string ("title", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_DESCRIPTION] = g_param_spec_string ("description", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (my_props), my_props);
}
