#include <glib.h>
#include <gio/gio.h>

typedef enum {
  LIST_FLAGS_NONE     = 0,
  LIST_FLAGS_USER     = 1 << 0,
  LIST_FLAGS_SYSTEM   = 1 << 1,
  LIST_FLAGS_ENABLED  = 1 << 2,
  LIST_FLAGS_DISABLED = 1 << 3,
} G_GNUC_FLAG_ENUM ListFilterFlags;
