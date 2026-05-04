typedef struct {
    int used_field;
    int unused_field; /* never accessed */
} FooData;

static void
foo_init (FooData *data)
{
    data->used_field = 1;
}
