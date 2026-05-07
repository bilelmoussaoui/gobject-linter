#pragma once

#include <glib-object.h>

#define MY_TYPE_WIDGET (my_widget_get_type ())

MY_AVAILABLE_IN_4_0
G_DECLARE_FINAL_TYPE (MyWidget, my_widget, MY, WIDGET, GObject)

/* Function at same version as type — no Since: needed */
MY_AVAILABLE_IN_4_0
void my_widget_show (MyWidget *self);

/* Function at newer version — needs Since: */
MY_AVAILABLE_IN_4_8
void my_widget_set_icon (MyWidget *self, const char *icon);

/* Function with no Since: anywhere */
MY_AVAILABLE_IN_4_12
int my_widget_get_baseline (MyWidget *self);

/* Function with mismatched Since: */
MY_AVAILABLE_IN_4_10
void my_widget_get_color (MyWidget *self);
