/* Already correct: modifier before return type */
coroutine_fn void coro_decl_ok(void);
no_coroutine_fn void noncoro_decl_ok(void);
coroutine_mixed_fn void mixed_decl_ok(void);

/* Already correct: modifier before static */
coroutine_fn static void
coro_def_ok(void)
{
}

/* No modifier at all */
static void
plain_func(void)
{
}

/* Wrapper markers go before the return type too */
co_wrapper_mixed int wrapped_decl_ok(void);
coroutine_fn no_co_wrapper void co_wrapper_decl_ok(void);
