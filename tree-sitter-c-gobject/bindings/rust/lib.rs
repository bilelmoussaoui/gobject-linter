use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_c_gobject() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_c_gobject) };

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    fn parse(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading C gobject parser");
        parser.parse(source, None).expect("parser returned no tree")
    }

    #[test]
    fn test_can_load_grammar() {
        parse("");
    }

    #[test]
    fn qemu_modifiers_parse_in_supported_declarator_positions() {
        let source = r#"
            void coroutine_fn direct(void);
            typedef void coroutine_fn (*Callback)(void);
            struct Ops { void coroutine_fn (*run)(void); };
            QMPRequest * coroutine_fn pointer_return(void);
            co_wrapper_mixed int wrapper(void);
            GRAPH_RDLOCK void graph_locked(void);
            TSA_REQUIRES(lock) void guarded(void);
            typedef BlockBackend * coroutine_fn GRAPH_UNLOCKED_PTR
                (*ExtentFn)(int size);
            static inline GraphLockable * TSA_ACQUIRE_SHARED(graph_lock)
                coroutine_fn graph_lockable_auto_lock(GraphLockable *x) { return x; }
            struct BlockDriver {
                BlockAIOCB *coroutine_fn GRAPH_RDLOCK_PTR (*bdrv_aio_ioctl)(void);
            };
            typedef struct TSA_CAPABILITY("mutex") BdrvGraphLock {
            } BdrvGraphLock;
        "#;
        let tree = parse(source);
        assert!(
            !tree.root_node().has_error(),
            "{}",
            tree.root_node().to_sexp()
        );
        let sexp = tree.root_node().to_sexp();
        assert!(sexp.matches("macro_modifier").count() >= 7, "{sexp}");
    }
}
