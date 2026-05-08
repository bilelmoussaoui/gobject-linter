This rule detects manual cleanup patterns that can be replaced with `g_autoptr` for automatic resource management.

## Why?

- **Safety**: Prevents memory leaks from early returns or error paths
- **Cleaner code**: No manual cleanup needed
- **Compiler support**: Works with GCC cleanup attribute

## Examples

**Bad** (manual cleanup):
```c
void
my_function (void)
{
  GObject *obj = g_object_new (MY_TYPE_OBJECT, NULL);
  
  // ... use obj ...
  
  g_object_unref (obj);
}
```

**Good** (automatic cleanup):
```c
void
my_function (void)
{
  g_autoptr(GObject) obj = g_object_new (MY_TYPE_OBJECT, NULL);
  
  // ... use obj ...
  // Automatic cleanup when function returns!
}
```
