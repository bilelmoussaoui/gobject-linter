Suggests replacing `g_settings_get_value()` / `g_settings_set_value()` with
`g_variant_new()` / `g_variant_get_*()` combinations with the direct typed
helpers like `g_settings_get_string()`, `g_settings_set_boolean()`, etc.

## Why?

- **Type safety**: typed accessors enforce the expected GVariant type at compile
  time, catching schema mismatches earlier
- **Readability**: `g_settings_set_string(settings, "key", value)` is clearer
  than `g_settings_set_value(settings, "key", g_variant_new("s", value))`
- **Less boilerplate**: eliminates manual GVariant construction and extraction

## Supported type mappings

| GVariant format | Setter                     | Getter                     |
|-----------------|----------------------------|----------------------------|
| `"y"`           | `g_settings_set_uint`      | `g_settings_get_uint`      |
| `"n"`           | `g_settings_set_int`       | `g_settings_get_int`       |
| `"q"`           | `g_settings_set_uint`      | `g_settings_get_uint`      |
| `"i"`           | `g_settings_set_int`       | `g_settings_get_int`       |
| `"u"`           | `g_settings_set_uint`      | `g_settings_get_uint`      |
| `"x"`           | `g_settings_set_int64`     | `g_settings_get_int64`     |
| `"t"`           | `g_settings_set_uint64`    | `g_settings_get_uint64`    |
| `"s"`           | `g_settings_set_string`    | `g_settings_get_string`    |
| `"b"`           | `g_settings_set_boolean`   | `g_settings_get_boolean`   |
| `"d"`           | `g_settings_set_double`    | `g_settings_get_double`    |
| `"as"`          | `g_settings_set_strv`      | `g_settings_get_strv`      |

## Examples

**Bad** (manual GVariant wrapping):
```c
g_settings_set_value (settings, "name", g_variant_new ("s", name));
const gchar *val = g_variant_get_string (g_settings_get_value (settings, "name"), NULL);
```

**Good** (typed accessors):
```c
g_settings_set_string (settings, "name", name);
g_autofree gchar *val = g_settings_get_string (settings, "name");
```

Requires GLib >= 2.26.

This rule supports `--fix` to automatically replace `g_settings_get/set_value()` with the typed accessor.
