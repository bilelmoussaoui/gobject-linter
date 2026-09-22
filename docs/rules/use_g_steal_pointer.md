Suggests using `g_steal_pointer()` instead of manual copy-then-NULL patterns
for transferring pointer ownership.

## Why?

- **Atomic intent**: `g_steal_pointer()` expresses ownership transfer as a
  single operation — the reader immediately understands what is happening
- **Fewer lines**: replaces two or three statements with one expression
- **Safer**: reduces the risk of forgetting to NULL the source pointer after
  copying, which could lead to double-free bugs

## Patterns detected

The rule recognizes several manual steal patterns:

1. **Assign then NULL**:
   ```c
   dest = src;
   src = NULL;
   ```

2. **Declare, NULL, return**:
   ```c
   char *tmp = self->ptr;
   self->ptr = NULL;
   return tmp;
   ```

3. **If/else copy-and-NULL**:
   ```c
   if (ptr) { dest = ptr; ptr = NULL; } else { dest = NULL; }
   ```

## Examples

**Bad** (manual ownership transfer):
```c
result = self->cached_value;
self->cached_value = NULL;
return result;
```

**Good**:
```c
return g_steal_pointer (&self->cached_value);
```

**Bad** (conditional steal):
```c
if (self->pending)
  {
    result = self->pending;
    self->pending = NULL;
  }
else
  {
    result = NULL;
  }
```

**Good**:
```c
result = g_steal_pointer (&self->pending);
```

This rule supports `--fix` to automatically replace the manual pattern with `g_steal_pointer()`.
