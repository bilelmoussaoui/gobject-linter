// Detailed signals: "signal::detail" and "signal::" MACRO concatenation
#include <glib-object.h>

#define CUSTOM_ITEM "custom-item"

void setup_signals(GObject *obj) {
    // Detail with underscored signal name, should fix signal part only
    g_signal_connect(obj, "notify::some_property", NULL, NULL);

    // Underscored signal with inline detail, should fix signal part only
    g_signal_connect(obj, "item-activated::details", NULL, NULL);

    // Concatenated macro detail, should fix signal part, leave macro alone
    g_signal_connect(obj, "item-activated::" CUSTOM_ITEM, NULL, NULL);

    // No underscores, should not trigger
    g_signal_connect(obj, "custom-item-activated::" CUSTOM_ITEM, NULL, NULL);

    // Plain detailed signal, already correct, should not trigger
    g_signal_connect(obj, "notify::visible", NULL, NULL);
}
