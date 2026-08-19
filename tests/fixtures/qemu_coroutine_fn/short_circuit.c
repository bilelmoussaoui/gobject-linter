int coroutine_fn coro_pred(void);
int no_coroutine_fn plain_pred(void);
int qemu_in_coroutine(void);
void consume(int value);

static int coroutine_mixed_fn
guarded_expressions(int ready)
{
    qemu_in_coroutine() && coro_pred();
    !qemu_in_coroutine() && plain_pred();
    !qemu_in_coroutine() || coro_pred();
    qemu_in_coroutine() || plain_pred();

    consume(qemu_in_coroutine() && coro_pred());
    consume(ready && (qemu_in_coroutine() && coro_pred()));
    int result = !qemu_in_coroutine() || coro_pred();
    result = qemu_in_coroutine() && coro_pred();
    result += !(qemu_in_coroutine() && coro_pred());

    return !qemu_in_coroutine() || coro_pred();
}

static int coroutine_mixed_fn
invalid_expressions(void)
{
    qemu_in_coroutine() && plain_pred();
    !qemu_in_coroutine() && coro_pred();
    !qemu_in_coroutine() || plain_pred();
    qemu_in_coroutine() || coro_pred();

    consume(qemu_in_coroutine() || coro_pred());
    coro_pred() && qemu_in_coroutine();
    qemu_in_coroutine() && coro_pred();
    coro_pred();

    return qemu_in_coroutine() || coro_pred();
}
