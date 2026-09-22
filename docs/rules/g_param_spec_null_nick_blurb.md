Ensures that `g_param_spec_*` calls pass `NULL` for the nick and blurb
parameters instead of string literals.

## Why?

- **Redundant strings**: passing `NULL` for nick and blurb causes
  `g_param_spec_get_nick()` to return the property name as a fallback (e.g.,
  property `"font-size"` gets nick `"font-size"`)
- **Memory savings**: string literal nick/blurb are still copied internally
  unless you also set `G_PARAM_STATIC_STRINGS`. Passing `NULL` avoids the
  allocation entirely
- **Less maintenance**: auto-generated nicks stay in sync with the property
  name — manually written nicks can drift

## Examples

**Bad** (redundant nick and blurb):
```c
g_param_spec_string ("name",
                      "Name",
                      "The object name",
                      NULL,
                      G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
```

**Good** (NULL nick and blurb):
```c
g_param_spec_string ("name",
                      NULL,
                      NULL,
                      NULL,
                      G_PARAM_READWRITE | G_PARAM_STATIC_NAME);
```

## Notes

When nick and blurb are set to `NULL`, `G_PARAM_STATIC_NICK` and
`G_PARAM_STATIC_BLURB` flags are no longer needed — the rule's auto-fix
removes them and keeps only `G_PARAM_STATIC_NAME`.

## Configuration

- **`static_flags`**: list of custom flag constants that already include
  `G_PARAM_STATIC_STRINGS` (e.g., `["MY_PARAM_READWRITE"]`). Properties using
  these flags are not flagged.
