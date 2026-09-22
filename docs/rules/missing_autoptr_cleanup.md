Detects types that are missing `g_autoptr()` support — i.e. no
`G_DEFINE_AUTOPTR_CLEANUP_FUNC` macro and no `G_DECLARE_*` declaration that
would provide one automatically.

## Why?

- **Memory safety**: without cleanup support, callers cannot use `g_autoptr()`
  and must free manually, which is error-prone on early-return paths
- **API completeness**: library consumers expect every public type to work with
  `g_autoptr()`
- **Modern GLib idiom**: `G_DECLARE_DERIVABLE_TYPE` / `G_DECLARE_FINAL_TYPE`
  generate the cleanup macro automatically; old-style `G_DEFINE_TYPE` does not

## What is flagged

- **Boxed types** (`G_DEFINE_BOXED_TYPE` / `G_DEFINE_BOXED_TYPE_WITH_CODE`)
  without a matching `G_DEFINE_AUTOPTR_CLEANUP_FUNC`
- **Pointer types** (`G_DEFINE_POINTER_TYPE`) without a matching
  `G_DEFINE_AUTOPTR_CLEANUP_FUNC`
- **Old-style GObject types** (`G_DEFINE_TYPE` and variants) that don't use
  `G_DECLARE_*` and have no explicit cleanup macro

## Examples

**Bad** (boxed type without cleanup):
```c
G_DEFINE_BOXED_TYPE (MyBoxed, my_boxed, my_boxed_copy, my_boxed_free)
// g_autoptr(MyBoxed) will not compile
```

**Good** (cleanup macro added):
```c
G_DEFINE_BOXED_TYPE (MyBoxed, my_boxed, my_boxed_copy, my_boxed_free)

G_DEFINE_AUTOPTR_CLEANUP_FUNC (MyBoxed, my_boxed_free)
```

**Good** (GObject using G_DECLARE_FINAL_TYPE — cleanup is automatic):
```c
G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)
```

This rule supports `--fix` to automatically insert `G_DEFINE_AUTOPTR_CLEANUP_FUNC`
before `G_END_DECLS` when the free function is visible in a public header.
