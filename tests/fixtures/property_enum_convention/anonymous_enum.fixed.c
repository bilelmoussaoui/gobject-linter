#include <glib-object.h>

typedef struct _ClutterActor ClutterActor;
G_DECLARE_FINAL_TYPE (ClutterActor, clutter_actor, CLUTTER, ACTOR, GObject)
struct _ClutterActor {
  GObject parent_instance;
  GObject *actor;
  char *name;
  gboolean enabled;
};
G_DEFINE_TYPE (ClutterActor, clutter_actor, G_TYPE_OBJECT)
static void clutter_actor_init (ClutterActor *self) { }


typedef enum
{
  PROP_ACTOR = 1,
  PROP_NAME,
  PROP_ENABLED,
} ClutterActorProps;

static GParamSpec *clutter_actor_props[PROP_ENABLED + 1] = { NULL, };

static void
clutter_actor_get_property (GObject    *object,
                            guint       prop_id,
                            GValue     *value,
                            GParamSpec *pspec)
{
  ClutterActor *self = CLUTTER_ACTOR (object);

  switch ((ClutterActorProps) prop_id)
    {
    case PROP_ACTOR:
      g_value_set_object (value, self->actor);
      break;
    case PROP_NAME:
      g_value_set_string (value, self->name);
      break;
    case PROP_ENABLED:
      g_value_set_boolean (value, self->enabled);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
clutter_actor_set_property (GObject      *object,
                            guint         prop_id,
                            const GValue *value,
                            GParamSpec   *pspec)
{
  ClutterActor *self = CLUTTER_ACTOR (object);

  switch ((ClutterActorProps) prop_id)
    {
    case PROP_ACTOR:
      self->actor = g_value_dup_object (value);
      break;
    case PROP_NAME:
      g_free (self->name);
      self->name = g_value_dup_string (value);
      break;
    case PROP_ENABLED:
      self->enabled = g_value_get_boolean (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
clutter_actor_class_init (ClutterActorClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = clutter_actor_get_property;
  object_class->set_property = clutter_actor_set_property;

  g_object_class_install_properties (object_class, G_N_ELEMENTS (clutter_actor_props), clutter_actor_props);
}
