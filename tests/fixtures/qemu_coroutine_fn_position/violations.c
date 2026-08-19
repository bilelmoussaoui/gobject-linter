/* Declaration: modifier between return type and name */
void coroutine_fn coro_decl(void);
void no_coroutine_fn noncoro_decl(void);
void coroutine_mixed_fn mixed_decl(void);

/* Definition: modifier between return type and name (same line) */
static void coroutine_fn
coro_def(void)
{
}

/* Definition: modifier between return type and name (all on one line) */
static void no_coroutine_fn noncoro_def(void)
{
}

/* Pointer return type with modifier after it */
static int *coroutine_fn ptr_coro_def(void)
{
    return 0;
}

/* Struct with function pointer fields */
typedef struct V9fsPDU V9fsPDU;

struct V9fsTransport {
    void        coroutine_fn (*push_and_notify)(V9fsPDU *pdu);
    int         coroutine_fn (*msize_limit)(int s);
};

/* Wrapper markers belong before the return type */
int64_t co_wrapper_mixed misplaced_wrapper_decl(void);
void no_co_wrapper_bdrv_rdlock misplaced_no_wrapper_decl(void);

/* Wrapper declarations take coroutine_fn before the return type as well */
void coroutine_fn no_co_wrapper co_wrapper_decl(void);

struct V9fsTransport2 {
    void co_wrapper_mixed (*wrapped)(void);
};

/* Storage class specifiers keep their place */
static inline int coroutine_fn inline_coro_def(void)
{
    return 0;
}
