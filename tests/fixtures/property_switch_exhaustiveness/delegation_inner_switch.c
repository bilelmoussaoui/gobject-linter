#include <glib-object.h>

typedef struct {
  GObject parent_instance;
  char *name;
  gboolean editing;
} MyEditable;

typedef struct {
  GObjectClass parent_class;
} MyEditableClass;

G_DEFINE_TYPE (MyEditable, my_editable, G_TYPE_OBJECT)

enum {
  PROP_NAME = 1,
  PROP_EDITING,
  N_PROPS
};

static GParamSpec *props[N_PROPS] = { NULL, };

static void
my_editable_get_property (GObject    *object,
                          guint       prop_id,
                          GValue     *value,
                          GParamSpec *pspec)
{
  MyEditable *self = MY_EDITABLE (object);

  if (gtk_editable_delegate_get_property (object, prop_id, value, pspec))
    {
      switch (prop_id)
        {
        case PROP_NAME:
          break;
        default:
          break;
        }
      return;
    }

  switch (prop_id)
    {
    case PROP_NAME:
      g_value_set_string (value, self->name);
      break;
    case PROP_EDITING:
      g_value_set_boolean (value, self->editing);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
my_editable_set_property (GObject      *object,
                          guint         prop_id,
                          const GValue *value,
                          GParamSpec   *pspec)
{
  MyEditable *self = MY_EDITABLE (object);

  if (gtk_editable_delegate_set_property (object, prop_id, value, pspec))
    {
      switch (prop_id)
        {
        case PROP_NAME:
          break;
        default:
          break;
        }
      return;
    }

  switch (prop_id)
    {
    case PROP_NAME:
      g_free (self->name);
      self->name = g_value_dup_string (value);
      break;
    case PROP_EDITING:
      self->editing = g_value_get_boolean (value);
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
      break;
    }
}

static void
my_editable_class_init (MyEditableClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->get_property = my_editable_get_property;
  object_class->set_property = my_editable_set_property;

  props[PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  props[PROP_EDITING] = g_param_spec_boolean ("editing", NULL, NULL, FALSE, G_PARAM_READABLE);

  g_object_class_install_properties (object_class, N_PROPS, props);
}

static void
my_editable_init (MyEditable *self)
{
}
