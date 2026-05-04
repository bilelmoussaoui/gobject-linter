typedef enum {
    FOO_MODE_NORMAL,
    FOO_MODE_COMPACT,
    FOO_MODE_MINI,   /* never referenced */
} FooMode;

static void
set_mode (FooMode mode)
{
    if (mode == FOO_MODE_NORMAL || mode == FOO_MODE_COMPACT)
        return;
}
