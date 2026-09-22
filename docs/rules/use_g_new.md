Detects calls to `g_malloc(sizeof(Type))` and `g_malloc0(sizeof(Type))` and
suggests replacing them with `g_new(Type, 1)` and `g_new0(Type, 1)`.

## Why?

- **Type safety**: `g_new` and `g_new0` encode the type in the call, so the
  compiler can catch mismatches between the allocation size and the pointer type
- **Readability**: `g_new(MyStruct, 1)` is clearer than
  `g_malloc(sizeof(MyStruct))` — the intent (allocate one `MyStruct`) is
  immediately visible
- **Consistency**: using `g_new`/`g_new0` throughout the codebase establishes
  a uniform allocation style

## Examples

**Bad** (raw sizeof):
```c
MyStruct *s = g_malloc (sizeof (MyStruct));
MyStruct *s = g_malloc0 (sizeof (MyStruct));
```

**Good** (type-safe):
```c
MyStruct *s = g_new (MyStruct, 1);
MyStruct *s = g_new0 (MyStruct, 1);
```

This rule supports `--fix` to automatically replace `g_malloc`/`g_malloc0` with `g_new`/`g_new0`.
