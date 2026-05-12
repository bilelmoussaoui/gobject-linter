#include <glib.h>

typedef struct {
  guint ioc_tag;
  gboolean disconnecting;
} VncState;

/* if-block with 3 statements: pair + extra work */

static void
vnc_disconnect (VncState *vs)
{
  if (vs->disconnecting) {
    g_clear_handle_id (&vs->ioc_tag, g_source_remove);
    vs->disconnecting = FALSE;
  }
}

/* g_clear_handle_id guarded by unrelated condition — no violation */

static void
vnc_disconnect2 (VncState *vs)
{
  if (vs->disconnecting) {
    g_clear_handle_id (&vs->ioc_tag, g_source_remove);
  }
}
