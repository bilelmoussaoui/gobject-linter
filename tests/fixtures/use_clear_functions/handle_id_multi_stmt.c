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
    g_source_remove (vs->ioc_tag);
    vs->ioc_tag = 0;
    vs->disconnecting = FALSE;
  }
}
