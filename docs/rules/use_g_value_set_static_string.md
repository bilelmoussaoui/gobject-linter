Suggests using `g_value_set_static_string()` instead of `g_value_set_string()`
when the value being set is a string literal.

## Why?

- **Performance**: `g_value_set_string()` duplicates the string with `g_strdup()`,
  while `g_value_set_static_string()` stores the pointer directly — this avoids
  an unnecessary allocation and copy for compile-time constant strings
- **Correct semantics**: string literals have static storage duration and never
  need to be freed, so `g_value_set_static_string()` accurately reflects their
  lifetime

## Examples

**Bad** (unnecessary string copy):
```c
g_value_set_string (value, "default-name");
```

**Good** (zero-copy for literals):
```c
g_value_set_static_string (value, "default-name");
```

## Notes

Only triggers when the second argument is a string literal. Calls with
variables, function return values, or other non-literal expressions are not
flagged.

This rule supports `--fix` to automatically replace `g_value_set_string()` with `g_value_set_static_string()`.
