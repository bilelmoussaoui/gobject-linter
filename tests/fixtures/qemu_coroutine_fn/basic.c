void coroutine_fn coro_callee(void);
void no_coroutine_fn blocking_callee(void);
void co_wrapper_mixed mixed_wrapper(void);
void coroutine_mixed_fn mixed_callee(void);

typedef void coroutine_fn (*CoroCallback)(void);

struct Driver {
    void coroutine_fn (*run)(void);
};

static void
plain_calls(CoroCallback callback, struct Driver *driver)
{
    CoroCallback local = coro_callee;

    coro_callee();
    callback();
    driver->run();
    local();
    mixed_callee();
}

static void coroutine_fn
coroutine_calls(void)
{
    blocking_callee();
    mixed_wrapper();
}

static void coroutine_mixed_fn
mixed_calls(void)
{
    coro_callee();
    mixed_wrapper();

    if (qemu_in_coroutine()) {
        coro_callee();
    }
}
