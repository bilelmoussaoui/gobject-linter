#ifndef TREE_SITTER_C_GOBJECT_MODIFIERS_H_
#define TREE_SITTER_C_GOBJECT_MODIFIERS_H_

static bool qemu_is_lowercase_modifier(const char *name) {
    static const char *const modifiers[] = {
        "coroutine_fn",
        "coroutine_mixed_fn",
        "no_coroutine_fn",
        "co_wrapper",
        "co_wrapper_mixed",
        "co_wrapper_bdrv_rdlock",
        "co_wrapper_mixed_bdrv_rdlock",
        "no_co_wrapper",
        "no_co_wrapper_bdrv_rdlock",
        "no_co_wrapper_bdrv_wrlock",
    };

    for (unsigned i = 0; i < sizeof(modifiers) / sizeof(modifiers[0]); i++) {
        if (strcmp(name, modifiers[i]) == 0) return true;
    }
    return false;
}

static bool qemu_is_uppercase_modifier(const char *name, int len) {
    return (len >= 6 && strncmp(name, "GRAPH_", 6) == 0) ||
           (len >= 4 && strncmp(name, "TSA_", 4) == 0);
}

#endif
