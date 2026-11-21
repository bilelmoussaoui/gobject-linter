#pragma once

#include <glib-object.h>

#define MY_LIB_AVAILABLE_IN_ALL
#define MY_LIB_DEPRECATED
#define MY_LIB_DEPRECATED_FOR(f)
#define MY_LIB_EXPORT

/* Type whose macro carries no version — not flagged */
#define MY_TYPE_WIDGET (my_widget_get_type ())

MY_LIB_AVAILABLE_IN_ALL
G_DECLARE_FINAL_TYPE (MyWidget, my_widget, MY, WIDGET, GObject)

/* Not flagged: *_AVAILABLE_IN_ALL needs no versioned macro */
MY_LIB_AVAILABLE_IN_ALL
void my_lib_get_name (void);

/* Not flagged: *_DEPRECATED needs no versioned macro */
MY_LIB_DEPRECATED
void my_lib_get_size (void);

/* Not flagged: *_DEPRECATED_FOR needs no versioned macro */
MY_LIB_DEPRECATED_FOR (my_lib_get_size)
void my_lib_get_label (void);

/* Flagged: MY_LIB_EXPORT is not an allowed export macro by default */
MY_LIB_EXPORT
void my_lib_get_width (void);
