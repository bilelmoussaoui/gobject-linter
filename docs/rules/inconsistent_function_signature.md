Detects functions whose declaration (in a header) and definition (in a `.c`
file) disagree on return type, parameter types, or parameter count.

## Why?

- **Silent bugs**: C allows implicit conversions between compatible types, so a
  mismatched signature may compile without warnings yet produce wrong results
  at runtime (e.g. truncated return values, wrong signedness)
- **ABI breakage**: when the header says `gboolean` but the definition returns
  `gint`, callers compiled against the header may interpret the value
  differently
- **Maintenance hazard**: a signature changed in one place but not the other is
  a common source of regressions after refactoring

## What is checked

- **Return type** differences between declaration and definition
- **Parameter count** mismatches
- **Parameter type** differences (position by position)
- **Static functions** with a forward declaration in the same `.c` file
- Types must be known (from typedefs, structs, enums, or GObject type macros)
  to avoid false positives from unresolved preprocessor defines

## Examples

**Flagged** (return type mismatch):
```c
/* header */
gboolean foo_is_valid (void);

/* source — returns gint instead of gboolean */
gint
foo_is_valid (void)
{
  return TRUE;
}
```

**Flagged** (parameter type mismatch):
```c
/* header */
void quux_set_value (GObject *obj, gboolean enabled);

/* source — gboolean changed to gint */
void
quux_set_value (GObject *obj, gint enabled)
{
}
```

**Not flagged** (matching signatures):
```c
/* header */
gboolean foo_is_valid (void);

/* source */
gboolean
foo_is_valid (void)
{
  return TRUE;
}
```
