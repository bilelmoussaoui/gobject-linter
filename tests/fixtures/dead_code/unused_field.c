typedef struct {
    int read_field;       /* written and read — not dead */
    int write_only_field; /* written but never read — dead */
    int unused_field;     /* never accessed — dead */
} FooData;

static void
foo_init (FooData *data)
{
    data->read_field = 1;
    data->write_only_field = 2;
}

static int
foo_get (FooData *data)
{
    return data->read_field;
}
