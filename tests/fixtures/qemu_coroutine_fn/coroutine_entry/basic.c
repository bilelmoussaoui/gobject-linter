typedef void coroutine_fn CoroutineEntry(void *opaque);
typedef void coroutine_fn (*CoroCallback)(void);

struct Coroutine {
    CoroutineEntry *entry;
    void *entry_arg;
};

struct Worker {
    CoroCallback entry;
};

void coroutine_fn coro_body(void *opaque);
void no_coroutine_fn main_only(void);

static void trampoline(void *opaque)
{
    struct Coroutine *co = opaque;

    co->entry(co->entry_arg);
    co->entry(co->entry_arg);
}

static void not_a_trampoline(struct Worker *worker, void *opaque)
{
    worker->entry();
    coro_body(opaque);
    main_only();
}
