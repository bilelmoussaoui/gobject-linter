#include <glib.h>

/* Not flagged: <math.h> is not included, so sinf() cannot be replaced by
 * the type-generic macro */

static float
my_sinf (float x)
{
  return sinf (x);
}
