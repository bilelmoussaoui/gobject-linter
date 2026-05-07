#include <glib-object.h>

G_DEFINE_TYPE (MetaScreenCastSession, meta_screen_cast_session, G_TYPE_OBJECT)


typedef enum {
  PROP_0,
  PROP_REMOTE_DESKTOP_SESSION,
  N_PROPS
} MetaScreenCastSessionProperty;

static GParamSpec *props[N_PROPS] = { NULL, };

static void
meta_screen_cast_session_class_init (MetaScreenCastSessionClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = meta_screen_cast_session_get_property;
  object_class->set_property = meta_screen_cast_session_set_property;

  g_object_class_install_properties (object_class, N_PROPS, props);
}

static void
meta_screen_cast_session_set_property (GObject      *object,
                                       guint         prop_id,
                                       const GValue *value,
                                       GParamSpec   *pspec)
{
  MetaScreenCastSession *session = META_SCREEN_CAST_SESSION (object);

  switch (prop_id)
    {
    case PROP_REMOTE_DESKTOP_SESSION:
      session->remote_desktop_session = g_value_get_object (value);
      break;
    case N_PROPS + META_DBUS_SESSION_PROP_SESSION_MANAGER:
      session->session_manager = g_value_get_object (value);
      break;
    case N_PROPS + META_DBUS_SESSION_PROP_PEER_NAME:
      session->peer_name = g_value_dup_string (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}
