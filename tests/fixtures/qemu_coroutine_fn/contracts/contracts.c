#include "contracts.h"

void
inherited_contract(void)
{
}

void no_coroutine_fn
mismatched_contract(void)
{
}

static void
plain_caller(void)
{
    inherited_contract();
}

