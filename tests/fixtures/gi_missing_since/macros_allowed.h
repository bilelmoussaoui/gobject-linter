#pragma once

#include <glib-object.h>

#define MY_LIB_EXPORT

/* Not flagged: MY_LIB_EXPORT is configured as an allowed export macro */
MY_LIB_EXPORT
void my_lib_get_width (void);
