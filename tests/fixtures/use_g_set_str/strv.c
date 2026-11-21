#include <glib.h>

typedef struct {
  char **args;
} MyObj;

static void
strv_copy (MyObj *self, char **args)
{
  g_strfreev (self->args);
  self->args = g_strdupv (args);
}

static void
strv_split (MyObj *self, const char *str)
{
  g_strfreev (self->args);
  self->args = g_strsplit (str, ",", 0);
}

static void
strv_steal (MyObj *self, char **other)
{
  g_clear_pointer (&self->args, g_strfreev);
  self->args = g_steal_pointer (&other);
}
