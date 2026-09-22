Detects calls to `g_object_notify(obj, "property-name")` and suggests replacing
them with `g_object_notify_by_pspec(obj, props[PROP_NAME])`.

## Why?

- **Performance**: `g_object_notify()` performs a string lookup on every call to
  find the matching `GParamSpec`. `g_object_notify_by_pspec()` skips this lookup
  entirely by using the cached pspec directly
- **Type safety**: using the property enum constant `props[PROP_NAME]` is
  checked at compile time — a typo in a property name string is only caught at
  runtime
- **Refactoring**: renaming a property updates the pspec and enum in one place;
  scattered string literals are easy to miss

## Examples

**Bad** (string-based notification):
```c
g_object_notify (G_OBJECT (self), "name");
```

**Good** (pspec-based notification):
```c
g_object_notify_by_pspec (G_OBJECT (self), props[PROP_NAME]);
```

## Notes

Requires a `GParamSpec*` properties array (see `use_g_object_class_install_properties`).
Requires GLib >= 2.26.

This rule supports `--fix` to automatically replace `g_object_notify` with `g_object_notify_by_pspec`.
