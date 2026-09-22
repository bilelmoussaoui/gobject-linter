Detects manual cleanup patterns that can be replaced with `g_autoptr` for automatic resource management.

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

### Goto-cleanup pattern

The rule also detects `goto`-based cleanup idioms where a variable is allocated,
then freed under a cleanup label. It suggests using `g_autoptr` with
`g_steal_pointer` to eliminate the goto:

**Bad** (goto cleanup):
```c
GObject *obj = g_object_new (MY_TYPE_OBJECT, NULL);

if (!do_something (obj))
  goto cleanup;

result = g_steal_pointer (&obj);

cleanup:
  g_clear_object (&obj);
  return result;
```

**Good** (auto cleanup):
```c
g_autoptr(GObject) obj = g_object_new (MY_TYPE_OBJECT, NULL);

if (!do_something (obj))
  return NULL;

return g_steal_pointer (&obj);
```

## Notes

Requires GLib >= 2.44, and is automatically disabled when
`msvc_compatible = true` (MSVC does not support `__attribute__((cleanup))`).

## Configuration

### `ignore_types` (default: `[]`)

List of glob patterns for types to skip. Use this to suppress suggestions for
types whose cleanup functions have side effects or that are managed by an
external framework:

```toml
[rules.use_auto_cleanup]
ignore_types = ["cairo_*", "Pango*", "RsvgHandle"]
```

### `allocation_proof` (default: `true`)

When `true`, the rule only suggests auto-cleanup when the variable is provably
allocated in the current function (e.g. via `g_object_new`, `g_strdup`,
`g_new0`, etc.). This avoids false positives for variables that are freed but
not locally allocated (borrowed pointers, out-parameters, etc.).

Set to `false` to flag any manually freed variable, regardless of whether the
allocation is visible:

```toml
[rules.use_auto_cleanup]
allocation_proof = false
```
