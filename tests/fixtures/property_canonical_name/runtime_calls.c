#include <glib-object.h>

void test_property_names(GObject *obj, GObjectClass *klass) {
    g_object_set(obj, "display_name", "hello", NULL);
    g_object_get(obj, "font_size", NULL, NULL);
    g_object_notify(obj, "text_color");
    g_object_set_property(obj, "line_width", NULL);
    g_object_get_property(obj, "border_radius", NULL);
    g_object_class_find_property(klass, "wrap_mode");
    g_object_class_override_property(klass, 1, "has_focus");

    // These should NOT trigger (already canonical)
    g_object_set(obj, "display-name", "hello", NULL);
    g_object_notify(obj, "visible");

    // Multiple properties in one g_object_set call
    g_object_set(obj, "first_name", "John", "last_name", "Doe", NULL);

    // #ifdef inside g_object_new should not cause false positives
    g_object_new(MY_TYPE,
                 "drive", drive,
                 "mount", mount,
#ifdef HAVE_CLOUDPROVIDERS
                 "cloud-provider-account", cloud_provider_account,
#endif
                 NULL);
}
