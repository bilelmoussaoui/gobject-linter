int qemu_in_coroutine(void);
void assert(int condition);
void coroutine_fn coro_callee(void);

static void coroutine_mixed_fn
compound_assert(int oob)
{
    assert(!oob && qemu_in_coroutine());
    coro_callee();
}

static void coroutine_mixed_fn
plain_assert(void)
{
    assert(qemu_in_coroutine());
    coro_callee();
}

static void coroutine_mixed_fn
disjunctive_assert(int oob)
{
    assert(oob || qemu_in_coroutine());
    coro_callee();
}
