#include <glib.h>

typedef struct {
  gchar *display_seat_id;
} MyObj;

void test_function(MyObj *self) {
    g_free (self->display_seat_id);
    self->display_seat_id = NULL;
}
