#include <gtk/gtk.h>

#include <libintl.h>

#define _(s) (s)

void test_untranslated(void) {
    GtkWidget *label;
    GtkWidget *button;
    GtkWidget *window;

    // Should be flagged - untranslated string
    label = gtk_label_new("Hello World");

    // Should be flagged
    gtk_label_set_text(GTK_LABEL(label), "Untranslated text");

    // Should NOT be flagged - already wrapped
    label = gtk_label_new(_("Translated"));

    // Should NOT be flagged - already wrapped with gettext
    gtk_label_set_text(GTK_LABEL(label), gettext("Another translated"));

    // Should be flagged
    button = gtk_button_new_with_label("Click Me");

    // Should be flagged
    gtk_window_set_title(GTK_WINDOW(window), "Window Title");
}
