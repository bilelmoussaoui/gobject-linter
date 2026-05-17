#include "myobj.h"

G_DEFINE_TYPE (MyObj, my_obj, G_TYPE_OBJECT)

static void my_obj_do_something (MyObj *self) {}

static void
my_obj_class_init (MyObjClass *klass)
{
  klass->do_something = my_obj_do_something;
}

static void
my_obj_init (MyObj *self)
{
}

void
my_obj_trigger (MyObj *self)
{
  MyObjClass *klass = MY_OBJ_GET_CLASS (self);
  klass->do_something (self);
}
