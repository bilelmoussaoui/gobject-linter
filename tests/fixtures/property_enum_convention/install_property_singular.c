
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)

// Test case for g_object_class_install_property (singular) detection
typedef struct _MyObject MyObject;
typedef struct _MyObjectClass MyObjectClass;

enum {
  PROP_0,
  PROP_FOO,
  PROP_BAR,
  N_PROPS
};

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
  GParamSpec *pspec;

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  pspec = g_param_spec_string ("foo", NULL, NULL, NULL, G_PARAM_READWRITE);
  g_object_class_install_property (object_class, PROP_FOO, pspec);

  pspec = g_param_spec_int ("bar", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  g_object_class_install_property (object_class, PROP_BAR, pspec);
}
