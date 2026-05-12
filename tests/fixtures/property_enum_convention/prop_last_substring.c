#include <glib-object.h>

typedef struct _MyObject MyObject;
G_DECLARE_FINAL_TYPE (MyObject, my_object, MY, OBJECT, GObject)
struct _MyObject { GObject parent_instance; };
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)
static void my_object_init (MyObject *self) { }

// Test case where a property name contains the sentinel as a substring
// e.g., PROP_LAST_CHILD contains PROP_LAST

enum
{
  PROP_0,
  PROP_FIRST_CHILD,
  PROP_LAST_CHILD,
  PROP_LAST
};

static GParamSpec *obj_props[PROP_LAST];

static void
my_object_get_property (GObject *object,
                        guint property_id,
                        GValue *value,
                        GParamSpec *pspec)
{
  switch (property_id)
    {
    case PROP_FIRST_CHILD:
      break;
    case PROP_LAST_CHILD:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, property_id, pspec);
    }
}

static void
my_object_set_property (GObject *object,
                        guint property_id,
                        const GValue *value,
                        GParamSpec *pspec)
{
  switch (property_id)
    {
    case PROP_FIRST_CHILD:
      break;
    case PROP_LAST_CHILD:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, property_id, pspec);
    }
}

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  obj_props[PROP_FIRST_CHILD] = g_param_spec_string ("first-child", NULL, NULL,
                                                      NULL,
                                                      G_PARAM_READABLE);

  obj_props[PROP_LAST_CHILD] = g_param_spec_string ("last-child", NULL, NULL,
                                                     NULL,
                                                     G_PARAM_READABLE);

  g_object_class_install_properties (object_class, PROP_LAST, obj_props);
}
