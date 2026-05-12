#include <glib-object.h>

typedef struct _Example Example;
G_DECLARE_FINAL_TYPE (Example, example, MY, EXAMPLE, GObject)
struct _Example { GObject parent_instance; };
G_DEFINE_TYPE (Example, example, G_TYPE_OBJECT)
static void example_init (Example *self) { }


/* Edge case: N_PROPS has an explicit value assignment (N_PROPS = PROP_ORIENTATION) */
typedef enum {
  PROP_VALID = 1,
  PROP_SPACING,
  PROP_PUZZLE_KIND,
  PROP_ORIENTATION,
} ExampleProps;

static GParamSpec *obj_props[PROP_PUZZLE_KIND + 1] = { NULL, };

static void
example_set_property (GObject    *object,
                      guint       prop_id,
                      const GValue *value,
                      GParamSpec *pspec)
{
  switch ((ExampleProps) prop_id)
    {
    case PROP_VALID:
      break;
    case PROP_SPACING:
      break;
    case PROP_PUZZLE_KIND:
      break;
    case PROP_ORIENTATION:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}

static void
example_get_property (GObject    *object,
                      guint       prop_id,
                      GValue     *value,
                      GParamSpec *pspec)
{
  switch ((ExampleProps) prop_id)
    {
    case PROP_VALID:
      break;
    case PROP_SPACING:
      break;
    case PROP_PUZZLE_KIND:
      break;
    case PROP_ORIENTATION:
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}

static void
example_class_init (ExampleClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  object_class->set_property = example_set_property;
  object_class->get_property = example_get_property;

  g_object_class_override_property (object_class, PROP_ORIENTATION, "orientation");

  obj_props[PROP_VALID] = g_param_spec_boolean ("valid", NULL, NULL, FALSE, G_PARAM_READWRITE);
  obj_props[PROP_SPACING] = g_param_spec_int ("spacing", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  obj_props[PROP_PUZZLE_KIND] = g_param_spec_int ("puzzle-kind", NULL, NULL, 0, 10, 0, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (obj_props), obj_props);
}
