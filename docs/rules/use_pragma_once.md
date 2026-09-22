Flags header files that use traditional `#ifndef`/`#define` include guards and
suggests `#pragma once` instead.

## Why?

- **Less boilerplate**: `#pragma once` is a single line instead of three
  (`#ifndef`, `#define`, `#endif`)
- **No naming collisions**: traditional guards rely on a unique macro name —
  copy-paste errors or name clashes silently suppress the header's contents
- **Compiler support**: `#pragma once` is supported by all major C compilers
  (GCC, Clang, MSVC) and is widely used in GLib/GNOME projects

## Examples

**Bad** (traditional include guard):
```c
#ifndef MY_PROJECT_FOO_H
#define MY_PROJECT_FOO_H

void foo_do_something (void);

#endif /* MY_PROJECT_FOO_H */
```

**Good** (`#pragma once`):
```c
#pragma once

void foo_do_something (void);
```

This rule supports `--fix` to automatically replace the `#ifndef`/`#define` pair
with `#pragma once` and remove the closing `#endif`.
