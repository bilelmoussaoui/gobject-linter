#include <glib.h>

/* Not flagged: non-%s conversions, modifiers, no placeholder, non-literal
 * format, argument count mismatch */

static char *
numeric_format (const char *a, int n)
{
  return g_strdup_printf ("%s has %d items", a, n);
}

static char *
only_numeric_format (int n)
{
  return g_strdup_printf ("%d items", n);
}

static char *
width_format (const char *a)
{
  return g_strdup_printf ("%10s", a);
}

static char *
precision_format (const char *a)
{
  return g_strdup_printf ("%.3s", a);
}

static char *
length_modifier_format (const char *a)
{
  return g_strdup_printf ("%ls", a);
}

static char *
no_placeholder (void)
{
  return g_strdup_printf ("constant");
}

static char *
non_literal_format (const char *fmt, const char *a)
{
  return g_strdup_printf (fmt, a);
}

static char *
extra_args (const char *a)
{
  return g_strdup_printf ("%s", a, a);
}

static char *
already_good (const char *a, const char *b)
{
  return g_strconcat (a, b, NULL);
}
