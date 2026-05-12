#include <glib.h>

typedef struct {
  gchar *word;
} MyObj;

static const char *outer_func (const char *s) { return s; }
static const char *inner_func (const char *s) { return s; }

/* nested function calls as strdup arg — trim_end_matches eats extra parens */

static void
set_word (MyObj *self, const char *raw)
{
  g_clear_pointer (&self->word, g_free);
  self->word = g_strdup (outer_func (inner_func (raw)));
}
