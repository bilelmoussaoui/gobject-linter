#include <glib.h>

typedef struct {
  GObject *source;
  gulong   signal_id;
  gulong   notify_id;
} MyObj;

/* 2-statement pattern: disconnect then zero */

static void
clear_2stmt (MyObj *self)
{
  g_signal_handler_disconnect (self->source, self->signal_id);
  self->signal_id = 0;
}

/* Multiple IDs in same function */

static void
clear_2stmt_multiple (MyObj *self)
{
  g_signal_handler_disconnect (self->source, self->signal_id);
  self->signal_id = 0;
  g_signal_handler_disconnect (self->source, self->notify_id);
  self->notify_id = 0;
}

/* if-guarded: if (id) — guard is redundant, replace entire if */

static void
clear_if_truthy (MyObj *self)
{
  if (self->signal_id)
    {
      g_signal_handler_disconnect (self->source, self->signal_id);
      self->signal_id = 0;
    }
}

/* if-guarded: if (id > 0) */

static void
clear_if_gt_zero (MyObj *self)
{
  if (self->signal_id > 0)
    {
      g_signal_handler_disconnect (self->source, self->signal_id);
      self->signal_id = 0;
    }
}

/* if-guarded: if (id != 0) */

static void
clear_if_neq_zero (MyObj *self)
{
  if (self->notify_id != 0)
    {
      g_signal_handler_disconnect (self->source, self->notify_id);
      self->notify_id = 0;
    }
}

/* bare disconnect on struct member — no following zero-assign */

static void
clear_bare_member (MyObj *self)
{
  g_signal_handler_disconnect (self->source, self->signal_id);
}

/* bare disconnect on struct member, but the struct is freed — skip */

static void
clear_bare_member_freed (MyObj *self)
{
  g_signal_handler_disconnect (self->source, self->signal_id);
  g_free (self);
}

/* bare disconnect on struct member, but the source is g_clear_object'd — skip */

static void
clear_bare_member_source_cleared (MyObj *self)
{
  g_signal_handler_disconnect (self->source, self->signal_id);
  g_clear_object (&self->source);
}
