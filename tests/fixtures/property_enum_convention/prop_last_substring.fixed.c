
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)

// Test case where a property name contains the sentinel as a substring
// e.g., PROP_LAST_CHILD contains PROP_LAST
typedef struct _MyObject MyObject;
typedef struct _MyObjectClass MyObjectClass;

typedef enum
{
  PROP_FIRST_CHILD = 1,
  PROP_LAST_CHILD,
} MyObjectProps;

static GParamSpec *obj_props[PROP_LAST_CHILD + 1];

static void
my_object_get_property (MyObject *self,
                        guint property_id,
                        GValue *value,
                        GParamSpec *pspec)
{
  switch ((MyObjectProps) property_id)
    {
    case PROP_FIRST_CHILD:
      break;
    case PROP_LAST_CHILD:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (self, property_id, pspec);
    }
}

static void
my_object_set_property (MyObject *self,
                        guint property_id,
                        const GValue *value,
                        GParamSpec *pspec)
{
  switch ((MyObjectProps) property_id)
    {
    case PROP_FIRST_CHILD:
      break;
    case PROP_LAST_CHILD:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (self, property_id, pspec);
    }
}

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  obj_props[PROP_FIRST_CHILD] = g_param_spec_object ("first-child", NULL, NULL,
                                                      MY_TYPE_OBJECT,
                                                      G_PARAM_READABLE);

  obj_props[PROP_LAST_CHILD] = g_param_spec_object ("last-child", NULL, NULL,
                                                     MY_TYPE_OBJECT,
                                                     G_PARAM_READABLE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (obj_props), obj_props);
}
