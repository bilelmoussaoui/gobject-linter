#include <glib-object.h>

G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)


typedef enum {
  PROP_NAME = 1,
  PROP_TITLE,
} MyObjectProperty;

static GParamSpec *my_props[PROP_TITLE + 1] = { NULL, };

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  g_object_class_install_properties (object_class, G_N_ELEMENTS (my_props), my_props);
}

static void
my_object_get_property (GObject    *object,
                        guint       prop_id,
                        GValue     *value,
                        GParamSpec *pspec)
{
  MyObject *self = MY_OBJECT (object);

  switch ((MyObjectProperty) prop_id)
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

  switch ((MyObjectProperty) prop_id)
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
