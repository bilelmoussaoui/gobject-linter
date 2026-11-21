Suggests the C99 `<tgmath.h>` type-generic math macros instead of explicit
`float`/`long double` variants like `sinf()`.

## Why?

- **Flexibility**: `sin()` dispatches to `sinf()`, `sin()` or `sinl()`
  depending on the argument type, so the code keeps working when the type
  changes
- **Less noise**: no need to keep the call in sync with the variable type

## Examples

**Bad**:
```c
#include <math.h>

float
f (float x)
{
  return sinf (x);
}
```

**Good**:
```c
#include <tgmath.h>

float
f (float x)
{
  return sin (x);
}
```

Only files including `<math.h>` without `<tgmath.h>` are checked, and only
functions that have a type-generic macro in `<tgmath.h>` (per C99) are
flagged — e.g. `log2f()` or `roundf()` are left alone.

This rule supports `--fix`: it renames the calls and replaces the
`<math.h>` include with `<tgmath.h>`.
