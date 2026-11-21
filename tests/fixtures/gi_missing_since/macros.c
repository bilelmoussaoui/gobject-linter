#include "macros.h"

/**
 * MyWidget:
 *
 * A test widget.
 *
 * Since: 1.0
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

/**
 * my_lib_get_name:
 *
 * Since: 1.2
 */
void
my_lib_get_name (void)
{
}

/**
 * my_lib_get_size:
 *
 * Since: 1.4
 */
void
my_lib_get_size (void)
{
}

/**
 * my_lib_get_label:
 *
 * Since: 1.6
 */
void
my_lib_get_label (void)
{
}

/**
 * my_lib_get_width:
 *
 * Since: 1.8
 */
void
my_lib_get_width (void)
{
}
