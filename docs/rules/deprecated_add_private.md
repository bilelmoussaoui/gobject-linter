Flags calls to `g_type_class_add_private()`, which has been deprecated since
GLib 2.58.

## Why?

- **Deprecated API**: `g_type_class_add_private()` emits a runtime deprecation
  warning on every type registration, cluttering logs and test output
- **Simpler alternative**: `G_DEFINE_TYPE_WITH_PRIVATE` and `G_ADD_PRIVATE`
  handle private data allocation declaratively, without manual setup in
  `class_init`
- **Future removal**: deprecated API may be removed in a future GLib major
  version

## Examples

**Bad** (deprecated manual private registration):
```c
static void
my_obj_class_init (MyObjClass *klass)
{
  g_type_class_add_private (klass, sizeof (MyObjPrivate));
}
```

**Good** (declarative private data):
```c
G_DEFINE_TYPE_WITH_PRIVATE (MyObj, my_obj, G_TYPE_OBJECT)

static void
my_obj_class_init (MyObjClass *klass)
{
  /* private data is set up automatically */
}

static void
my_obj_init (MyObj *self)
{
  MyObjPrivate *priv = my_obj_get_instance_private (self);
  /* ... */
}
```
