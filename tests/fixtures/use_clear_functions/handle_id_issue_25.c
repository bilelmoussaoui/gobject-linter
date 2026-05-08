#include <glib.h>

typedef struct {
  guint autosave_idle_handler;
  guint timeout_id;
} MyObj;

/* if (id) — entire if should be removed */

static void
clear_if_truthy (MyObj *self)
{
  if (self->autosave_idle_handler)
    {
      g_source_remove (self->autosave_idle_handler);
      self->autosave_idle_handler = 0;
    }
}

/* if (id > 0) */

static void
clear_if_gt_zero (MyObj *self)
{
  if (self->timeout_id > 0)
    {
      g_source_remove (self->timeout_id);
      self->timeout_id = 0;
    }
}

/* if (id != 0) */

static void
clear_if_neq_zero (MyObj *self)
{
  if (self->autosave_idle_handler != 0)
    {
      g_source_remove (self->autosave_idle_handler);
      self->autosave_idle_handler = 0;
    }
}

/* if-guard with else — cannot remove the if, only the braces */

static void
clear_if_with_else (MyObj *self)
{
  if (self->timeout_id)
    {
      g_source_remove (self->timeout_id);
      self->timeout_id = 0;
    }
  else
    {
      g_source_remove (42);
    }
}
