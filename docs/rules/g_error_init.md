Ensures that `GError *` variables are initialized to `NULL` at the point of
declaration.

## Why?

- **Undefined behavior**: GLib functions that accept a `GError **` parameter
  check whether `*error != NULL` before setting the error. If the pointer
  contains garbage, the check may skip writing the error or trigger an assertion
  failure
- **Silent bugs**: an uninitialized `GError *` that happens to be non-NULL
  causes `g_set_error` to silently bail out, so the caller never sees the actual
  error and error handling is broken
- **Convention**: the GLib documentation mandates that `GError *` variables be
  initialized to `NULL` before use

## Examples

**Bad** (uninitialized GError pointer):
```c
void
my_func (void)
{
  GError *error;

  some_operation (&error);
  if (error != NULL)
    g_warning ("%s", error->message);
}
```

**Good** (initialized to NULL):
```c
void
my_func (void)
{
  GError *error = NULL;

  some_operation (&error);
  if (error != NULL)
    g_warning ("%s", error->message);
}
```

## Notes

The rule skips variables that are immediately assigned on the next statement.

This rule supports `--fix` to automatically add `= NULL` initialization.
