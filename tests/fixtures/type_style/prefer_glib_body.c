#include <glib.h>

typedef struct {
    int count;
    unsigned int flags;
    char *name;
    float ratio;
} MyData;

int
my_sum(int a, int b)
{
    int result = a + b;

    for (int i = 0; i < result; i++) {
        result += i;
    }

    return result;
}
