#include <glib.h>

static char *
two_strings (const char *a, const char *b)
{
  return g_strconcat (a, b, NULL);
}

static char *
with_separator (const char *dir, const char *name)
{
  return g_strconcat (dir, "/", name, NULL);
}

static char *
with_prefix_and_suffix (const char *name)
{
  return g_strconcat ("Hello ", name, "!", NULL);
}

static char *
single_string (const char *a)
{
  return g_strconcat (a, NULL);
}

static char *
escaped_percent (const char *name)
{
  return g_strconcat ("100% of ", name, NULL);
}

static char *
newline_escape (const char *a)
{
  return g_strconcat (a, "\n", NULL);
}

static char *
string_literal_args (void)
{
  return g_strconcat ("foo", "-", "bar", NULL);
}
