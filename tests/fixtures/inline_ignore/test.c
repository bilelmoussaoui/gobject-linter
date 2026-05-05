#include <string.h>
#include <glib.h>

typedef struct {
    char name[100];
} Data;

void test_inline_ignore(const char *input) {
    Data data;

    // This strcpy should be flagged - no ignore directive
    strcpy(data.name, input);

    // This strcpy should be ignored - C-style comment
    /* gobject-linter-ignore-next-line: use_g_strlcpy */
    strcpy(data.name, input);

    // This strcpy should also be ignored - C++ style comment
    // gobject-linter-ignore-next-line: use_g_strlcpy
    strcpy(data.name, input);

    // Multiple rules can be ignored (comma-separated)
    /* gobject-linter-ignore-next-line: use_g_strlcpy, some_other_rule */
    strcpy(data.name, input);

    // This one should be flagged again
    strcpy(data.name, input);
}
