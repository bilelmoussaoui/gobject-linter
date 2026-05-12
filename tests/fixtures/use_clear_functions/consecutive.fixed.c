#include <glib.h>

typedef struct {
  gchar *display_seat_id;
} MyObj;

void test_function(MyObj *self) {
    g_clear_pointer (&self->display_seat_id, g_free);
}
