#include "type_since.h"

/**
 * MyDialog:
 *
 * A dialog widget.
 *
 * Since: 4.4
 */

struct _MyDialog
{
  GObject parent;
};

G_DEFINE_FINAL_TYPE (MyDialog, my_dialog, G_TYPE_OBJECT)

static void
my_dialog_class_init (MyDialogClass *klass)
{
}

static void
my_dialog_init (MyDialog *self)
{
}

struct _MyPopover
{
  GObject parent;
};

G_DEFINE_FINAL_TYPE (MyPopover, my_popover, G_TYPE_OBJECT)

static void
my_popover_class_init (MyPopoverClass *klass)
{
}

static void
my_popover_init (MyPopover *self)
{
}
