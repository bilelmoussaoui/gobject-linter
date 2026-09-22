Detects the pattern of calling `g_bytes_get_data()` followed by
`g_bytes_unref()` and suggests replacing it with `g_bytes_unref_to_data()`.

## Why?

- **Avoids an extra copy**: when the `GBytes` has a single reference,
  `g_bytes_unref_to_data()` steals the underlying buffer directly instead of
  copying it first
- **Simpler code**: one call replaces two, reducing the chance of accidentally
  using the `GBytes` after unref

## Examples

**Bad** (two separate calls):
```c
data = g_bytes_get_data (bytes, &size);
g_bytes_unref (bytes);
```

**Good** (combined):
```c
data = g_bytes_unref_to_data (bytes, &size);
```

## Notes

Requires GLib >= 2.32.

This rule supports `--fix` to automatically replace the two-call pattern with `g_bytes_unref_to_data`.
