#pragma once

#include <glib-object.h>

/* --- Variadic: should warn --- */

MY_EXPORT
void my_widget_set_properties (MyWidget *self, const char *first, ...);

MY_EXPORT
MyWidget *my_widget_new_with_args (const char *first, ...);

/* --- Variadic: skipped, should NOT warn --- */

/**
 * my_widget_printf: (skip)
 * @self: a widget
 * @fmt: format string
 * @...: arguments
 */
MY_EXPORT
void my_widget_printf (MyWidget *self, const char *fmt, ...);

/* --- Out params: 3 out params, should warn --- */

MY_EXPORT
void my_widget_get_geometry (MyWidget *self,
                             int     **out_x,
                             int     **out_y,
                             int     **out_w);

/* --- Out params: 3 out params + GError, should still warn (3 real out params) --- */

MY_EXPORT
gboolean my_widget_parse (MyWidget    *self,
                          char       **out_name,
                          char       **out_value,
                          int        **out_len,
                          GError     **error);

/* --- Out params: 2 out params, should NOT warn --- */

MY_EXPORT
void my_widget_get_size (MyWidget *self,
                         int     **out_w,
                         int     **out_h);

/* --- Out params: 2 out params + GError, should NOT warn --- */

MY_EXPORT
gboolean my_widget_load (MyWidget    *self,
                         char       **out_data,
                         gsize      **out_len,
                         GError     **error);

/* --- Container types: should warn --- */

MY_EXPORT
GList *my_widget_get_children (MyWidget *self);

MY_EXPORT
GSList *my_widget_get_items (MyWidget *self);

MY_EXPORT
GHashTable *my_widget_get_attributes (MyWidget *self);

MY_EXPORT
GPtrArray *my_widget_get_actions (MyWidget *self);

MY_EXPORT
void my_widget_set_items (MyWidget *self, GArray *items);

MY_EXPORT
void my_widget_set_data (MyWidget *self, GByteArray *data);

/* --- Container types in params: should warn --- */

MY_EXPORT
void my_widget_add_children (MyWidget *self, GList *children);

/* --- No issues: should NOT warn --- */

MY_EXPORT
void my_widget_show (MyWidget *self);

MY_EXPORT
const char *my_widget_get_name (MyWidget *self);

/* --- No export macro: should NOT warn --- */

GList *my_internal_get_list (void);

/* --- _get_type: should NOT warn even if variadic (convention skip) --- */

MY_EXPORT
GType my_widget_get_type (void);
