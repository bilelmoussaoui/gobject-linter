static void no_coroutine_fn
same_name(void)
{
}

static void
caller_b(void)
{
    same_name();
}

