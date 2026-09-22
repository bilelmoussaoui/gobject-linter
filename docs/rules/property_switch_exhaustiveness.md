Checks that `get_property` and `set_property` switch statements handle every
property enum case. Also warns when a `default:` branch is unreachable because
all cases are already covered.

## Why?

- **Missing handlers**: forgetting a case in `get_property` or `set_property`
  means the property silently does nothing when read or written, which is hard
  to debug
- **Dead default**: if every property enum value already has a `case`, the
  `default:` branch is unreachable dead code — it may mask a warning the
  compiler would otherwise emit when a new property is added

## Examples

**Bad** (missing case in set_property):
```c
static void
my_obj_set_property (GObject    *object,
                     guint       prop_id,
                     const GValue *value,
                     GParamSpec *pspec)
{
  switch (prop_id)
    {
    case PROP_TITLE:
      /* ... */
      break;
    /* PROP_WIDTH is missing */
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}
```

**Good** (all properties handled):
```c
static void
my_obj_set_property (GObject    *object,
                     guint       prop_id,
                     const GValue *value,
                     GParamSpec *pspec)
{
  switch (prop_id)
    {
    case PROP_TITLE:
      /* ... */
      break;
    case PROP_WIDTH:
      /* ... */
      break;
    default:
      G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}
```

## Notes

This rule supports `--fix` to automatically insert missing `case` branches.

## Configuration

### `style` (default: `"typed"`)

Property enum style. `"typed"` is strict — requires enum casts and checks that
all properties appear in both `get_property` and `set_property` switches.
`"legacy"` is relaxed — only checks read-write properties:

```toml
[rules.property_switch_exhaustiveness]
style = "legacy"
```

### `readable_flags` (default: `[]`)

Additional flag names indicating readable properties. `G_PARAM_READABLE` and
`G_PARAM_READWRITE` are always included:

```toml
[rules.property_switch_exhaustiveness]
readable_flags = ["MY_LIB_READABLE", "MY_LIB_READWRITE"]
```

### `writable_flags` (default: `[]`)

Additional flag names indicating writable properties. `G_PARAM_WRITABLE` and
`G_PARAM_READWRITE` are always included:

```toml
[rules.property_switch_exhaustiveness]
writable_flags = ["MY_LIB_WRITABLE", "MY_LIB_READWRITE"]
```
