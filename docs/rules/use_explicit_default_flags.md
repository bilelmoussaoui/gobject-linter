Flags function calls that pass a bare `0` where a named "none" or "default"
constant exists. Common examples include `g_application_new(…, 0)` instead of
`g_application_new(…, G_APPLICATION_DEFAULT_FLAGS)`.

## Why?

- **Readability**: `G_APPLICATION_DEFAULT_FLAGS` is self-documenting, while `0`
  requires the reader to know the API convention
- **Grep-ability**: searching for the named constant finds all call sites that
  use the default; searching for `0` produces noise
- **Intent**: a named constant makes it clear that "no flags" is deliberate,
  not an oversight

## Covered APIs

| Function | Replacement constant | Since |
|----------|---------------------|-------|
| `g_application_new` | `G_APPLICATION_DEFAULT_FLAGS` | 2.74 |
| `gtk_application_new` | `G_APPLICATION_DEFAULT_FLAGS` | 2.74 |
| `adw_application_new` | `G_APPLICATION_DEFAULT_FLAGS` | 2.74 |
| `gtk_widget_class_add_binding` | `GDK_NO_MODIFIER_MASK` | — |
| `gtk_widget_class_add_binding_signal` | `GDK_NO_MODIFIER_MASK` | — |
| `gtk_widget_class_add_binding_action` | `GDK_NO_MODIFIER_MASK` | — |
| `gtk_shortcut_new` | `GDK_NO_MODIFIER_MASK` | — |
| `g_dbus_connection_new` | `G_DBUS_CONNECTION_FLAGS_NONE` | 2.26 |
| `g_dbus_connection_new_sync` | `G_DBUS_CONNECTION_FLAGS_NONE` | 2.26 |
| `g_dbus_connection_new_for_address` | `G_DBUS_CONNECTION_FLAGS_NONE` | 2.26 |
| `g_dbus_connection_new_for_address_sync` | `G_DBUS_CONNECTION_FLAGS_NONE` | 2.26 |
| `g_dbus_proxy_new` | `G_DBUS_PROXY_FLAGS_NONE` | 2.26 |
| `g_dbus_proxy_new_sync` | `G_DBUS_PROXY_FLAGS_NONE` | 2.26 |
| `g_dbus_proxy_new_for_bus` | `G_DBUS_PROXY_FLAGS_NONE` | 2.26 |
| `g_dbus_proxy_new_for_bus_sync` | `G_DBUS_PROXY_FLAGS_NONE` | 2.26 |
| `g_file_query_info` | `G_FILE_QUERY_INFO_NONE` | — |
| `g_file_query_info_async` | `G_FILE_QUERY_INFO_NONE` | — |
| `g_file_enumerate_children` | `G_FILE_QUERY_INFO_NONE` | — |
| `g_file_enumerate_children_async` | `G_FILE_QUERY_INFO_NONE` | — |
| `g_subprocess_new` | `G_SUBPROCESS_FLAGS_NONE` | 2.40 |
| `g_subprocess_launcher_new` | `G_SUBPROCESS_FLAGS_NONE` | 2.40 |
| `g_settings_new_with_backend_and_path` | `G_SETTINGS_BIND_DEFAULT` | — |
| `gtk_icon_theme_lookup_icon` | `GTK_ICON_LOOKUP_NONE` | — |
| `gtk_icon_theme_lookup_by_gicon` | `GTK_ICON_LOOKUP_NONE` | — |
| `gtk_drop_target_async_new` | `GDK_ACTION_NONE` | — |
| `gtk_drop_target_new` | `GDK_ACTION_NONE` | — |
| `g_signal_connect_data` | `G_CONNECT_DEFAULT` | 2.74 |
| `g_signal_connect_object` | `G_CONNECT_DEFAULT` | 2.74 |
| `g_signal_group_connect_data` | `G_CONNECT_DEFAULT` | 2.74 |
| `g_signal_group_connect_object` | `G_CONNECT_DEFAULT` | 2.74 |

## Examples

**Bad** (bare zero):
```c
app = g_application_new ("org.example.App", 0);
proxy = g_dbus_proxy_new_sync (conn, 0, NULL, name, path, iface, NULL, NULL);
```

**Good** (named constant):
```c
app = g_application_new ("org.example.App", G_APPLICATION_DEFAULT_FLAGS);
proxy = g_dbus_proxy_new_sync (conn, G_DBUS_PROXY_FLAGS_NONE, NULL, name, path, iface, NULL, NULL);
```

This rule supports `--fix` to automatically replace bare `0` with the named constant.
