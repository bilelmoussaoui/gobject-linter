#include <glib.h>
#include <gio/gio.h>

static void
test_function (void)
{
        g_autofree char *str = g_strdup ("hello");
        g_autofree guint8 *data = g_malloc (10);
        g_autoptr(GFile) file = g_file_new_for_path ("/tmp/test");
        g_autoptr(GError) error = NULL;
        g_autolist(GFile) files = NULL;
        g_autoslist(GFile) slist = NULL;
        g_autoqueue(GFile) queue = NULL;
        g_auto(GStrv) builder = NULL;

        g_print ("%s\n", str);
}
