Suggests using typed `g_variant_new_*()` constructors instead of
`g_variant_new()` with single-character format strings.

## Why?

- **Type safety**: `g_variant_new_string(value)` is checked at compile time,
  while `g_variant_new("s", value)` relies on the format string being correct —
  a typo like `"i"` instead of `"s"` compiles silently but produces wrong data
- **Readability**: the typed constructor name immediately tells the reader what
  type of variant is being created
- **No format parsing**: typed constructors skip the format string parser,
  making the call marginally more efficient

## Supported type mappings

| Format | Typed constructor          |
|--------|----------------------------|
| `"s"`  | `g_variant_new_string`     |
| `"b"`  | `g_variant_new_boolean`    |
| `"y"`  | `g_variant_new_byte`       |
| `"n"`  | `g_variant_new_int16`      |
| `"q"`  | `g_variant_new_uint16`     |
| `"i"`  | `g_variant_new_int32`      |
| `"u"`  | `g_variant_new_uint32`     |
| `"x"`  | `g_variant_new_int64`      |
| `"t"`  | `g_variant_new_uint64`     |
| `"h"`  | `g_variant_new_handle`     |
| `"d"`  | `g_variant_new_double`     |
| `"o"`  | `g_variant_new_object_path`|
| `"g"`  | `g_variant_new_signature`  |
| `"v"`  | `g_variant_new_variant`    |

## Examples

**Bad** (format string):
```c
GVariant *v = g_variant_new ("s", name);
GVariant *flag = g_variant_new ("b", TRUE);
```

**Good** (typed constructors):
```c
GVariant *v = g_variant_new_string (name);
GVariant *flag = g_variant_new_boolean (TRUE);
```

## Notes

Requires GLib >= 2.24. Only single-character format strings are
matched — complex format strings like `"(si)"` are left unchanged.

This rule supports `--fix` to automatically replace `g_variant_new()` with the typed constructor.
