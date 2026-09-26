/* Declaration: modifier between return type and name */
coroutine_fn void coro_decl(void);
no_coroutine_fn void noncoro_decl(void);
coroutine_mixed_fn void mixed_decl(void);

/* Definition: modifier between return type and name (same line) */
coroutine_fn static void
coro_def(void)
{
}

/* Definition: modifier between return type and name (all on one line) */
no_coroutine_fn static void noncoro_def(void)
{
}

/* Pointer return type with modifier after it */
coroutine_fn static int *ptr_coro_def(void)
{
    return 0;
}

/* Struct with function pointer fields */
typedef struct V9fsPDU V9fsPDU;

struct V9fsTransport {
    coroutine_fn void        (*push_and_notify)(V9fsPDU *pdu);
    coroutine_fn int         (*msize_limit)(int s);
};
