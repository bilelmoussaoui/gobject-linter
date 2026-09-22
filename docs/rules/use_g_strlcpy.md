Flags calls to `strcpy()`, `strcat()`, and `strncat()` and suggests using
GLib's bounds-checked alternatives `g_strlcpy()` and `g_strlcat()`.

## Why?

- **Buffer overflow prevention**: `strcpy()` and `strcat()` perform no bounds
  checking — if the source string is longer than the destination buffer, they
  silently corrupt adjacent memory
- **Correct size semantics**: `strncat()`'s `n` parameter is the maximum number
  of characters to *append*, not the total buffer size — this is a common source
  of off-by-one errors. `g_strlcat()` takes the total buffer size, which is
  simpler and safer
- **Truncation detection**: `g_strlcpy()` and `g_strlcat()` return the total
  length that would have been written, allowing callers to detect truncation

## Examples

**Bad** (no bounds checking):
```c
char buf[256];
strcpy (buf, user_input);
strcat (buf, suffix);
```

**Good** (bounds-checked):
```c
char buf[256];
g_strlcpy (buf, user_input, sizeof (buf));
g_strlcat (buf, suffix, sizeof (buf));
```

**Bad** (confusing size semantics):
```c
strncat (buf, extra, sizeof (buf) - strlen (buf) - 1);
```

**Good**:
```c
g_strlcat (buf, extra, sizeof (buf));
```
