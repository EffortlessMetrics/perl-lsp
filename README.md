# perl-lsp

![CI](https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![codecov](https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/perl-lsp)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)

A fast, native Perl language server and parser toolkit written in Rust. Currently in **Initial Public Alpha (v0.10.0)**.

## Origins

This project started in Q2 2025. It was initially forked on July 15th, 2025 from [tree-sitter-perl-better](https://github.com/tree-sitter-perl/tree-sitter-perl) (the current official tree-sitter repository). Since then, it has evolved into a native Rust implementation focused on LSP and DAP performance.

## Features

- **Language Server** -- completion, hover, go-to-definition, references, rename, diagnostics, formatting, code actions, document symbols, workspace symbols, and more (**100% advertised user-visible coverage**; `53/53` user-visible and `97/97` protocol methods; `features.toml`)
- **Debug Adapter** -- breakpoints, stepping, variable inspection via DAP bridge to `perl -d`
- **Parser** -- recursive-descent Perl parser with error recovery, heredoc/regex/quote support, and S-expression output
- **Fast** -- pure Rust, no runtime dependencies on Perl for parsing or LSP

## Install

### From crates.io

```bash
cargo install perl-lsp
```

### From source (default)

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases), or use the installer scripts (best-effort / non-canonical):

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.ps1 | iex
```

### Homebrew (macOS / Linux)

```bash
brew install perl-lsp
```

### Scoop (Windows)

```powershell
scoop install perl-lsp
```

### Chocolatey (Windows)

```powershell
choco install perl-lsp
```

## Editor Setup

### VS Code

Install the **Perl Language Server** extension (`effortlesssteven.perl-lsp`) from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp), or build from [source](vscode-extension/).

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

## Access Paths

- **Rust crates on crates.io**: install binaries with `cargo install perl-lsp` / `cargo install perl-dap`, and consume libraries with `cargo add perl-parser` / `cargo add perl-lexer`.
- **Editor integrations**: the VS Code extension is the primary packaged integration; any LSP-compatible editor can connect to `perl-lsp --stdio` (Neovim, Emacs, etc.).
- **Debug adapter**: `perl-dap` for VS Code and other DAP-compatible editors.
- **FFI / non-Rust integration**: the tree-sitter workspace exposes optional C interoperability through `tree-sitter-perl-rs` (`c-parser` feature) for existing C/FFI consumers.

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
| [`perl-dap`](https://crates.io/crates/perl-dap) | Debug Adapter Protocol binary |
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
  perl-lsp-*/         LSP feature crates (21 crates: completion, diagnostics, navigation, ...)
  perl-module-*/      Module resolution microcrates (13 crates)
  perl-dap-*/         DAP components (4 crates: breakpoint, eval, stack, variables)
  perl-ts-*/          Tree-sitter integration (5 crates)
  perl-workspace-*/   Workspace discovery and indexing (4 crates)
  perl-*/             Core support crates (ast, token, quote, regex, heredoc, error, ...)
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

# Find SRP-focused microcrates in the workspace
scripts/find_srp_microcrates.sh
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for our community standards, [SUPPORT.md](SUPPORT.md) for how to get help, and [CLAUDE.md](CLAUDE.md) for the full command reference.

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
