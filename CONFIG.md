# Configuration

## Config File

Create a `gobject-linter.toml` file in your project root:

```toml
# Set minimum GLib version (disables rules that require newer versions)
min_glib_version = "2.76"

# Target MSVC-compatible code (disables g_auto* cleanup attributes)
# When true: enables no_g_auto_macros forbidding all usage of g_auto macros
msvc_compatible = true

# Editor URL template for clickable links in output
# {path}, {line}, {column} are replaced with actual values
editor_url = "vscode://file{path}:{line}:{column}"

# Global ignore patterns (glob syntax)
ignore = [
    "target/**",
    "**/build/**",
    "vendor/**",
]

# Output format, one of "text", "json", "sarif", "gcc", or "gitlab-codequality"
format = "text"

# Function call style: space before parenthesis (GLib convention, default)
# Set to false for no-space style: `g_new(Type, 1)`
[style]
space_before_paren = true

# Configure individual rules
[rules.use_g_strlcpy]
level = "error"  # "error", "warn", or "ignore"

[rules.use_g_new]
level = "warn"
ignore = ["tests/**"]  # Ignore this rule for files matching these globs
```

### Rule Levels

- `error` - Fails the linter (exit code 1)
- `warn` - Reports but doesn't fail
- `ignore` - Disables the rule completely

### Default Level

To make all non-opt-in rules fail instead of warn, set `default_level`:

```toml
default_level = "error"
```

This applies to every non-opt-in rule that does not have an explicit `level` set. Opt-in rules (`dead_code`, `missing_export_macro`) are unaffected and must still be enabled explicitly.

### Global Ignore Patterns

The top-level `ignore` field skips files/directories for **all rules**:

```toml
ignore = [
    "vendor/**",        # Ignore entire vendor directory
    "**/test/**",       # Ignore all test directories
    "generated/*.c",    # Ignore generated C files
    "**/*-autogen.c",   # Ignore all auto-generated files
]
```

**Note:** gobject-linter automatically respects `.gitignore` files. Files/directories ignored by git are also ignored by the linter.

### Per-Rule Ignores

Use the rule-level `ignore` field to skip files for **specific rules only**:

```toml
[rules.use_g_autoptr]
level = "error"
ignore = [
    "tests/**",
    "examples/*.c",
    "legacy/old-code.c"
]
```

### Per-Rule Configuration Options

Some rules accept additional configuration that are documented in https://bilelmoussaoui.github.io/gobject-linter/.

## Inline Ignore Directives

Suppress violations on a specific line using comments:

```c
// Ignore next line for a specific rule
/* gobject-linter-ignore-next-line: use_g_strlcpy */
strcpy(dst, src);

// Ignore multiple rules (comma-separated)
/* gobject-linter-ignore-next-line: use_g_new, use_g_strlcpy */
char *ptr = malloc(100);

// Ignore all rules with wildcard
/* gobject-linter-ignore-next-line: all */
strcpy(dst, src);

// C++ style comments work too
// gobject-linter-ignore-next-line: use_g_strlcpy
strcpy(dst, src);
```

**Note:** Invalid rule names will produce a warning but won't suppress violations.

## List All Rules

```bash
gobject-linter --list-rules
```

This shows all available rules with their current status (error/warn/ignore) based on your config.
