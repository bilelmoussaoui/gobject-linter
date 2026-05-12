#include <glib-object.h>

typedef struct _MetaBackendNative MetaBackendNative;
typedef MetaBackendNative MetaBackendClass;
G_DECLARE_FINAL_TYPE (MetaBackendNative, meta_backend_native, META, BACKEND_NATIVE, GObject)
struct _MetaBackendNative { GObject parent_instance; int mode; };
typedef struct _MetaBackendNative MetaBackendNativePrivate;
G_DEFINE_TYPE (MetaBackendNative, meta_backend_native, G_TYPE_OBJECT)
static void meta_backend_native_init (MetaBackendNative *self) { }


enum
{
  PROP_0,

  PROP_MODE,

  N_PROPS
};

static GParamSpec *obj_props[N_PROPS];

static void
meta_backend_native_set_property (GObject      *object,
                                  guint         prop_id,
                                  const GValue *value,
                                  GParamSpec   *pspec)
{
  MetaBackendNative *backend_native = META_BACKEND_NATIVE (object);
  MetaBackendNativePrivate *priv =
    meta_backend_native_get_instance_private (backend_native);

  switch (prop_id)
    {
    case PROP_MODE:
      priv->mode = g_value_get_int (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
meta_backend_native_class_init (MetaBackendNativeClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->set_property = meta_backend_native_set_property;

  obj_props[PROP_MODE] =
    g_param_spec_int ("mode", NULL, NULL,
                      0, 2, 0,
                      G_PARAM_WRITABLE |
                      G_PARAM_CONSTRUCT_ONLY |
                      G_PARAM_STATIC_STRINGS);
  g_object_class_install_properties (object_class, N_PROPS, obj_props);
}
