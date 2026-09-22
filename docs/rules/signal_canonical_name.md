Ensures GObject signal names use the canonical kebab-case format with hyphens,
not underscores.

## Why?

- **GLib convention**: GLib normalizes signal names by converting underscores to
  hyphens internally. Using underscores works but is non-canonical and
  inconsistent with documentation and `g_signal_lookup` expectations
- **Consistency with properties**: GObject property names also use kebab-case;
  using the same convention for signals avoids confusion
- **Documentation**: gtk-doc and gi-docgen index signals by their canonical
  name — using underscores can cause lookup mismatches

## Examples

**Bad** (underscores in signal name):
```c
g_signal_new ("notify_changed",
              G_TYPE_FROM_CLASS (klass),
              G_SIGNAL_RUN_LAST,
              0, NULL, NULL, NULL,
              G_TYPE_NONE, 0);
```

**Good** (canonical kebab-case):
```c
g_signal_new ("notify-changed",
              G_TYPE_FROM_CLASS (klass),
              G_SIGNAL_RUN_LAST,
              0, NULL, NULL, NULL,
              G_TYPE_NONE, 0);
```

The rule also checks signal names in `g_signal_connect`, `g_signal_emit_by_name`,
and similar functions.

This rule supports `--fix` to automatically replace underscores with hyphens.
