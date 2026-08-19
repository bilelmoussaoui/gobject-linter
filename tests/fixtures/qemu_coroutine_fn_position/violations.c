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
