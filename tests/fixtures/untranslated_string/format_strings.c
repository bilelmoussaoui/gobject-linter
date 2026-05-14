#include <gtk/gtk.h>

#include <libintl.h>

#define _(s) (s)

void test_format_strings(void)
{
    // Should NOT be flagged - %s used to make next string not use printf
    gtk_message_dialog_new (NULL, 0, GTK_MESSAGE_QUESTION, GTK_BUTTONS_OK,
                            "%s", _("Some string"));

    // Should be flagged - 1st string isn't just escape
    gtk_message_dialog_new (NULL, 0, GTK_MESSAGE_QUESTION, GTK_BUTTONS_OK,
                            "%d %d", 1234, 5678);

    // Should NOT be flagged - 1st string is translated
    gtk_message_dialog_new (NULL, 0, GTK_MESSAGE_QUESTION, GTK_BUTTONS_OK,
                            _("%X"), 49374);
}

