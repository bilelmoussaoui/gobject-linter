#include <glib.h>

typedef struct {
  GDateTime *now;
} MyObj;

static void
date_time_ref (MyObj *self, GDateTime *dt)
{
  g_set_date_time (&self->now, dt);
}

static void
date_time_clear_pointer (MyObj *self, GDateTime *dt)
{
  g_set_date_time (&self->now, dt);
}
