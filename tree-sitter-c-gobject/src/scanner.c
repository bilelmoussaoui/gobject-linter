#include "tree_sitter/parser.h"
#include <string.h>

typedef enum {
    GOBJECT_MACRO_NAME,            /* G_DECLARE_* / G_DEFINE_* (not _WITH_CODE) */
    GOBJECT_MACRO_NAME_WITH_CODE,  /* G_DEFINE_*_WITH_CODE                      */
    GOBJECT_BEGIN_DECLS,           /* G_BEGIN_DECLS                              */
    GOBJECT_END_DECLS,             /* G_END_DECLS                                */
    MACRO_MODIFIER_NAME,           /* any other ALL_CAPS ident                  */
    GOBJECT_EXPORT_MACRO,          /* ALL_CAPS ident immediately before G_DECLARE_* or G_DEFINE_* */
    GOBJECT_IGNORE_MACRO,          /* G_GNUC_BEGIN_IGNORE_DEPRECATIONS etc.     */
    G_ALLOCATION_FUNCTION,         /* g_new, g_renew, g_slice_new, etc.         */
    OBJC_BLOCK,                    /* @interface...@end / @implementation...@end / @protocol...@end */
    OBJC_CLASS_FORWARD,            /* @class Foo;                               */
    OBJC_SELECTOR_EXPR,            /* @selector(name:)                          */
    OBJC_STRING_LITERAL,           /* @"string"                                 */
    OBJC_MESSAGE_EXPR,             /* [obj message:arg]                         */
} TokenType;

void *tree_sitter_c_gobject_external_scanner_create(void) { return NULL; }
void tree_sitter_c_gobject_external_scanner_destroy(void *payload) { (void)payload; }
void tree_sitter_c_gobject_external_scanner_reset(void *payload) { (void)payload; }
unsigned tree_sitter_c_gobject_external_scanner_serialize(void *payload, char *buffer) {
    (void)payload; (void)buffer;
    return 0;
}
void tree_sitter_c_gobject_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    (void)payload; (void)buffer; (void)length;
}

static void skip_whitespace(TSLexer *lexer) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
           lexer->lookahead == '\n' || lexer->lookahead == '\r') {
        lexer->advance(lexer, true);
    }
}

/* Like skip_whitespace but uses advance(..., false) so it does NOT reset
 * token_start_position.  Must be used for lookahead that happens after
 * mark_end() has already pinned the token boundary. */
static void lookahead_skip_whitespace(TSLexer *lexer) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
           lexer->lookahead == '\n' || lexer->lookahead == '\r') {
        lexer->advance(lexer, false);
    }
}

/* Advance past a balanced argument list starting at '(' (already confirmed). */
static void skip_argument_list(TSLexer *lexer) {
    int depth = 0;
    while (lexer->lookahead) {
        char c = (char)lexer->lookahead;
        lexer->advance(lexer, false);
        if (c == '(') depth++;
        else if (c == ')') { if (--depth == 0) return; }
    }
}

/* Check whether what follows the already-read ALL_CAPS identifier is '->'
 * (expression context: type-cast macro like G_OBJECT_CLASS(x)->method).
 * The scanner resets its position on false return, so reads here are safe. */
static bool followed_by_arrow(TSLexer *lexer) {
    lookahead_skip_whitespace(lexer);
    if (lexer->lookahead == '(') {
        skip_argument_list(lexer);
        lookahead_skip_whitespace(lexer);
    }
    return lexer->lookahead == '-';
}

/* Peek ahead (after whitespace) to check whether the next ALL_CAPS identifier
 * is a GObject macro name (G_DECLARE_* or G_DEFINE_*).
 * Call mark_end() BEFORE calling this so the token boundary is already saved.
 * Returns true if followed by a GObject macro, false otherwise.
 * Advances the lexer (caller relies on mark_end for the correct token end). */
/* Check whether buf contains both _DEFINE_ and _TYPE (project-specific
 * type-definition macros such as GDK_DEFINE_EVENT_TYPE,
 * _G_DEFINE_TYPE_EXTENDED_WITH_PRELUDE, GI_DEFINE_BASE_INFO_TYPE).
 * All custom define macros are treated as WITH_CODE since the grammar
 * for WITH_CODE is a superset that handles code blocks when present. */
static int custom_define_type_match(const char *buf, int len) {
    (void)len;
    if (strstr(buf, "_DEFINE_") == NULL) return 0;
    if (strstr(buf, "_TYPE") == NULL) return 0;
    return 1;
}

static bool followed_by_gobject_macro(TSLexer *lexer) {
    lookahead_skip_whitespace(lexer);

    char buf[256];
    int len = 0;
    while (len < 255 &&
           (lexer->lookahead == '_' ||
            (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z') ||
            (lexer->lookahead >= '0' && lexer->lookahead <= '9'))) {
        buf[len++] = (char)lexer->lookahead;
        lexer->advance(lexer, false);
    }
    buf[len] = '\0';

    return (len >= 10 && strncmp(buf, "G_DECLARE_", 10) == 0) ||
           (len >= 9  && strncmp(buf, "G_DEFINE_",   9) == 0) ||
           custom_define_type_match(buf, len) > 0;
}

/* Read a lowercase keyword after '@' into buf.  Returns the length. */
static int read_objc_keyword(TSLexer *lexer, char *buf, int max) {
    int len = 0;
    while (len < max - 1 &&
           lexer->lookahead >= 'a' && lexer->lookahead <= 'z') {
        buf[len++] = (char)lexer->lookahead;
        lexer->advance(lexer, false);
    }
    buf[len] = '\0';
    return len;
}

/* Consume everything from the current position until @end (inclusive).
 * Assumes '@interface', '@implementation', or '@protocol' was already consumed. */
static void consume_until_at_end(TSLexer *lexer) {
    while (lexer->lookahead) {
        if (lexer->lookahead == '@') {
            lexer->advance(lexer, false);
            char kw[16];
            int kw_len = read_objc_keyword(lexer, kw, sizeof(kw));
            if (kw_len == 3 && strcmp(kw, "end") == 0) {
                return;
            }
        } else {
            lexer->advance(lexer, false);
        }
    }
}

/* Try to scan an Objective-C construct starting with '@'. */
static bool scan_objc(TSLexer *lexer, const bool *valid_symbols) {
    bool any_objc = valid_symbols[OBJC_BLOCK]          ||
                    valid_symbols[OBJC_CLASS_FORWARD]   ||
                    valid_symbols[OBJC_SELECTOR_EXPR]   ||
                    valid_symbols[OBJC_STRING_LITERAL];
    if (!any_objc) return false;

    /* Advance past '@' */
    lexer->advance(lexer, false);

    /* @"string" */
    if (valid_symbols[OBJC_STRING_LITERAL] && lexer->lookahead == '"') {
        lexer->advance(lexer, false);
        while (lexer->lookahead && lexer->lookahead != '"') {
            if (lexer->lookahead == '\\') lexer->advance(lexer, false);
            if (lexer->lookahead) lexer->advance(lexer, false);
        }
        if (lexer->lookahead == '"') lexer->advance(lexer, false);
        lexer->result_symbol = OBJC_STRING_LITERAL;
        return true;
    }

    /* @[ array literal — consume as @"..." would be too complex; just let
     * tree-sitter see it as an expression.  Skip for now. */

    /* Read the keyword */
    char kw[32];
    int kw_len = read_objc_keyword(lexer, kw, sizeof(kw));
    if (kw_len == 0) return false;

    /* @interface / @implementation / @protocol ... @end */
    if (valid_symbols[OBJC_BLOCK] &&
        (strcmp(kw, "interface") == 0 ||
         strcmp(kw, "implementation") == 0 ||
         strcmp(kw, "protocol") == 0)) {
        consume_until_at_end(lexer);
        lexer->result_symbol = OBJC_BLOCK;
        return true;
    }

    /* @class Foo; or @class Foo, Bar; */
    if (valid_symbols[OBJC_CLASS_FORWARD] && strcmp(kw, "class") == 0) {
        while (lexer->lookahead && lexer->lookahead != ';') {
            lexer->advance(lexer, false);
        }
        if (lexer->lookahead == ';') lexer->advance(lexer, false);
        lexer->result_symbol = OBJC_CLASS_FORWARD;
        return true;
    }

    /* @selector(name:) */
    if (valid_symbols[OBJC_SELECTOR_EXPR] && strcmp(kw, "selector") == 0) {
        lookahead_skip_whitespace(lexer);
        if (lexer->lookahead == '(') {
            skip_argument_list(lexer);
            lexer->result_symbol = OBJC_SELECTOR_EXPR;
            return true;
        }
    }

    return false;
}

bool tree_sitter_c_gobject_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    (void)payload;

    bool any_valid = valid_symbols[GOBJECT_MACRO_NAME]           ||
                     valid_symbols[GOBJECT_MACRO_NAME_WITH_CODE]  ||
                     valid_symbols[GOBJECT_BEGIN_DECLS]           ||
                     valid_symbols[GOBJECT_END_DECLS]             ||
                     valid_symbols[MACRO_MODIFIER_NAME]           ||
                     valid_symbols[GOBJECT_EXPORT_MACRO]          ||
                     valid_symbols[G_ALLOCATION_FUNCTION]         ||
                     valid_symbols[OBJC_BLOCK]                    ||
                     valid_symbols[OBJC_CLASS_FORWARD]            ||
                     valid_symbols[OBJC_SELECTOR_EXPR]            ||
                     valid_symbols[OBJC_STRING_LITERAL]           ||
                     valid_symbols[OBJC_MESSAGE_EXPR];
    if (!any_valid) return false;

    skip_whitespace(lexer);

    /* g_new, g_renew, g_slice_new, etc. - GLib memory allocation functions */
    if (valid_symbols[G_ALLOCATION_FUNCTION] && lexer->lookahead == 'g') {
        char buf[32];
        int len = 0;
        /* Read identifier (lowercase, underscore, digits) */
        while (len < 31 &&
               (lexer->lookahead == '_' ||
                (lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
                (lexer->lookahead >= '0' && lexer->lookahead <= '9'))) {
            buf[len++] = (char)lexer->lookahead;
            lexer->advance(lexer, false);
        }
        buf[len] = '\0';

        /* Match specific allocation function names */
        if (strcmp(buf, "g_new") == 0 ||
            strcmp(buf, "g_new0") == 0 ||
            strcmp(buf, "g_newa") == 0 ||
            strcmp(buf, "g_renew") == 0 ||
            strcmp(buf, "g_try_new") == 0 ||
            strcmp(buf, "g_try_new0") == 0 ||
            strcmp(buf, "g_try_renew") == 0 ||
            strcmp(buf, "g_slice_new") == 0 ||
            strcmp(buf, "g_slice_new0") == 0 ||
            strcmp(buf, "g_slice_dup") == 0 ||
            strcmp(buf, "g_slice_free") == 0) {
            lexer->result_symbol = G_ALLOCATION_FUNCTION;
            return true;
        }
        /* Not a match, return false so the regular lexer handles it */
        return false;
    }

    /* Objective-C constructs starting with '@' */
    if (lexer->lookahead == '@') {
        return scan_objc(lexer, valid_symbols);
    }

    /* Objective-C message send: [receiver message:arg]
     * In C, '[' at expression-start is never valid (subscript requires a
     * preceding expression), so consuming balanced [...] is safe here. */
    if (valid_symbols[OBJC_MESSAGE_EXPR] && lexer->lookahead == '[') {
        int depth = 0;
        do {
            if (lexer->lookahead == '[') depth++;
            else if (lexer->lookahead == ']') depth--;
            lexer->advance(lexer, false);
        } while (depth > 0 && lexer->lookahead);
        lexer->result_symbol = OBJC_MESSAGE_EXPR;
        return true;
    }

    /* Must start with an uppercase letter or underscore */
    if (!((lexer->lookahead >= 'A' && lexer->lookahead <= 'Z') ||
          lexer->lookahead == '_')) {
        return false;
    }

    /* Consume only uppercase letters, digits, and underscores.
     * Stops at the first lowercase letter, which means CamelCase identifiers
     * like GObject only contribute their uppercase prefix. */
    char buf[256];
    int len = 0;
    while (len < 255 &&
           (lexer->lookahead == '_' ||
            (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z') ||
            (lexer->lookahead >= '0' && lexer->lookahead <= '9'))) {
        buf[len++] = (char)lexer->lookahead;
        lexer->advance(lexer, false);
    }
    buf[len] = '\0';

    /* If the very next character is a lowercase letter the original identifier
     * is CamelCase (e.g. GObject, MyType) — not a macro, so bail out and let
     * the regular lexer produce an identifier token. */
    if (lexer->lookahead >= 'a' && lexer->lookahead <= 'z') {
        return false;
    }

    /* G_DEFINE_TYPE_EXTENDED — 5-arg variant with a code block as the last arg,
     * same structure as *_WITH_CODE macros. Must be checked before the general
     * G_DEFINE_* rule so the more-specific token wins. */
    if (valid_symbols[GOBJECT_MACRO_NAME_WITH_CODE] &&
        strcmp(buf, "G_DEFINE_TYPE_EXTENDED") == 0) {
        lexer->result_symbol = GOBJECT_MACRO_NAME_WITH_CODE;
        return true;
    }

    /* G_DEFINE_*_WITH_CODE — must be checked before the general G_DEFINE_* rule
     * so the more-specific token wins. */
    if (valid_symbols[GOBJECT_MACRO_NAME_WITH_CODE] &&
        len >= 9 && strncmp(buf, "G_DEFINE_", 9) == 0 &&
        len >= 10 && strncmp(buf + len - 10, "_WITH_CODE", 10) == 0) {
        lexer->result_symbol = GOBJECT_MACRO_NAME_WITH_CODE;
        return true;
    }

    /* G_DECLARE_* / G_DEFINE_* — GObject type-system macros */
    if (valid_symbols[GOBJECT_MACRO_NAME] &&
        ((len >= 10 && strncmp(buf, "G_DECLARE_", 10) == 0) ||
         (len >= 9  && strncmp(buf, "G_DEFINE_",   9) == 0))) {
        lexer->result_symbol = GOBJECT_MACRO_NAME;
        return true;
    }

    /* Project-specific *_DEFINE_*_TYPE / *_DEFINE_*_TYPE_WITH_CODE macros
     * (e.g. GDK_DEFINE_EVENT_TYPE, GSK_DEFINE_RENDER_NODE_TYPE,
     * GTK_DEFINE_BUILTIN_MODULE_TYPE_WITH_CODE). */
    {
        int m = custom_define_type_match(buf, len);
        if (m == 1 && valid_symbols[GOBJECT_MACRO_NAME_WITH_CODE]) {
            lexer->result_symbol = GOBJECT_MACRO_NAME_WITH_CODE;
            return true;
        }
        if (m == 2 && valid_symbols[GOBJECT_MACRO_NAME]) {
            lexer->result_symbol = GOBJECT_MACRO_NAME;
            return true;
        }
    }

    if (valid_symbols[GOBJECT_BEGIN_DECLS] && strcmp(buf, "G_BEGIN_DECLS") == 0) {
        lexer->result_symbol = GOBJECT_BEGIN_DECLS;
        return true;
    }

    if (valid_symbols[GOBJECT_END_DECLS] && strcmp(buf, "G_END_DECLS") == 0) {
        lexer->result_symbol = GOBJECT_END_DECLS;
        return true;
    }

    /* G_GNUC_BEGIN_IGNORE_DEPRECATIONS / G_GNUC_END_IGNORE_DEPRECATIONS —
     * standalone macros that expand to _Pragma directives.  They appear
     * without a semicolon inside function bodies and at top-level.
     * Emit them as a zero-width ignored token so the parser skips them. */
    if (valid_symbols[GOBJECT_IGNORE_MACRO] &&
        (strcmp(buf, "G_GNUC_BEGIN_IGNORE_DEPRECATIONS") == 0 ||
         strcmp(buf, "G_GNUC_END_IGNORE_DEPRECATIONS") == 0)) {
        lexer->result_symbol = GOBJECT_IGNORE_MACRO;
        return true;
    }

    /* Pin the token boundary to the end of the ALL_CAPS identifier.  All
     * subsequent advances are look-ahead only. */
    lexer->mark_end(lexer);

    /* GOBJECT_EXPORT_MACRO: identifier immediately before G_DECLARE_* or G_DEFINE_* */
    if (valid_symbols[GOBJECT_EXPORT_MACRO] && len >= 1) {
        if (followed_by_gobject_macro(lexer)) {
            lexer->result_symbol = GOBJECT_EXPORT_MACRO;
            return true;
        }
        /* followed_by_gobject_macro() advanced into the next identifier.
         * After this point the lexer sits at whatever follows that identifier
         * (or at '(' if there was no next identifier). */
    }

    /* MACRO_MODIFIER_NAME: only emit for known modifier patterns, not every
     * ALL_CAPS identifier.  This prevents Windows types (LRESULT, BOOL, VOID,
     * DWORD, etc.) from being swallowed as modifiers. */
    if (valid_symbols[MACRO_MODIFIER_NAME] && len >= 1) {
        if (followed_by_arrow(lexer)) return false;
        bool is_modifier =
            strstr(buf, "_EXPORT") != NULL ||
            strstr(buf, "_DEPRECATED") != NULL ||
            strstr(buf, "_AVAILABLE") != NULL ||
            strstr(buf, "_UNAVAILABLE") != NULL ||
            strstr(buf, "_ENUMERATOR_") != NULL ||
            (len >= 7 && strncmp(buf, "G_GNUC_", 7) == 0) ||
            strstr(buf, "_INLINE") != NULL;
        if (!is_modifier) {
            /* An ALL_CAPS identifier followed by struct/union/enum is an
             * attribute macro (e.g. SECTION, PACKED) — never a type name. */
            lookahead_skip_whitespace(lexer);
            char kw[8];
            int kw_len = 0;
            while (kw_len < 6 &&
                   lexer->lookahead >= 'a' && lexer->lookahead <= 'z') {
                kw[kw_len++] = (char)lexer->lookahead;
                lexer->advance(lexer, false);
            }
            kw[kw_len] = '\0';
            is_modifier = (strcmp(kw, "struct") == 0 ||
                           strcmp(kw, "union") == 0 ||
                           strcmp(kw, "enum") == 0);
        }
        if (!is_modifier) return false;
        lexer->result_symbol = MACRO_MODIFIER_NAME;
        return true;
    }

    return false;
}
