Ensures that variables declared with `g_autoptr`, `g_autofree`, `g_autofd`,
or `g_auto` are properly initialized at the point of declaration.

## Why?

- **Double-free / garbage cleanup**: when an auto-cleanup variable goes out of
  scope, GLib calls its cleanup function unconditionally. If the variable was
  never initialized, the cleanup runs on a garbage pointer — causing a
  double-free, crash, or silent corruption
- **File descriptor leaks**: `g_autofd` variables must be initialized to `-1`
  (the "no descriptor" sentinel). An uninitialized value like `0` is a valid fd
  (`stdin`), and `close(0)` has nasty side effects
- **Struct macros**: `g_auto(GQueue)` requires `G_QUEUE_INIT`, `g_auto(GValue)`
  requires `G_VALUE_INIT`, etc. These macros zero-initialize the struct so the
  cleanup function can distinguish an empty value from a live one

## Expected initializers

| Declaration            | Required initializer |
|------------------------|----------------------|
| `g_autoptr(T)`         | `NULL`               |
| `g_autofree T`         | `NULL`               |
| `g_autofd int`         | `-1`                 |
| `g_auto(GQueue)`       | `G_QUEUE_INIT`       |
| `g_auto(GValue)`       | `G_VALUE_INIT`       |
| `g_auto(GOnce)`        | `G_ONCE_INIT`        |
| `g_auto(GPathBuf)`     | `G_PATH_BUF_INIT`    |
| `g_auto(GUnixPipe)`    | `G_UNIX_PIPE_INIT`   |

## Examples

**Bad** (uninitialized auto-cleanup variable):
```c
void
my_func (void)
{
  g_autoptr(GError) error;
  g_autofd int fd;

  /* if the function returns early, cleanup runs on garbage */
}
```

**Good** (properly initialized):
```c
void
my_func (void)
{
  g_autoptr(GError) error = NULL;
  g_autofd int fd = -1;
  g_auto(GQueue) queue = G_QUEUE_INIT;
}
```

## Notes

The rule is smart enough to skip variables that are immediately assigned on the
next statement (e.g., `g_autoptr(GFile) file; file = g_file_new(...);`).

This rule supports `--fix` to automatically insert the correct initializer.
