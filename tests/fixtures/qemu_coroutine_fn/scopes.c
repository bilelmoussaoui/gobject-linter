typedef void coroutine_fn (*CoroCallback)(void);
typedef void (*PlainCallback)(void);

void coroutine_fn coro_target(void);
void plain_target(void);
int qemu_in_coroutine(void);

static void
shadow_same_type_callback(void)
{
    PlainCallback callback = coro_target;

    {
        PlainCallback callback = plain_target;
        callback();
    }
    callback();
}

static void
shadow_parameter(CoroCallback callback)
{
    {
        PlainCallback callback = plain_target;
        callback();
    }
    callback();
}

static void
shadow_plain_callback(void)
{
    PlainCallback callback = plain_target;

    {
        CoroCallback callback = coro_target;
        callback();
    }
    callback();
}

static void coroutine_mixed_fn
shadow_in_guard(void)
{
    CoroCallback callback = coro_target;

    if (qemu_in_coroutine()) {
        PlainCallback callback = plain_target;
        callback();
    }
    callback();
}
