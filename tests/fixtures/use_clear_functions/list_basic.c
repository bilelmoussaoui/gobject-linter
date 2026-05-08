#include <glib.h>

static void
my_func (GList *list, GSList *slist)
{
  g_list_free (list);
  list = NULL;

  g_slist_free (slist);
  slist = NULL;
}
