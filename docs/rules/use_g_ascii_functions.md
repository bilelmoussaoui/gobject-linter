Flags calls to C standard library `ctype.h` functions (`tolower`, `toupper`,
`isdigit`, `isalpha`, `isalnum`, `isspace`, etc.) and suggests GLib's
locale-independent `g_ascii_*` replacements.

## Why?

- **Locale dependence**: C ctype functions like `tolower()` and `isalpha()`
  behave differently depending on the current locale. In a Turkish locale,
  `toupper('i')` returns `'İ'` (U+0130), not `'I'` — breaking protocol parsers,
  file format readers, and any code that expects ASCII semantics
- **Reproducibility**: `g_ascii_*` functions always use the C/POSIX locale,
  giving consistent results regardless of the user's environment
- **Safety**: the `g_ascii_*` functions handle `char` arguments correctly
  without requiring a cast to `unsigned char`

## Replacements

| C function  | GLib replacement       |
|-------------|------------------------|
| `tolower`   | `g_ascii_tolower`      |
| `toupper`   | `g_ascii_toupper`      |
| `isdigit`   | `g_ascii_isdigit`      |
| `isalpha`   | `g_ascii_isalpha`      |
| `isalnum`   | `g_ascii_isalnum`      |
| `isspace`   | `g_ascii_isspace`      |
| `isupper`   | `g_ascii_isupper`      |
| `islower`   | `g_ascii_islower`      |
| `isxdigit`  | `g_ascii_isxdigit`     |
| `ispunct`   | `g_ascii_ispunct`      |
| `isprint`   | `g_ascii_isprint`      |
| `isgraph`   | `g_ascii_isgraph`      |
| `iscntrl`   | `g_ascii_iscntrl`      |

## Examples

**Bad** (locale-dependent):
```c
if (isalpha (c))
  c = tolower (c);
```

**Good** (locale-independent):
```c
if (g_ascii_isalpha (c))
  c = g_ascii_tolower (c);
```

This rule supports `--fix` to automatically rename function calls.
