#include "old_style_gobject.h"

struct _OldObj {
  GObject parent;
};

struct _OldObjClass {
  GObjectClass parent_class;
};

G_DEFINE_TYPE (OldObj, old_obj, G_TYPE_OBJECT)

static void
old_obj_class_init (OldObjClass *klass)
{
}

static void
old_obj_init (OldObj *self)
{
}
