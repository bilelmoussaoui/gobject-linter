#include "mywidget.h"

G_DEFINE_TYPE (MyWidget, my_widget, G_TYPE_OBJECT)

static void my_widget_real_activate (MyWidget *self) {}

static void
my_widget_class_init (MyWidgetClass *klass)
{
  klass->activate = my_widget_real_activate;

  g_signal_new ("activate",
                G_TYPE_FROM_CLASS (klass),
                G_SIGNAL_RUN_LAST,
                G_STRUCT_OFFSET (MyWidgetClass, activate),
                NULL, NULL, NULL,
                G_TYPE_NONE, 0);
}

static void
my_widget_init (MyWidget *self)
{
}
