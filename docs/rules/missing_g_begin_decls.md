Checks that public header files wrap their declarations in
`G_BEGIN_DECLS` / `G_END_DECLS`, and that the pair is not mismatched.

## Why?

- **C++ compatibility**: `G_BEGIN_DECLS` expands to `extern "C" {` when
  compiled as C++, ensuring C linkage for all declared symbols — without it,
  C++ consumers get mangled names and linker errors
- **GObject convention**: every public GLib/GNOME header uses this pattern;
  omitting it breaks interoperability with C++ bindings (gtkmm, glibmm, etc.)
- **Subtle failures**: a missing or mismatched pair may compile fine in pure C
  projects but fail when the header is included from C++ code

## What is flagged

- **Public headers without `G_BEGIN_DECLS`/`G_END_DECLS`** — only flagged if
  the header contains function or type declarations and is installed (public)
- **Orphan `G_BEGIN_DECLS`** — present without a matching `G_END_DECLS`
- **Orphan `G_END_DECLS`** — present without a matching `G_BEGIN_DECLS`

## Examples

**Bad** (missing pair in a public header):
```c
#pragma once

#include <glib.h>

void my_lib_do_work (void);
```

**Good**:
```c
#pragma once

#include <glib.h>

G_BEGIN_DECLS

void my_lib_do_work (void);

G_END_DECLS
```

**Bad** (mismatched — missing closing):
```c
G_BEGIN_DECLS

void my_lib_do_work (void);

/* G_END_DECLS is missing */
```
