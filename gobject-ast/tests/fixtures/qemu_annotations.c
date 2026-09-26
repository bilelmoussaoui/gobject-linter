typedef void coroutine_fn CoroutineEntry(void);
typedef int no_coroutine_fn (*BlockingCallback)(void);

typedef struct Ops {
    void coroutine_fn (*run)(void);
    int co_wrapper_mixed (*dispatch)(void);
} Ops;

void coroutine_fn declared(void);
void coroutine_mixed_fn no_coroutine_fn mixed_only(void);
QMPRequest * coroutine_fn pointer_return(void);

coroutine_fn static void
defined(void)
{
}

