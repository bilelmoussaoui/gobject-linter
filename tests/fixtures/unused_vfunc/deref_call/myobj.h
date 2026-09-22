#pragma once

#include <glib-object.h>

G_BEGIN_DECLS

#define MY_TYPE_OBJ (my_obj_get_type ())
G_DECLARE_DERIVABLE_TYPE (MyObj, my_obj, MY, OBJ, GObject)

struct _MyObjClass {
  GObjectClass parent_class;

  int (*compute) (MyObj *self, int value);
};

G_END_DECLS
