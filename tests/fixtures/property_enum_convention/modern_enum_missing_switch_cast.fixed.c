#include <glib-object.h>

typedef struct _MyObject MyObject;
G_DECLARE_FINAL_TYPE (MyObject, my_object, MY, OBJECT, GObject)
struct _MyObject { GObject parent_instance; char *name; char *title; };
G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)
static void my_object_init (MyObject *self) { }


/* Modern named enum but missing switch cast */
typedef enum {
  PROP_NAME = 1,
  PROP_TITLE,
  PROP_AGE
} MyObjectProps;

static GParamSpec *my_props[PROP_AGE + 1] = { NULL, };

static void
my_object_get_property (GObject    *object,
                        guint       prop_id,
                        GValue     *value,
                        GParamSpec *pspec)
{
  MyObject *self = MY_OBJECT (object);

  switch ((MyObjectProps) prop_id)
    {
    case PROP_NAME:
      g_value_set_string (value, self->name);
      break;
    case PROP_TITLE:
      g_value_set_string (value, self->title);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
my_object_set_property (GObject      *object,
                        guint         prop_id,
                        const GValue *value,
                        GParamSpec   *pspec)
{
  MyObject *self = MY_OBJECT (object);

  switch ((MyObjectProps) prop_id)
    {
    case PROP_NAME:
      g_free (self->name);
      self->name = g_value_dup_string (value);
      break;
    case PROP_TITLE:
      g_free (self->title);
      self->title = g_value_dup_string (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  my_props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_TITLE] = g_param_spec_string ("title", NULL, NULL, NULL, G_PARAM_READWRITE);
  my_props[PROP_AGE] = g_param_spec_int ("age", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (my_props), my_props);
}
