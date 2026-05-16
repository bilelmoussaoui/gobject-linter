// Should trigger: g_signal_connect and variants with underscores
#include <glib-object.h>

static void on_value_changed (void) { }
static void on_item_selected (void) { }
static void on_state_updated (void) { }

void setup_signals(GObject *obj) {
    g_signal_connect(obj, "value_changed", G_CALLBACK(on_value_changed), NULL);
    g_signal_connect_after(obj, "item_selected", G_CALLBACK(on_item_selected), NULL);
    g_signal_connect_swapped(obj, "state_updated", G_CALLBACK(on_state_updated), NULL);
    g_signal_emit_by_name(obj, "notify_user");
    g_signal_stop_emission_by_name(obj, "insert_text");
}
