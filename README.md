# gobject-linter

[![crates.io](https://img.shields.io/crates/v/gobject-linter)](https://crates.io/crates/gobject-linter) ![CI](https://github.com/bilelmoussaoui/gobject-linter/workflows/CI/badge.svg)

A fast tree-sitter-based linter for GObject/C code.

Previously known as **goblint**.

## Usage

```bash
# Lint current directory with default config
gobject-linter

# Lint specific directory
gobject-linter /path/to/project

# Use custom config file
gobject-linter --config my-lint.toml /path/to/project

# Verbose output
gobject-linter -v

# List all available rules with their enabled/disabled status
gobject-linter --list-rules

# Run only specific rules (overrides config)
gobject-linter --only use_g_strcmp0 --only use_clear_functions

# Add custom ignore patterns
gobject-linter --ignore "build/**" --ignore "tests/**"
```

## Available Rules

Browse all available rules at **https://bilelmoussaoui.github.io/gobject-linter/** with descriptions, examples, and configuration options.

Run `gobject-linter --list-rules` to see the current status of all rules in your terminal.

## Configuration

Create a `gobject-linter.toml` file in your project root to configure rules, set minimum GLib version, and define per-rule ignore patterns.

You can also use inline comments to suppress specific violations:

```c
/* gobject-linter-ignore-next-line: use_g_strlcpy */
strcpy(dst, src);
```

See [CONFIG.md](CONFIG.md) for complete configuration documentation.

## CI/CD Integration

### Container Image

gobject-linter is available as a container image for easy CI/CD integration:

```bash
podman run --rm -v "$PWD:/workspace:Z" ghcr.io/bilelmoussaoui/gobject-linter:latest
```

### GitHub Actions

Using the container image with GitHub Code Scanning:

```yaml
name: GObject Lint

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  lint:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/bilelmoussaoui/gobject-linter:latest
    permissions:
      security-events: write  # Required for uploading SARIF results

    steps:
      - uses: actions/checkout@v4

      - name: Run gobject-linter
        run: gobject-linter --format sarif > gobject-linter.sarif

      - name: Upload SARIF results
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: gobject-linter.sarif
          category: gobject-linter
```

The results will appear in the "Security" tab under "Code scanning alerts" for your repository, and as inline comments on pull requests.

### GitLab CI

Using the container image with GitLab's SARIF ingestion or CodeQuality report:

```yaml
gobject-linter:
  stage: lint
  image:
    name: "ghcr.io/bilelmoussaoui/gobject-linter:latest"
    entrypoint: [""]
  script:
    # Only available in Enterprise Edition
    - gobject-linter --format sarif > gobject-linter.sarif
    # Available in the Community Edition
    - gobject-linter --format gitlab-codequality > gobject-linter-codequality.json
  artifacts:
    expire_in: "1 week"
    reports:
      # Only available in Enterprise Edition
      sarif: gobject-linter.sarif
      # Available in the Community Edition
      codequality: gobject-linter-codequality.json
```

The results will appear in the merge request's security report and as inline comments.

### Diff-scoped reporting

Pass `--diff -` to restrict violations to lines changed in the current pull request.
This is useful for incremental adoption: existing violations in untouched code are silenced,
and only new or modified lines are checked.

**GitHub Actions:**

```yaml
- name: Run gobject-linter (PR changes only)
  if: github.event_name == 'pull_request'
  run: |
    git diff origin/${{ github.base_ref }}...HEAD | gobject-linter --diff -
```

**GitLab CI:**

```yaml
- git diff origin/$CI_MERGE_REQUEST_TARGET_BRANCH_NAME...HEAD | gobject-linter --diff -
```

### Installation Alternative

If you prefer installing locally instead of using containers:

```bash
cargo install --git https://github.com/bilelmoussaoui/gobject-linter gobject-linter
```

## LSP Server

For real-time linting in your editor:

```bash
cargo build --release --bin gobject-linter-lsp
```

**Neovim** (nvim-lspconfig):
```lua
require('lspconfig.configs').gobject_lsp = {
  default_config = {
    cmd = {"gobject-linter-lsp"},
    filetypes = {'c', 'h'},
    root_dir = require('lspconfig.util').root_pattern('gobject-linter.toml', 'goblint.toml', '.git'),
  },
}
require('lspconfig').gobject_lsp.setup{}
```

**VS Code**: Use a generic LSP client extension pointing to `gobject-linter-lsp`

**Helix** (`~/.config/helix/languages.toml`):
```toml
[[language]]
name = "c"
language-servers = ["clangd", "gobject-linter-lsp"]

[language-server.gobject-linter-lsp]
command = "gobject-linter-lsp"
```

## Projects using gobject-linter

- [fwupd](https://github.com/fwupd/fwupd) - A system daemon to allow session software to update firmware ([workflow](https://github.com/fwupd/fwupd/actions/workflows/goblint.yml))
- [xdg-desktop-portal](https://github.com/flatpak/xdg-desktop-portal) - Desktop integration portal ([workflow](https://github.com/flatpak/xdg-desktop-portal/blob/main/.github/workflows/build-and-test.yml#L15))
- [Crosswords](https://gitlab.gnome.org/jrb/crosswords) - A Crossword player and editor for GNOME ([workflow](https://gitlab.gnome.org/jrb/crosswords/-/blob/main/.gitlab-ci.yml?ref_type=heads#L185))

Co-Authored by Claude Code.
