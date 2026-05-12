#include <glib-object.h>

typedef struct {
  GObject parent;
  guint timeout_id;
} MyObject;

static gboolean
timeout_callback (gpointer user_data)
{
  MyObject *self = user_data;
  g_print ("timeout fired\n");
  return G_SOURCE_REMOVE;
}

static void
bad_timeout_not_stored (MyObject *self)
{
  /* Should trigger - ID not stored */
  g_timeout_add (1000, timeout_callback, self);
}

static void
bad_idle_not_stored (MyObject *self)
{
  /* Should trigger - ID not stored */
  g_idle_add (timeout_callback, self);
}

static void
bad_timeout_seconds_not_stored (MyObject *self)
{
  /* Should trigger - ID not stored */
  g_timeout_add_seconds (10, timeout_callback, self);
}

static void
good_timeout_stored (MyObject *self)
{
  /* Should NOT trigger - ID is stored */
  self->timeout_id = g_timeout_add (1000, timeout_callback, self);
}

static void
ok_null_user_data (void)
{
  /* Should NOT trigger - NULL user_data, likely doesn't need cleanup */
  g_timeout_add (1000, timeout_callback, NULL);
}

static void
once_callback (gpointer user_data)
{
  g_print ("once fired\n");
}

static void
ok_timeout_once (MyObject *self)
{
  /* Should trigger - g_timeout_add_once still returns an ID */
  g_timeout_add_once (1000, once_callback, self);
}

static void
nested_bad (MyObject *self)
{
  if (TRUE)
    {
      /* Should trigger - nested but still not stored */
      g_timeout_add (500, timeout_callback, self);
    }
}
