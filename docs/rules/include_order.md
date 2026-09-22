Enforces a consistent `#include` ordering, separated into groups with blank
lines between them.

## Why?

- **Catches missing includes**: placing the config header first and the
  associated header before system headers ensures that each header is
  self-contained — if it silently depends on a prior include, the build breaks
  immediately
- **Readability**: a predictable order makes it easy to spot whether a header
  is already included
- **Merge conflicts**: alphabetical sorting within groups reduces conflicts
  when multiple branches add includes to the same file

## Expected order

1. **Config header** (`config.h` by default) — must be first when present
2. **Associated header** — for `foo.c`, this is `foo.h` (also matches
   `foo-private.h` / `fooprivate.h`)
3. **Standard C / POSIX headers** — `<stdio.h>`, `<unistd.h>`, etc.
4. **System / library headers** — `<glib.h>`, `<gtk/gtk.h>`, etc.
5. **Project headers** — `"bar.h"`, `"utils/helpers.h"`, etc.

Within each group, headers are sorted alphabetically. Groups are separated by
a blank line.

## Examples

**Bad** (mixed order):
```c
#include "foo.h"
#include <glib.h>
#include <stdio.h>
#include "config.h"
#include "bar.h"
```

**Good** (correct order):
```c
#include "config.h"

#include <stdio.h>

#include <glib.h>

#include "bar.h"
#include "foo.h"
```

## Configuration

### `config_header` (default: `"config.h"`)

Set this if your project uses a differently named config header:

```toml
[rules.include_order]
config_header = "myproject-config.h"
```

This rule supports `--fix` to automatically reorder includes.
