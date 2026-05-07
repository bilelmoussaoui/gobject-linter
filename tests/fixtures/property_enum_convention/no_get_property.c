#include <glib-object.h>

G_DEFINE_TYPE (MetaBackendNative, meta_backend_native, G_TYPE_OBJECT)


enum
{
  PROP_0,

  PROP_MODE,

  N_PROPS
};

static GParamSpec *obj_props[N_PROPS];

static void
meta_backend_native_class_init (MetaBackendNativeClass *klass)
{
  MetaBackendClass *backend_class = META_BACKEND_CLASS (klass);
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->set_property = meta_backend_native_set_property;
  object_class->dispose = meta_backend_native_dispose;

  obj_props[PROP_MODE] =
    g_param_spec_enum ("mode", NULL, NULL,
                       META_TYPE_BACKEND_NATIVE_MODE,
                       META_BACKEND_NATIVE_MODE_DEFAULT,
                       G_PARAM_WRITABLE |
                       G_PARAM_CONSTRUCT_ONLY |
                       G_PARAM_STATIC_STRINGS);
  g_object_class_install_properties (object_class, N_PROPS, obj_props);
}

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
      priv->mode = g_value_get_enum (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}
