#include <glib.h>

typedef struct {
    gint count;
    guint flags;
    gchar *name;
    gfloat ratio;
} MyData;

gint
my_sum(gint a, gint b)
{
    gint result = a + b;

    for (gint i = 0; i < result; i++) {
        result += i;
    }

    return result;
}
