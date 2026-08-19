void no_coroutine_fn blocking_call(void);

static void shadowed_declaration(void);
static void coroutine_fn inherited_contract(void);

static void shadowed_definition(void)
{
    blocking_call();
}

static void inherited_contract(void)
{
    shadowed_definition();
}

static void caller(void)
{
    shadowed_definition();
    shadowed_declaration();
    inherited_contract();
}
