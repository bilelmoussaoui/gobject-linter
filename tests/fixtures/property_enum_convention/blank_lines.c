
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)

// Test case for blank line removal
// Blank lines after PROP_0 and before N_PROPS should be removed
typedef struct _MyObject MyObject;
typedef struct _MyObjectClass MyObjectClass;

enum
{
  PROP_0,

  PROP_FOO,
  PROP_BAR,

  N_PROPS
};

static GParamSpec *obj_props[N_PROPS];

static void
my_object_get_property (MyObject *self,
                        guint property_id,
                        GValue *value,
                        GParamSpec *pspec)
{
  switch (property_id)
    {
    case PROP_FOO:
      break;
    case PROP_BAR:
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
  switch (property_id)
    {
    case PROP_FOO:
      break;
    case PROP_BAR:
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

  obj_props[PROP_FOO] = g_param_spec_int ("foo", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  obj_props[PROP_BAR] = g_param_spec_int ("bar", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, N_PROPS, obj_props);
}
