Enforces consistent use of either GLib type aliases (`gint`, `gchar`,
`gboolean`, …) or C standard types (`int`, `char`, `int32_t`, …) throughout
the codebase.

## Why?

- **Consistency**: mixing `gint` and `int` in the same project (or the same
  file) is distracting and makes grep-based refactoring unreliable
- **Project policy**: some projects (e.g. GNOME) prefer GLib types, while
  others prefer C standard types — the rule enforces whichever the project
  chooses

## Examples

With `style = "glib"`:

**Bad**:
```c
int
my_obj_get_width (MyObj *self)
{
  unsigned int count = 0;
  /* ... */
}
```

**Good**:
```c
gint
my_obj_get_width (MyObj *self)
{
  guint count = 0;
  /* ... */
}
```

With `style = "c"`:

**Bad**:
```c
gint
my_obj_get_width (MyObj *self)
{
  guint count = 0;
  /* ... */
}
```

**Good**:
```c
int
my_obj_get_width (MyObj *self)
{
  unsigned int count = 0;
  /* ... */
}
```

## Configuration

```toml
[rules.type_style]
options = { style = "glib" }  # or "c"
```

- **glib** (default): prefer `gint`, `guint`, `gchar`, `gboolean`, `gpointer`,
  `gsize`, etc.
- **c**: prefer `int`, `unsigned int`, `char`, `bool`/`_Bool`, `void *`,
  `size_t`, `int32_t`, etc.

This rule supports `--fix` to automatically replace type names.
