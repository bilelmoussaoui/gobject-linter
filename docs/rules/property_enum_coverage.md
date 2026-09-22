Checks that every value in the property enum has a corresponding
`g_param_spec_*` or `g_object_class_override_property` call.

## Why?

- **Dead enum values**: a property enum member without a matching
  `g_param_spec_*` call is dead code — the property is never registered and
  cannot be used
- **Refactoring safety**: when removing properties, it is easy to delete the
  install call and forget the enum value

## Examples

**Bad** (enum value without corresponding param spec):
```c
enum {
  PROP_0,
  PROP_TITLE,
  PROP_WIDTH,   /* no g_param_spec for this */
  N_PROPS,
};

/* class_init only installs PROP_TITLE */
```

**Good** (enum and param specs match):
```c
enum {
  PROP_0,
  PROP_TITLE,
  PROP_WIDTH,
  N_PROPS,
};

/* class_init installs both PROP_TITLE and PROP_WIDTH */
```
