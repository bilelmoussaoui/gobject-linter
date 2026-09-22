#include "myobj.h"

G_DEFINE_TYPE (MyObj, my_obj, G_TYPE_OBJECT)

static int my_obj_real_compute (MyObj *self, int value) { return value * 2; }

static void
my_obj_class_init (MyObjClass *klass)
{
  klass->compute = my_obj_real_compute;
}

static void
my_obj_init (MyObj *self)
{
}

int
my_obj_compute (MyObj *self, int value)
{
  MyObjClass *klass = MY_OBJ_GET_CLASS (self);
  return (*klass->compute) (self, value);
}
