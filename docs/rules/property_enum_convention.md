Enforces a consistent style for the GObject property enumeration. Two styles
are supported via configuration:

- **typed** (default): each property gets a named constant starting at 1
  (`PROP_TITLE = 1, PROP_WIDTH, …`), no `PROP_0` sentinel and no `N_PROPS`
  count. The enum itself should be a typed `enum` (not anonymous).
- **legacy**: traditional GObject convention with a `PROP_0` sentinel at the
  start and `N_PROPS` count at the end, inside an anonymous `enum`.

## Why?

- **Consistency**: mixing the two styles within a project confuses contributors
  and makes automated refactoring harder
- **Typed enums**: the typed style produces real enum types the compiler can
  check, and eliminates unused sentinel values
- **Legacy compatibility**: some projects or coding standards require the
  traditional `PROP_0` / `N_PROPS` pattern — the rule supports both

## Examples

**Bad** (mixed style — has PROP_0 but also typed enum):
```c
typedef enum {
  PROP_0,
  PROP_TITLE = 1,
  PROP_WIDTH,
} MyObjProperty;
```

**Good** (typed style):
```c
typedef enum {
  PROP_TITLE = 1,
  PROP_WIDTH,
} MyObjProperty;
```

**Good** (legacy style):
```c
enum {
  PROP_0,
  PROP_TITLE,
  PROP_WIDTH,
  N_PROPS,
};
```

## Configuration

```toml
[rules.property_enum_convention]
options = { style = "typed" }   # or "legacy"
```

This rule supports `--fix` to automatically convert between styles.
