#include <gio/gio.h>

enum { PROP_0, PROP_G_INTERFACE_INFO };

// Should NOT trigger: G_PARAM_STATIC_STRINGS is already present, even with cast
void test_cast_flags(GObjectClass *gobject_class) {
    g_object_class_install_property(
        gobject_class, PROP_G_INTERFACE_INFO,
        g_param_spec_boxed(
            "g-interface-info", "Interface Info",
            "A DBusInterfaceInfo representing the exported object",
            G_TYPE_DBUS_INTERFACE_INFO,
            (GParamFlags)(G_PARAM_STATIC_STRINGS | G_PARAM_WRITABLE |
                          G_PARAM_CONSTRUCT_ONLY)));
}
