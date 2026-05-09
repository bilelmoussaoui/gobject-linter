#include <glib-object.h>

typedef struct {
  GObject parent_instance;
  char *name;
  char *password;
  char *secret;
  char *token;
  char *status;
  int progress;
} MyObject;

typedef struct {
  GObjectClass parent_class;
} MyObjectClass;

G_DEFINE_TYPE (MyObject, my_object, G_TYPE_OBJECT)

typedef enum {
  PROP_NAME = 1,
  PROP_PASSWORD,
  PROP_SECRET,
  PROP_TOKEN,
  PROP_STATUS,
  PROP_PROGRESS,
} MyObjectProperty;

static GParamSpec *props[PROP_PROGRESS + 1] = { NULL, };

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
    case PROP_STATUS:
      g_value_set_string (value, self->status);
      break;
    case PROP_PROGRESS:
      g_value_set_int (value, self->progress);
      break;
    case PROP_PASSWORD:
    case PROP_SECRET:
    case PROP_TOKEN:
      g_assert_not_reached ();
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
    case PROP_PASSWORD:
      g_free (self->password);
      self->password = g_value_dup_string (value);
      break;
    case PROP_SECRET:
      g_free (self->secret);
      self->secret = g_value_dup_string (value);
      break;
    case PROP_TOKEN:
      g_free (self->token);
      self->token = g_value_dup_string (value);
      break;
    case PROP_STATUS:
    case PROP_PROGRESS:
      g_assert_not_reached ();
      break;
    }
}

static void
my_object_class_init (MyObjectClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_object_get_property;
  object_class->set_property = my_object_set_property;

  props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  props[PROP_PASSWORD] = g_param_spec_string ("password", NULL, NULL, NULL, G_PARAM_WRITABLE);
  props[PROP_SECRET] = g_param_spec_string ("secret", NULL, NULL, NULL, G_PARAM_WRITABLE);
  props[PROP_TOKEN] = g_param_spec_string ("token", NULL, NULL, NULL, G_PARAM_WRITABLE);
  props[PROP_STATUS] = g_param_spec_string ("status", NULL, NULL, NULL, G_PARAM_READABLE);
  props[PROP_PROGRESS] = g_param_spec_int ("progress", NULL, NULL, 0, 100, 0, G_PARAM_READABLE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (props), props);
}

static void
my_object_init (MyObject *self)
{
}
