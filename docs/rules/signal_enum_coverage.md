Checks that every value in the signal enum has a corresponding `g_signal_new`
call.

## Why?

- **Dead enum values**: a signal enum member without a matching `g_signal_new`
  call is dead code — the signal is never registered
- **Refactoring safety**: when removing signals, it is easy to delete the
  `g_signal_new` call and forget the enum value

## Examples

**Bad** (enum value without corresponding g_signal_new):
```c
enum {
  SIGNAL_ACTIVATED,
  SIGNAL_CLOSED,    /* no g_signal_new for this */
  N_SIGNALS,
};

/* class_init only registers SIGNAL_ACTIVATED */
```

**Good** (enum and signal definitions match):
```c
enum {
  SIGNAL_ACTIVATED,
  SIGNAL_CLOSED,
  N_SIGNALS,
};

/* class_init registers both */
```
