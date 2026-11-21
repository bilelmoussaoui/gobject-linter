Suggests using `g_strconcat()` instead of `g_strdup_printf()` when the format
string only contains `%s` placeholders.

## Why?

- **Performance**: `g_strdup_printf()` runs the full printf format parser,
  which is unnecessary overhead for plain string concatenation.
- **Readability**: `g_strconcat(a, b, NULL)` states the intent more directly
  than `g_strdup_printf("%s%s", a, b)`.

## Examples

**Bad**:
```c
char *path = g_strdup_printf ("%s/%s", dir, name);
```

**Good**:
```c
char *path = g_strconcat (dir, "/", name, NULL);
```

Only formats whose conversions are all plain `%s` (no flags, width, precision
or length modifiers) with a matching number of arguments are flagged.

Note that `g_strdup_printf("%s", NULL)` returns `"(null)"` on glibc, while
`g_strconcat(NULL, ...)` returns `NULL`.

This rule supports `--fix` to automatically rewrite the call.
