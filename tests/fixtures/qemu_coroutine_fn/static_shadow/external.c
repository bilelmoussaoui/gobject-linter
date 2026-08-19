void coroutine_fn shadowed_definition(void);
void coroutine_fn shadowed_declaration(void);
void no_coroutine_fn blocking_call(void);

static void caller(void)
{
    shadowed_definition();
    shadowed_declaration();
}
