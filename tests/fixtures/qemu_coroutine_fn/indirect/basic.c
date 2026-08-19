typedef void coroutine_fn (*CoroCallback)(void);

struct CoroOps {
    void coroutine_fn (*run)(void);
};

struct PlainOps {
    void (*run)(void);
};

struct TypedOps {
    CoroCallback run;
};

void coroutine_fn coro_target(void);
void plain_target(void);

static void
indirect_calls(CoroCallback callback, CoroCallback callbacks[],
               struct CoroOps *coro_ops, struct PlainOps *plain_ops,
               struct TypedOps *typed_ops)
{
    CoroCallback local = coro_target;

    (*callback)();
    callbacks[0]();
    coro_ops->run();
    plain_ops->run();
    typed_ops->run();
    local();

    local = plain_target;
    local();
}

static void
ambiguous_assignment(int choose)
{
    CoroCallback local = coro_target;

    if (choose) {
        local = plain_target;
    }
    local();
}
