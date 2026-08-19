int coroutine_fn coroutine_condition(void);

static void
plain_condition(void)
{
    if (coroutine_condition()) {
        return;
    }
}

