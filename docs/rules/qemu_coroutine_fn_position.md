Ensures coroutine-related annotations are written in the position QEMU
requires:

- `coroutine_fn`, `coroutine_mixed_fn` and `no_coroutine_fn` go before the
  return type:

  ```c
  coroutine_fn void foo(void);
  ```

  rather than between the return type and the function name:

  ```c
  void coroutine_fn foo(void);
  ```

- The `co_wrapper` family (`co_wrapper`, `co_wrapper_mixed`,
  `co_wrapper_bdrv_rdlock`, `co_wrapper_mixed_bdrv_rdlock`, `no_co_wrapper`,
  `no_co_wrapper_bdrv_rdlock`, `no_co_wrapper_bdrv_wrlock`) also goes before
  the return type:

  ```c
  co_wrapper_mixed int blk_pread(...);
  coroutine_fn no_co_wrapper void bdrv_co_unref(...);
  ```

Keeping the coroutine annotation first makes it part of the function signature
instead of the return type, and keeps declarations and definitions uniform.

This rule is opt-in and disabled by default. Enable it for a QEMU tree with
`--only qemu:coroutine_fn_position` or in the configuration file:

```toml
[rules]
"qemu:coroutine_fn_position" = "error"
```

## Examples

**Bad**:
```c
static void coroutine_fn
qcow2_co_foo(void)
{
}

static int *coroutine_fn ptr_coro_def(void)
{
    return NULL;
}
```

**Good**:
```c
static coroutine_fn void
qcow2_co_foo(void)
{
}

static coroutine_fn int *ptr_coro_def(void)
{
    return NULL;
}
```

Storage class specifiers keep their place: the annotation goes after `static`
or `inline` and before the return type.

Struct function pointer fields are checked as well:

```c
struct V9fsTransport {
    void coroutine_fn (*push_and_notify)(V9fsPDU *pdu);   /* bad */
    coroutine_fn void (*push_and_notify)(V9fsPDU *pdu);   /* good */
};
```

## Wrapper markers

`scripts/block-coroutine-wrapper.py` accepts annotations before the return
type, so wrapper markers are linted like the other annotations:

**Bad**:
```c
int64_t co_wrapper_mixed blk_getlength(BlockBackend *blk);
void no_co_wrapper bdrv_co_unref(BlockDriverState *bs);
void coroutine_fn no_co_wrapper bdrv_co_pread(...);
```

**Good**:
```c
co_wrapper_mixed int64_t blk_getlength(BlockBackend *blk);
no_co_wrapper void bdrv_co_unref(BlockDriverState *bs);
coroutine_fn no_co_wrapper void bdrv_co_pread(...);
```

## Fix

`--fix` moves the annotation in front of the return type, including when it
sits on its own line, and removes the leftover whitespace. Storage class
specifiers are left alone: `static void coroutine_fn foo(void)` becomes
`static coroutine_fn void foo(void)`.

## Notes

`--fix` moves every misplaced annotation in a single edit, before the return
type and after any annotation that is already in place, keeping their relative
order. The legacy spelling, with annotations between the return type and the
function name, still parses but is reported by the rule.
