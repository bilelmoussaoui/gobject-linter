#include <glib.h>

static void
my_func (GList *list, GSList *slist)
{
  g_clear_list (&list, NULL);

  g_clear_slist (&slist, NULL);
}
