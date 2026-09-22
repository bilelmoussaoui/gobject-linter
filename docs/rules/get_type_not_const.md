Warns when `_get_type` functions are annotated with `G_GNUC_CONST` or
`G_GNUC_PURE`.

## Why?

- **Incorrect annotation**: `_get_type` functions have a side effect on first
  call — they register the type with the GObject type system. `G_GNUC_CONST`
  and `G_GNUC_PURE` promise the compiler the function has no side effects
- **Miscompilation**: with GCC 16+, the compiler may optimize away the
  type-registration call entirely based on the `const`/`pure` attribute,
  causing crashes when the type is used before it has been registered
- **Silent breakage**: the code may work for years until a compiler upgrade
  enables more aggressive optimization, making this a latent bug

## Examples

**Bad** (incorrectly annotated):
```c
G_GNUC_CONST GType my_obj_get_type (void);

GType my_widget_get_type (void) G_GNUC_PURE;
```

**Good** (no purity annotation):
```c
GType my_obj_get_type (void);

GType my_widget_get_type (void);
```

This rule supports `--fix` to automatically remove the attribute from the declaration.
