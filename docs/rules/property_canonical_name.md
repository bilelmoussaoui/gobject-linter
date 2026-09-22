Flags GObject property names that use underscores instead of the canonical
dash-separated form.

## Why?

- **Correctness**: when `G_PARAM_STATIC_NAME` or `G_PARAM_STATIC_STRINGS` is
  set, GLib asserts that the name is canonical — a non-canonical name triggers
  `g_param_spec_internal: assertion '!(flags & G_PARAM_STATIC_NAME) || is_canonical (name)' failed`
  at runtime
- **Consistency**: the GObject convention is dash-separated names
  (`"display-name"`, not `"display_name"`); mixing styles makes property lookup
  and binding code harder to follow
- **Interoperability**: bindings, GtkBuilder XML, and `g_object_bind_property`
  all expect the canonical form

## What is checked

Property names are checked in two contexts:

1. **`g_param_spec_*` definitions** — the first argument (the property name)
2. **Runtime API calls** — string arguments that represent property names in:
   - `g_object_notify`, `g_object_set_property`, `g_object_get_property`,
     `g_object_class_find_property`, `g_object_class_override_property`
   - `g_object_set`, `g_object_get`, `g_object_new` (varargs property/value
     pairs)
   - Various GTK helpers like `gtk_cell_layout_add_attribute`,
     `gtk_text_buffer_create_tag`, etc.

## Examples

**Bad** (underscores in property definition):
```c
g_param_spec_string ("display_name", NULL, NULL, NULL,
                     G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
```

**Good** (canonical dashes):
```c
g_param_spec_string ("display-name", NULL, NULL, NULL,
                     G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
```

**Bad** (underscores in runtime calls):
```c
g_object_set (obj, "display_name", "hello", NULL);
g_object_notify (obj, "text_color");
```

**Good**:
```c
g_object_set (obj, "display-name", "hello", NULL);
g_object_notify (obj, "text-color");
```

This rule supports `--fix` to automatically replace underscores with hyphens.
