#include <glib-object.h>

void test_property_names(GObject *obj, GObjectClass *klass) {
    g_object_set(obj, "display-name", "hello", NULL);
    g_object_get(obj, "font-size", NULL, NULL);
    g_object_notify(obj, "text-color");
    g_object_set_property(obj, "line-width", NULL);
    g_object_get_property(obj, "border-radius", NULL);
    g_object_class_find_property(klass, "wrap-mode");
    g_object_class_override_property(klass, 1, "has-focus");

    // These should NOT trigger (already canonical)
    g_object_set(obj, "display-name", "hello", NULL);
    g_object_notify(obj, "visible");

    // Multiple properties in one g_object_set call
    g_object_set(obj, "first-name", "John", "last-name", "Doe", NULL);
}
