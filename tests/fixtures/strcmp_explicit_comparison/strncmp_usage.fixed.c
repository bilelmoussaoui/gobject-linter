#include <glib.h>

static void
test_strncmp (const char *a, const char *b)
{
  /* Wrong: bare boolean check */
  if (strncmp (a, b, 3) != 0)
    g_print ("different\n");

  /* Wrong: negated */
  if (strncmp (a, b, 3) == 0)
    g_print ("equal\n");

  /* Wrong: bare g_ascii_strcasecmp */
  if (g_ascii_strcasecmp (a, b) != 0)
    g_print ("different\n");

  /* Wrong: negated g_ascii_strcasecmp */
  if (g_ascii_strcasecmp (a, b) == 0)
    g_print ("equal\n");

  /* Wrong: bare g_ascii_strncasecmp */
  if (g_ascii_strncasecmp (a, b, 5) != 0)
    g_print ("different\n");

  /* Wrong: negated g_ascii_strncasecmp */
  if (g_ascii_strncasecmp (a, b, 5) == 0)
    g_print ("equal\n");

  /* Correct: explicit comparison */
  if (strncmp (a, b, 3) == 0)
    g_print ("equal\n");

  if (g_ascii_strcasecmp (a, b) != 0)
    g_print ("different\n");

  if (g_ascii_strncasecmp (a, b, 5) == 0)
    g_print ("equal\n");
}
