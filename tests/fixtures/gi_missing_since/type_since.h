#pragma once

#include <glib-object.h>

#define MY_AVAILABLE_IN_4_4
#define MY_AVAILABLE_IN_4_6

/* Type with AVAILABLE_IN and matching Since: in .c file */
#define MY_TYPE_DIALOG (my_dialog_get_type ())

MY_AVAILABLE_IN_4_4
G_DECLARE_FINAL_TYPE (MyDialog, my_dialog, MY, DIALOG, GObject)

/* Type with AVAILABLE_IN but no Since: anywhere */
#define MY_TYPE_POPOVER (my_popover_get_type ())

MY_AVAILABLE_IN_4_6
G_DECLARE_FINAL_TYPE (MyPopover, my_popover, MY, POPOVER, GObject)
