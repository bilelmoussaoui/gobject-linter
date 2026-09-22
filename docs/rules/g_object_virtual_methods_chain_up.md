Ensures that `dispose`, `finalize`, and `constructed` virtual method
implementations chain up to the parent class.

## Why?

- **Resource leaks**: `dispose` drops references to other objects and
  `finalize` frees owned memory. If you don't chain up, the parent class's
  resources are never released
- **Incomplete initialization**: `constructed` is called after all construct
  properties are set. Skipping the parent's `constructed` may leave the object
  in a broken state
- **Silent corruption**: unlike missing a function call, a missing chain-up
  produces no compiler warning — the program silently leaks or misbehaves

## Conventions

- `dispose` and `finalize`: chain up at the **end** of the function
- `constructed`: chain up at the **beginning**, before accessing construct
  properties

## Examples

**Bad** (missing chain-up in dispose):
```c
static void
my_obj_dispose (GObject *object)
{
  MyObj *self = MY_OBJ (object);
  g_clear_object (&self->child);
  /* missing: G_OBJECT_CLASS (my_obj_parent_class)->dispose (object); */
}
```

**Good** (chain-up at the end of dispose):
```c
static void
my_obj_dispose (GObject *object)
{
  MyObj *self = MY_OBJ (object);
  g_clear_object (&self->child);
  G_OBJECT_CLASS (my_obj_parent_class)->dispose (object);
}
```

**Good** (chain-up at the start of constructed):
```c
static void
my_obj_constructed (GObject *object)
{
  G_OBJECT_CLASS (my_obj_parent_class)->constructed (object);

  MyObj *self = MY_OBJ (object);
  /* ... use construct properties ... */
}
```

This rule supports `--fix` to automatically insert the missing chain-up call.
