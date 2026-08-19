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
