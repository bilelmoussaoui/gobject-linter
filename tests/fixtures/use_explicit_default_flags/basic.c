#include <gio/gio.h>

static void
my_func (void)
{
  GApplication *app = g_application_new ("com.example.App", 0);
  GFile *file = g_file_new_for_path ("/tmp/foo");
  GFileInfo *info = g_file_query_info (file, "*", 0, NULL, NULL);
}
