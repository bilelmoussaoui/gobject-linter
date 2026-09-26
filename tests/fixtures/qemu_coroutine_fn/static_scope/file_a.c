static void coroutine_fn
same_name(void)
{
}

static void
caller_a(void)
{
    same_name();
}

