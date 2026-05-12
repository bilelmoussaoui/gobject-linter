#include <glib-object.h>

typedef struct _GdmDisplay GdmDisplay;
G_DECLARE_FINAL_TYPE (GdmDisplay, gdm_display, GDM, DISPLAY, GObject)

struct _GdmDisplay {
  GObject parent_instance;
};

G_DEFINE_TYPE (GdmDisplay, gdm_display, G_TYPE_OBJECT)

static void gdm_display_init (GdmDisplay *self) { }


enum {
        PROP_0,
        PROP_ID,
        PROP_STATUS,
        PROP_SEAT_ID,
        PROP_SESSION_ID,
        PROP_SESSION_CLASS,
        PROP_REMOTE_HOSTNAME,
        PROP_IS_LOCAL,
        PROP_LAUNCH_ENVIRONMENT,
        PROP_IS_INITIAL,
        PROP_AUTOLOGIN_USER,
        PROP_ALLOW_TIMED_LOGIN,
        PROP_HAVE_EXISTING_USER_ACCOUNTS,
        PROP_DOING_INITIAL_SETUP,
        PROP_SESSION_REGISTERED,
        PROP_SUPPORTED_SESSION_TYPES,
};

static void
gdm_display_class_init (GdmDisplayClass *klass)
{
        GObjectClass *object_class = G_OBJECT_CLASS (klass);

        g_object_class_install_property (object_class,
                                         PROP_ID,
                                         g_param_spec_string ("id",
                                                             NULL, NULL,
                                                             NULL,
                                                             G_PARAM_READWRITE | G_PARAM_CONSTRUCT));

        g_object_class_install_property (object_class,
                                         PROP_SUPPORTED_SESSION_TYPES,
                                         g_param_spec_boxed ("supported-session-types",
                                                             NULL, NULL,
                                                             G_TYPE_STRV,
                                                             G_PARAM_READWRITE | G_PARAM_CONSTRUCT | G_PARAM_STATIC_NAME));
}
