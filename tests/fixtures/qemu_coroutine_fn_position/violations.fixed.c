/* Declaration: modifier between return type and name */
coroutine_fn void coro_decl(void);
no_coroutine_fn void noncoro_decl(void);
coroutine_mixed_fn void mixed_decl(void);

/* Definition: modifier between return type and name (same line) */
static coroutine_fn void
coro_def(void)
{
}

/* Definition: modifier between return type and name (all on one line) */
static no_coroutine_fn void noncoro_def(void)
{
}

/* Pointer return type with modifier after it */
static coroutine_fn int *ptr_coro_def(void)
{
    return 0;
}

/* Struct with function pointer fields */
typedef struct V9fsPDU V9fsPDU;

struct V9fsTransport {
    coroutine_fn void        (*push_and_notify)(V9fsPDU *pdu);
    coroutine_fn int         (*msize_limit)(int s);
};

/* Wrapper markers belong before the return type */
co_wrapper_mixed int64_t misplaced_wrapper_decl(void);
no_co_wrapper_bdrv_rdlock void misplaced_no_wrapper_decl(void);

/* Wrapper declarations take coroutine_fn before the return type as well */
coroutine_fn no_co_wrapper void co_wrapper_decl(void);

struct V9fsTransport2 {
    co_wrapper_mixed void (*wrapped)(void);
};

/* Storage class specifiers keep their place */
static inline coroutine_fn int inline_coro_def(void)
{
    return 0;
}
