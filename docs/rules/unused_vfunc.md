Detects virtual methods that are assigned in `class_init` but never called
through the class vtable anywhere in the codebase.

## Why?

- **Dead code**: a vfunc that is set but never invoked is unreachable — no
  caller dispatches through it, so it cannot run
- **Design signal**: an unused vtable slot often means the virtual API was
  planned but never wired up, or was replaced by a signal and the assignment
  was left behind
- **Subclass confusion**: contributors may implement the vfunc in a subclass
  expecting it to be called, when nothing actually dispatches through that slot

## What is NOT flagged

- **GObject built-in overrides**: `dispose`, `finalize`, `constructed`,
  `get_property`, `set_property`, `notify`, `dispatch_properties_changed`,
  `constructor` — these are called by the GObject framework itself
- **Signal-backed vfuncs**: fields referenced in a `g_signal_new` class offset
  are dispatched by the signal machinery, so they are excluded

## Examples

**Flagged** (assigned but never dispatched):
```c
struct _MyObjClass {
  GObjectClass parent_class;
  void (*do_something) (MyObj *self);
};

static void
my_obj_class_init (MyObjClass *klass)
{
  klass->do_something = my_obj_do_something;
}

// No code anywhere calls klass->do_something(...)
```

**Not flagged** (vtable call exists):
```c
void
my_obj_trigger (MyObj *self)
{
  MyObjClass *klass = MY_OBJ_GET_CLASS (self);
  klass->do_something (self);
}
```
