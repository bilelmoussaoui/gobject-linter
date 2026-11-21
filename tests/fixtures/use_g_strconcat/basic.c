#include <glib.h>

static char *
two_strings (const char *a, const char *b)
{
  return g_strdup_printf ("%s%s", a, b);
}

static char *
with_separator (const char *dir, const char *name)
{
  return g_strdup_printf ("%s/%s", dir, name);
}

static char *
with_prefix_and_suffix (const char *name)
{
  return g_strdup_printf ("Hello %s!", name);
}

static char *
single_string (const char *a)
{
  return g_strdup_printf ("%s", a);
}

static char *
escaped_percent (const char *name)
{
  return g_strdup_printf ("100%% of %s", name);
}

static char *
newline_escape (const char *a)
{
  return g_strdup_printf ("%s\n", a);
}

static char *
string_literal_args (void)
{
  return g_strdup_printf ("%s-%s", "foo", "bar");
}
