# perl-lsp

[![Crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![codecov](https://codecov.io/gh/EffortlessMetrics/tree-sitter-perl-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/tree-sitter-perl-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

A fast, native Perl language server and parser toolkit written in Rust.

Provides LSP support for editors (VS Code, Neovim, Emacs, etc.), a Debug Adapter Protocol server, and a standalone Perl parser library.

## Features

- **Language Server** -- completion, hover, go-to-definition, references, rename, diagnostics, formatting, code actions, document symbols, workspace symbols, and more (97 LSP features)
- **Debug Adapter** -- breakpoints, stepping, variable inspection via DAP bridge to `perl -d`
- **Parser** -- recursive-descent Perl parser with error recovery, heredoc/regex/quote support, and S-expression output
- **Fast** -- pure Rust, no runtime dependencies on Perl for parsing or LSP

## Install

### From crates.io

```bash
cargo install perl-lsp
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases), or use the installer script:

```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/master/install.sh | bash
```

## Editor Setup

### VS Code

Install the [perl-lsp extension](vscode-extension/) from the included VS Code extension source, or point any LSP-compatible extension at the `perl-lsp` binary.

### Neovim (nvim-lspconfig)

```lua
require('lspconfig').perl_ls.setup {
  cmd = { "perl-lsp", "--stdio" },
}
```

### Emacs (lsp-mode / eglot)

```elisp
;; eglot
(add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
```

## Quick Start

```bash
# Run the language server
perl-lsp --stdio

# Run the debug adapter
perl-dap

# Parse a Perl file (library usage)
cargo run -p perl-parser -- path/to/file.pl
```

## Published Crates

| Crate | Purpose |
|-------|---------|
| [`perl-lsp`](https://crates.io/crates/perl-lsp) | Language Server Protocol binary |
| [`perl-dap`](crates/perl-dap/) | Debug Adapter Protocol binary |
| [`perl-parser`](https://crates.io/crates/perl-parser) | Recursive-descent Perl parser library |
| [`perl-lexer`](https://crates.io/crates/perl-lexer) | Context-aware Perl tokenizer |
| [`perl-corpus`](https://crates.io/crates/perl-corpus) | Parser/LSP test corpus |

## Workspace Layout

```text
crates/
  perl-lsp/           LSP server binary
  perl-dap/           DAP server binary
  perl-parser/        Parser entry points and high-level APIs
  perl-lexer/         Tokenizer
  perl-lsp-*/         LSP feature crates (completion, diagnostics, navigation, ...)
  perl-*/             Parser support crates (ast, token, quote, regex, heredoc, ...)
xtask/                Development automation
book/                 mdbook documentation
vscode-extension/     VS Code extension source
```

## Development

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace

# Lint + format
cargo clippy --workspace --lib && cargo fmt --all

# Full local gate (requires Nix)
nix develop -c just ci-gate
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and [CLAUDE.md](CLAUDE.md) for the full command reference.

## Documentation

- [Book](book/) -- comprehensive user and developer guide (mdbook)
- [docs/](docs/README.md) -- reference documentation index
- [LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md) -- server architecture
- [DAP User Guide](docs/DAP_USER_GUIDE.md) -- debugger setup and usage
- [Stability Policy](docs/STABILITY.md) -- API versioning and compatibility
- [features.toml](features.toml) -- canonical LSP feature catalog

## License

Dual licensed under MIT OR Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
