#pragma once

#include <glib-object.h>

#define MY_DEPRECATED_IN_4_0
#define MY_DEPRECATED_IN_4_0_FOR(f)

G_BEGIN_DECLS

/**
 * my_deprecated_func:
 *
 * Does stuff.
 */
MY_DEPRECATED_IN_4_0
void my_deprecated_func (void);

/**
 * my_deprecated_with_doc:
 *
 * Does more stuff.
 *
 * Deprecated: 4.0
 */
MY_DEPRECATED_IN_4_0
void my_deprecated_with_doc (void);

G_END_DECLS
