void coroutine_fn coro_callee(void);
void no_coroutine_fn blocking_callee(void);

typedef void (*Callback)(void);

void coroutine_fn register_co(Callback callback);
void register_main(Callback callback);

static void
mixed_guard(void)
{
    if (qemu_in_coroutine()) {
        coro_callee();
    } else {
        blocking_callee();
    }
}

static void
guarded_coroutine_call(void)
{
    if (qemu_in_coroutine()) {
        coro_callee();
    }
}

static void
asserted_coroutine(void)
{
    assert(qemu_in_coroutine());
    coro_callee();
}

static void coroutine_mixed_fn
annotated_mixed_guard(void)
{
    if (qemu_in_coroutine()) {
        coro_callee();
    } else {
        blocking_callee();
    }
}

static void
mixed_callback(void)
{
}

static void
main_callback(void)
{
}

static void
main_registration(void)
{
    register_main(&mixed_callback);
    register_main(main_callback);
}

static void coroutine_fn
co_registration(void)
{
    register_co(mixed_callback);
}
