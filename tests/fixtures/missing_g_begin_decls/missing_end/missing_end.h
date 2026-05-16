#pragma once

#include <glib-object.h>

G_BEGIN_DECLS

typedef struct _MyObj MyObj;

GType my_obj_get_type (void) G_GNUC_CONST;
