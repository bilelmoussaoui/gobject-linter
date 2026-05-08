#include <glib.h>

typedef struct {
  guint autosave_idle_handler;
  guint timeout_id;
} MyObj;

/* if (id) — entire if should be removed */

static void
clear_if_truthy (MyObj *self)
{
  g_clear_handle_id (&self->autosave_idle_handler, g_source_remove);
}

/* if (id > 0) */

static void
clear_if_gt_zero (MyObj *self)
{
  g_clear_handle_id (&self->timeout_id, g_source_remove);
}

/* if (id != 0) */

static void
clear_if_neq_zero (MyObj *self)
{
  g_clear_handle_id (&self->autosave_idle_handler, g_source_remove);
}

/* if-guard with else — cannot remove the if, only the braces */

static void
clear_if_with_else (MyObj *self)
{
  if (self->timeout_id)
    g_clear_handle_id (&self->timeout_id, g_source_remove);
  else
    {
      g_source_remove (42);
    }
}
