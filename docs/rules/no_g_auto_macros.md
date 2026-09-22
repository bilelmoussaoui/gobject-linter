Flags usage of `g_autoptr`, `g_autofree`, `g_auto`, `g_autofd`, and related
macros that rely on the GCC/Clang `__attribute__((cleanup))` extension.

## Why?

- **MSVC compatibility**: MSVC does not support `__attribute__((cleanup))`, so
  code using `g_auto*` macros cannot be compiled with the Microsoft toolchain
- **Portability**: projects that target Windows with MSVC (not just MinGW)
  must avoid these macros in shared code paths
- **Explicit cleanup**: manual resource management with `g_free` /
  `g_object_unref` / `g_clear_pointer` works on all compilers

## When this rule is active

This rule is automatically controlled by the `msvc_compatible` configuration
flag — it cannot be enabled or disabled independently:

- `msvc_compatible = true` → rule is enforced at **error** level, and all rules
  that suggest `g_auto*` macros (e.g. `use_auto_cleanup`) are disabled
- `msvc_compatible = false` (default) → rule is **ignored**

```toml
# Enable MSVC compatibility mode (activates this rule)
msvc_compatible = true
```

Most GLib/GNOME projects target GCC and Clang exclusively and benefit from
`g_auto*` macros — in that case, leave `msvc_compatible` unset and use the
`use_auto_cleanup` rule instead.

## Examples

**Flagged**:
```c
void
my_function (void)
{
  g_autoptr(GObject) obj = g_object_new (G_TYPE_OBJECT, NULL);
  g_autofree gchar *str = g_strdup ("hello");
  g_auto(GStrv) argv = g_strsplit ("a:b", ":", -1);
}
```

**Good** (manual cleanup):
```c
void
my_function (void)
{
  GObject *obj = g_object_new (G_TYPE_OBJECT, NULL);
  gchar *str = g_strdup ("hello");
  GStrv argv = g_strsplit ("a:b", ":", -1);

  /* ... */

  g_strfreev (argv);
  g_free (str);
  g_object_unref (obj);
}
```
