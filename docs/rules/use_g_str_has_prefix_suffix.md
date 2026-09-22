Suggests using `g_str_has_prefix()` and `g_str_has_suffix()` instead of manual
`strncmp` / `strcmp` patterns for checking string prefixes and suffixes.

## Why?

- **Readability**: `g_str_has_prefix(str, "foo")` is immediately clear, while
  `strncmp(str, "foo", strlen("foo")) == 0` requires the reader to mentally
  decode the pattern
- **Less error-prone**: the manual pattern requires keeping the string literal
  and the `strlen` argument in sync — a mismatch silently produces wrong results
- **Consistency**: GLib provides these helpers specifically for this purpose

## Patterns detected

**Prefix pattern**:
```c
strncmp (str, "prefix", strlen ("prefix")) == 0
```

**Suffix pattern**:
```c
strcmp (str + strlen (str) - strlen ("suffix"), "suffix") == 0
```

## Examples

**Bad** (manual prefix check):
```c
if (strncmp (filename, "/tmp/", strlen ("/tmp/")) == 0)
  handle_temp_file (filename);
```

**Good**:
```c
if (g_str_has_prefix (filename, "/tmp/"))
  handle_temp_file (filename);
```

**Bad** (manual suffix check):
```c
if (strcmp (filename + strlen (filename) - strlen (".txt"), ".txt") == 0)
  load_text_file (filename);
```

**Good**:
```c
if (g_str_has_suffix (filename, ".txt"))
  load_text_file (filename);
```

This rule supports `--fix` to automatically replace the manual pattern with `g_str_has_prefix()`/`g_str_has_suffix()`.
