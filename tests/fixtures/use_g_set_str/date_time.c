#include <glib.h>

typedef struct {
  GDateTime *now;
} MyObj;

static void
date_time_ref (MyObj *self, GDateTime *dt)
{
  g_date_time_unref (self->now);
  self->now = g_date_time_ref (dt);
}

static void
date_time_clear_pointer (MyObj *self, GDateTime *dt)
{
  g_clear_pointer (&self->now, g_date_time_unref);
  self->now = g_date_time_ref (dt);
}
