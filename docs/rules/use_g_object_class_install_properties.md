Detects multiple `g_object_class_install_property()` calls in a `class_init`
function and suggests replacing them with a single
`g_object_class_install_properties()` call using a static `GParamSpec*` array.

## Why?

- **Performance**: `g_object_class_install_properties()` installs all properties
  in one pass, avoiding repeated internal lookups and signal emissions
- **Notify by pspec**: with a static properties array, you can use
  `g_object_notify_by_pspec(obj, props[PROP_FOO])` instead of the slower
  string-based `g_object_notify(obj, "foo")`
- **Less boilerplate**: a single bulk call is more concise than N individual
  install calls

## Examples

**Bad** (individual installation):
```c
static void
my_obj_class_init (MyObjClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  g_object_class_install_property (object_class, PROP_NAME,
      g_param_spec_string ("name", NULL, NULL, NULL,
                           G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS));
  g_object_class_install_property (object_class, PROP_VALUE,
      g_param_spec_int ("value", NULL, NULL, 0, 100, 0,
                        G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS));
}
```

**Good** (bulk installation with array):
```c
static GParamSpec *props[N_PROPS] = { NULL, };

static void
my_obj_class_init (MyObjClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  props[PROP_NAME] =
      g_param_spec_string ("name", NULL, NULL, NULL,
                           G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
  props[PROP_VALUE] =
      g_param_spec_int ("value", NULL, NULL, 0, 100, 0,
                        G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);

  g_object_class_install_properties (object_class, N_PROPS, props);
}
```

## Notes

Requires GLib >= 2.26.

This rule supports `--fix` to automatically convert to bulk `g_object_class_install_properties`.
