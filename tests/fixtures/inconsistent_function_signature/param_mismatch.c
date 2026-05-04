#include "param_mismatch.h"

/* gboolean -> gint: GLib alias mismatch in parameter */
void
quux_set_value (GObject *obj, gint enabled, gint count)
{
}

/* declared with one param, defined with none */
void
quux_reset (void)
{
}
