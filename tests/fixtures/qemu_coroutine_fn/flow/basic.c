void coroutine_fn coro_call(void);
void no_coroutine_fn blocking_call(void);
void co_wrapper_mixed mixed_wrapper(void);
int qemu_in_coroutine(void);
void assert(int condition);

static void coroutine_mixed_fn
guarded_calls(void)
{
    mixed_wrapper();

    if (qemu_in_coroutine()) {
        coro_call();
        mixed_wrapper();
    } else {
        blocking_call();
    }

    if (!qemu_in_coroutine()) {
        blocking_call();
        mixed_wrapper();
    } else {
        coro_call();
    }

    assert(qemu_in_coroutine());
    coro_call();
    blocking_call();
}

static void coroutine_mixed_fn
exiting_guard(void)
{
    if (!qemu_in_coroutine()) {
        return;
    }
    coro_call();
}

