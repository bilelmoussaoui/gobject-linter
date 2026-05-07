#include "functions.h"

/**
 * MyWidget:
 *
 * A test widget.
 *
 * Since: 4.0
 */

struct _MyWidget
{
  GObject parent;
};

G_DEFINE_FINAL_TYPE (MyWidget, my_widget, G_TYPE_OBJECT)

static void
my_widget_class_init (MyWidgetClass *klass)
{
}

static void
my_widget_init (MyWidget *self)
{
}

void
my_widget_show (MyWidget *self)
{
}

/**
 * my_widget_set_icon:
 * @self: a widget
 * @icon: the icon name
 *
 * Sets the icon.
 *
 * Since: 4.8
 */
void
my_widget_set_icon (MyWidget *self, const char *icon)
{
}

int
my_widget_get_baseline (MyWidget *self)
{
  return 0;
}

/**
 * my_widget_get_color:
 * @self: a widget
 *
 * Gets the color.
 *
 * Since: 4.6
 */
void
my_widget_get_color (MyWidget *self)
{
}
