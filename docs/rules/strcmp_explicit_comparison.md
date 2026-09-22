Flags calls to `strcmp`, `strncmp`, `g_strcmp0`, `g_ascii_strcasecmp`, and
`g_ascii_strncasecmp` that are used directly as boolean conditions without an
explicit comparison to `0`.

## Why?

- **Inverted semantics**: `strcmp` returns `0` when strings are equal, which is
  falsy in C. Writing `if (strcmp (a, b))` tests for *inequality*, not equality
  — the opposite of what many developers expect
- **Readability**: `if (strcmp (a, b) == 0)` makes the intent unambiguous,
  while `if (!strcmp (a, b))` requires the reader to remember the convention
- **Common bug**: treating the return value as a boolean is one of the most
  frequent C string-handling mistakes

## Examples

**Bad** (implicit boolean):
```c
if (strcmp (a, b))
  do_something ();

if (!g_strcmp0 (a, b))
  do_equal_thing ();
```

**Good** (explicit comparison):
```c
if (strcmp (a, b) != 0)
  do_something ();

if (g_strcmp0 (a, b) == 0)
  do_equal_thing ();
```

This rule supports `--fix` to automatically insert explicit comparisons.
