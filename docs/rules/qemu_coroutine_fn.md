Checks that QEMU code respects the coroutine calling-context annotations:
a function annotated `coroutine_fn` may only run in coroutine context, a
`no_coroutine_fn` function only in non-coroutine context, and
`coroutine_mixed_fn` (or a `co_wrapper_mixed` wrapper) in both.

This rule is opt-in and disabled by default. Enable it for a QEMU tree with
`--only qemu:coroutine_fn` or in the configuration file:

```toml
[rules]
"qemu:coroutine_fn" = "error"
```

## Annotation model

| Annotation | Call from coroutine | Call from non-coroutine |
|---|---|---|
| `coroutine_fn` | ok | reported |
| `no_coroutine_fn` | reported | ok |
| `coroutine_mixed_fn` | ok | ok |
| `co_wrapper` / `co_wrapper_bdrv_rdlock` | reported | ok |
| `co_wrapper_mixed` / `co_wrapper_mixed_bdrv_rdlock` | ok | ok |
| `no_co_wrapper*` | unchanged by the wrapper | unchanged by the wrapper |

Passing `qemu_in_coroutine()` as a condition splits the flow into a coroutine
and a non-coroutine branch, and `assert(qemu_in_coroutine())` makes the rest of
the block coroutine context. Indirect calls through an annotated function
typedef or struct field (for example `CoroutineEntry`) are resolved as well.

## Missing annotations

A function that is used from both contexts but carries no annotation is
reported, even when every individual call is guarded:

```c
static int block_crypto_read_func(QCryptoBlock *block, size_t offset, ...)
{
    if (qemu_in_coroutine()) {
        return bdrv_co_pread(...);      /* coroutine only */
    }
    /* falls back to the blocking path, so the function is mixed */
    return bdrv_pread(...);
}
```

```text
`block_crypto_read_func` branches on qemu_in_coroutine() but is not annotated
coroutine_mixed_fn
```

Callbacks are tracked through the parameters of the function they are passed
to, so a function registered with both a coroutine and a non-coroutine caller
is reported too:

```c
static void my_read_func(...) { }       /* no annotation */

static void coroutine_fn open_co(void)  { qcrypto_block_open(..., my_read_func, ...); }
static void open_main(void)            { qcrypto_block_open(..., my_read_func, ...); }
```

```text
`my_read_func` is registered as a callback from both coroutine and
non-coroutine contexts; it should be coroutine_mixed_fn
```

## Limitations

- A `bool` parameter that selects the callable path is not correlated with the
  call site, so a mixed function guarded by such a flag is reported.
- Only a function passed directly as a call argument is treated as a callback
  registration. Assigning one to a struct field (`obj->cb = func`) is not, even
  though indirect calls through an annotated field are checked.
- Context is not propagated through helper wrappers that forward a callback
  function without annotating it.
