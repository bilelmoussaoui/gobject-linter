Detects `g_object_new()` calls that create an object with no properties followed
by one or more `g_object_set()` calls on the same object, and suggests setting
the properties directly in `g_object_new()`.

## Why?

- **Atomicity**: setting properties in `g_object_new()` ensures they are applied
  during construction, before signals are connected or the object is used
- **Performance**: each `g_object_set()` call triggers property notification
  signals individually, while properties passed to `g_object_new()` are set
  together during construction
- **Readability**: a single `g_object_new()` call with all properties is easier
  to scan than a create-then-configure sequence

## Examples

**Bad** (create then configure):
```c
obj = g_object_new (MY_TYPE_OBJ, NULL);
g_object_set (obj, "name", "hello", NULL);
g_object_set (obj, "value", 42, NULL);
```

**Good** (properties in constructor):
```c
obj = g_object_new (MY_TYPE_OBJ,
                    "name", "hello",
                    "value", 42,
                    NULL);
```
