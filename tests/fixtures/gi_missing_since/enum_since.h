#pragma once

#include <glib-object.h>

G_BEGIN_DECLS

/**
 * MyAlign:
 * @MY_ALIGN_FILL: Fill the space
 * @MY_ALIGN_START: Align to start
 * @MY_ALIGN_END: Align to end. Since: 4.12
 * @MY_ALIGN_CENTER: Align to center
 *
 * Alignment values.
 */
typedef enum {
  MY_ALIGN_FILL,
  MY_ALIGN_START,
  MY_ALIGN_END,
  MY_ALIGN_CENTER,
} MyAlign;

/**
 * MyPos:
 * @MY_POS_LEFT: Left position
 * @MY_POS_RIGHT: Right position
 *
 * Position values.
 *
 * Since: 4.2
 */
typedef enum {
  MY_POS_LEFT,
  MY_POS_RIGHT,
} MyPos;

/**
 * MyDir:
 * @MY_DIR_UP: Up
 *
 * Direction values.
 */
typedef enum {
  MY_DIR_UP,
  MY_DIR_DOWN,
} MyDir;

/**
 * MY_DIR_DOWN:
 *
 * Go downwards.
 *
 * Since: 4.8
 */

G_END_DECLS
