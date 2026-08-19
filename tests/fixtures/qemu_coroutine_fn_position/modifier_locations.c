/* Correct annotations must not match strings or comments after the type. */
coroutine_fn void coro_body(void)
{
    const char *label = "coroutine_fn";
    /* coroutine_fn stays in this comment. */
    (void)label;
}

no_coroutine_fn void noncoro_body(void)
{
    const char *label = "no_coroutine_fn";
    (void)label;
}

coroutine_mixed_fn void mixed_body(void)
{
    const char *label = "coroutine_mixed_fn";
    (void)label;
}

coroutine_fn void /* coroutine_fn */ correct_decl(void);

/* Fix the actual annotation, preserving the preceding comment. */
void /* coroutine_fn */ coroutine_fn misplaced_decl(void);

static int * /* no_coroutine_fn */ no_coroutine_fn misplaced_body(void)
{
    const char *label = "no_coroutine_fn";
    (void)label;
    return 0;
}

struct Ops {
    coroutine_fn void /* coroutine_fn */ (*correct)(void);
    void /* coroutine_mixed_fn */ coroutine_mixed_fn (*misplaced)(void);
};
