#include <glib.h>

typedef struct {
  char **args;
} MyObj;

static void
strv_copy (MyObj *self, char **args)
{
  g_set_strv (&self->args, args);
}

static void
strv_split (MyObj *self, const char *str)
{
  g_set_strv_take (&self->args, g_strsplit (str, ",", 0));
}

static void
strv_steal (MyObj *self, char **other)
{
  g_set_strv_take (&self->args, other);
}
