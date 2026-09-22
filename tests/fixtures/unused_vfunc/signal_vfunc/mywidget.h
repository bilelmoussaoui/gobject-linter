#pragma once

#include <glib-object.h>

G_BEGIN_DECLS

#define MY_TYPE_WIDGET (my_widget_get_type ())
G_DECLARE_DERIVABLE_TYPE (MyWidget, my_widget, MY, WIDGET, GObject)

struct _MyWidgetClass {
  GObjectClass parent_class;

  void (*activate) (MyWidget *self);
};

G_END_DECLS
