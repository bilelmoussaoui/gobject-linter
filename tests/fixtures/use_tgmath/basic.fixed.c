#include <glib.h>
#include <tgmath.h>

static float
my_sinf (float x)
{
  return sin (x);
}

static double
my_powl (double x, double y)
{
  return pow (x, y);
}

static float
my_fabsf (float x)
{
  return fabs (x);
}

/* Not flagged: already the type-generic macro */

static double
my_sqrt (double x)
{
  return sqrt (x);
}

/* Not flagged: log2 is not type-generic */

static double
my_log2f (double x)
{
  return log2f (x);
}

/* Not flagged: round is not in the C99 <tgmath.h> set */

static float
my_roundf (float x)
{
  return roundf (x);
}
