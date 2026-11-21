#include <glib.h>

typedef struct {
  char *name;
  char *path;
} MyObj;

static void
take_printf (MyObj *self, const char *name)
{
  g_free (self->name);
  self->name = g_strdup_printf ("%s!", name);
}

static void
take_concat (MyObj *self, const char *dir, const char *file)
{
  g_free (self->path);
  self->path = g_strconcat (dir, "/", file, NULL);
}

static void
take_steal (MyObj *self, char *name)
{
  g_clear_pointer (&self->name, g_free);
  self->name = g_steal_pointer (&name);
}

static void
take_strndup (MyObj *self, const char *name)
{
  g_free (self->name);
  self->name = g_strndup (name, 10);
}

/* Not flagged: ownership of the new value is unknown */

static void
unknown_ownership (MyObj *self, const char *name)
{
  g_free (self->name);
  self->name = name;
}

static void
null_assignment (MyObj *self)
{
  g_free (self->name);
  self->name = NULL;
}
