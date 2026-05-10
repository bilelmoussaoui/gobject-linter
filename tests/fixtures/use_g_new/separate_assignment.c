#include <glib.h>

typedef struct {
  int len;
  int data[1];
} MyBitmask;

static MyBitmask *
my_bitmask_new (int bits)
{
  MyBitmask *mask;

  mask = g_malloc (sizeof (MyBitmask));
  mask->len = bits ? 1 : 0;
  mask->data[0] = bits;

  return mask;
}

static MyBitmask *
my_bitmask_new0 (int bits)
{
  MyBitmask *mask;

  mask = g_malloc0 (sizeof (MyBitmask));
  mask->len = bits;

  return mask;
}
