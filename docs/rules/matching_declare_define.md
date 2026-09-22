Checks that `G_DECLARE_*` and `G_DEFINE_*` macros for the same GObject type
are used consistently — for example, a type declared with `G_DECLARE_FINAL_TYPE`
must be defined with `G_DEFINE_FINAL_TYPE` (or a variant), not `G_DEFINE_TYPE`.

## Why?

- **ABI safety**: `G_DECLARE_FINAL_TYPE` tells the compiler the type's instance
  struct is fully visible and will not be subclassed. Defining it with
  `G_DEFINE_TYPE` instead of `G_DEFINE_FINAL_TYPE` omits the finality guarantees
  and can produce subtle ABI mismatches
- **Derivability contract**: `G_DECLARE_DERIVABLE_TYPE` promises subclassing is
  supported. Using `G_DEFINE_FINAL_TYPE` in the implementation contradicts
  that promise
- **Interface consistency**: `G_DECLARE_INTERFACE` must pair with
  `G_DEFINE_INTERFACE` — mixing it with a concrete type define is always wrong
- **Readability**: mismatched macros confuse maintainers about whether a type is
  meant to be final, derivable, or an interface

## Compatibility matrix

| Declaration macro          | Compatible definition macros                        |
|----------------------------|-----------------------------------------------------|
| `G_DECLARE_FINAL_TYPE`     | `G_DEFINE_FINAL_TYPE`, `G_DEFINE_FINAL_TYPE_WITH_CODE`, `G_DEFINE_FINAL_TYPE_WITH_PRIVATE` |
| `G_DECLARE_DERIVABLE_TYPE` | `G_DEFINE_TYPE`, `G_DEFINE_TYPE_WITH_CODE`, `G_DEFINE_TYPE_WITH_PRIVATE`, `G_DEFINE_ABSTRACT_TYPE`, `G_DEFINE_ABSTRACT_TYPE_WITH_CODE`, `G_DEFINE_ABSTRACT_TYPE_WITH_PRIVATE`, `G_DEFINE_TYPE_EXTENDED` |
| `G_DECLARE_INTERFACE`      | `G_DEFINE_INTERFACE`, `G_DEFINE_INTERFACE_WITH_CODE` |

## Examples

**Bad** (final declaration with non-final definition):
```c
/* my-obj.h */
G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)

/* my-obj.c */
G_DEFINE_TYPE (MyObj, my_obj, G_TYPE_OBJECT)  /* should be G_DEFINE_FINAL_TYPE */
```

**Good** (matching final pair):
```c
/* my-obj.h */
G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)

/* my-obj.c */
G_DEFINE_FINAL_TYPE (MyObj, my_obj, G_TYPE_OBJECT)
```

**Bad** (derivable declaration with final definition):
```c
/* my-base.h */
G_DECLARE_DERIVABLE_TYPE (MyBase, my_base, MY, BASE, GObject)

/* my-base.c */
G_DEFINE_FINAL_TYPE (MyBase, my_base, G_TYPE_OBJECT)  /* contradicts derivable */
```

**Good** (matching derivable pair):
```c
/* my-base.h */
G_DECLARE_DERIVABLE_TYPE (MyBase, my_base, MY, BASE, GObject)

/* my-base.c */
G_DEFINE_TYPE (MyBase, my_base, G_TYPE_OBJECT)
```

## Notes

Requires GLib >= 2.70 (when `G_DEFINE_FINAL_TYPE` was introduced).
