<p align="center">
  <img src="icon/perl-lsp-logo-lockup.svg" alt="perl-lsp" width="400">
</p>

![CI](https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![docs.rs](https://docs.rs/perl-lsp/badge.svg)](https://docs.rs/perl-lsp)
[![codecov](https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/perl-lsp)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![Downloads](https://img.shields.io/crates/d/perl-lsp.svg)](https://crates.io/crates/perl-lsp)

A fast, native **Perl language server** and **parser toolkit** written in Rust — bringing modern IDE features to Perl. The workspace currently contains **over 115 Rust crates** and is in **Public Alpha (v0.11.0)**.

> **Full LSP coverage** · **fast incremental parsing** · **zero runtime Perl dependency**

## Origins

This project was initially forked on July 15th, 2025 (Q3 2025) from [tree-sitter-perl-better](https://github.com/tree-sitter-perl/tree-sitter-perl) (the current official tree-sitter repository). Since then, it has evolved into a native Rust implementation focused on LSP and DAP performance.

## Workspace Snapshot (current)

- **Crate directories**: 121 (under `crates/`)
- **Workspace members**: 116 (`Cargo.toml` workspace members)
- **Crate families**:
  - `perl-module-*`: 13
  - `perl-lsp-*`: 41
  - `perl-lsp-feature-*`: 8
  - `perl-dap-*`: 9
  - `perl-ts-*`: 5
  - `perl-workspace-*`: 6

Regenerate family counts with:

```bash
for prefix in perl-module- perl-lsp- perl-lsp-feature- perl-dap- perl-ts- perl-workspace-; do
  printf "%-18s %s\n" "$prefix" "$(find crates -maxdepth 1 -mindepth 1 -type d -name "${prefix}*" | wc -l)"
done
```

## Features at a Glance

| | Feature | Details |
|---|---------|---------|
| ✅ | **Full LSP Coverage** | 100% advertised coverage (53/53), 100% protocol compliance (97/97) |
| ✅ | **Completion** | Symbols, keywords, modules, variables, snippets |
| ✅ | **Navigation** | Go-to-definition, references, workspace symbols |
| ✅ | **Refactoring** | Rename, code actions, formatting |
| ✅ | **Diagnostics** | Real-time error detection and reporting |
| ✅ | **Hover** | Documentation and type information on hover |
| ✅ | **Debug Adapter** | Breakpoints, stepping, variable inspection via DAP bridge |
| ✅ | **Comprehensive Perl 5 Syntax** | ~100% Perl 5.8-5.40 coverage: heredocs, regex, quotes, formats, all constructs |
| ✅ | **Blazing Fast** | Sub-millisecond incremental parsing (<1ms updates) |
| ✅ | **Zero Perl Dependency** | Pure Rust — no Perl runtime needed for parsing or LSP |
| ✅ | **Cross-File Navigation** | Dual indexing with 98% reference coverage |
| ✅ | **Unicode-Safe** | Full UTF-8/UTF-16 handling with symmetric position conversion |
| ✅ | **Enterprise Security** | Supply chain security (SBOM + SLSA Level 2), path traversal prevention |

## Features

- **Language Server** -- completion, hover, go-to-definition, references, rename, diagnostics, formatting, code actions, document symbols, workspace symbols, and more (**100% advertised coverage**: 53/53 features; `features.toml`)
- **Debug Adapter** -- breakpoints, stepping, variable inspection via DAP bridge to `perl -d`
- **Parser** -- v3 native recursive-descent parser with error recovery, heredoc/regex/quote support, and S-expression output
- **Fast** -- pure Rust, sub-millisecond incremental parsing, no runtime dependencies on Perl for parsing or LSP

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

## Why perl-lsp?

| | perl-lsp | Perl::LanguageServer | PLS |
|---|----------|---------------------|-----|
| **Language** | Rust (native binary) | Perl | Perl |
| **Requires Perl runtime** | No (parsing/LSP) | Yes | Yes |
| **LSP coverage** | 100% (53/53 features) | Partial | Partial |
| **Protocol compliance** | 100% (97/97 methods) | Partial | Partial |
| **Incremental parsing** | Yes (<1ms updates) | N/A | N/A |
| **Debug adapter** | Built-in (DAP bridge) | Built-in | No |
| **Cross-file navigation** | Dual-indexed (98% coverage) | Limited | Limited |
| **Mutation tested** | 87% score | N/A | N/A |
| **Supply chain security** | SBOM + SLSA Level 2 | N/A | N/A |
| **Startup overhead** | Minimal (native) | Perl interpreter | Perl interpreter |

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

### 1. Install

```bash
cargo install perl-lsp
```

### 2. Configure your editor

**VS Code** — Install the extension `effortlesssteven.perl-lsp` from the marketplace. Done!

**Neovim** — Add to your LSP config:

```lua
require('lspconfig').perl_ls.setup {
  cmd = { "perl-lsp", "--stdio" },
}
```

**Emacs** — Add to your init:

```elisp
(add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
```

### 3. Open a Perl file and start coding

You immediately get completions, hover docs, go-to-definition, diagnostics, and more:

```
$ perl-lsp --stdio
# The server communicates via JSON-RPC over stdin/stdout.
# Your editor handles this automatically once configured.
```

### Command-line usage

```bash
# Run the language server (editors connect to this)
perl-lsp --stdio

# Run the debug adapter
perl-dap

# Parse a Perl file directly (library usage)
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

## Architecture

### Parser Evolution

The project maintains three parser versions for different use cases:

| Version | Implementation | Status | Purpose |
|---------|---------------|--------|---------|
| **v1** | tree-sitter | Legacy | C FFI compatibility, benchmarking |
| **v2** | Pest | Legacy | Kept out of default CI gate |
| **v3** | Native recursive descent | **Current** | ~100% Perl 5.8-5.40 coverage, sub-millisecond parsing |

### System Architecture

```
                          ┌──────────────────────┐
                          │    Editor / IDE       │
                          │ (VS Code, Neovim, …)  │
                          └─────────┬────────────┘
                                    │ JSON-RPC (stdio)
                          ┌─────────▼────────────┐
                          │      perl-lsp         │
                          │   (LSP Server)        │
                          │  3-tier profiles:     │
                          │  ga-lock/production/  │
                          │  all                  │
                          └─────────┬────────────┘
                                    │
            ┌───────────────────────┼───────────────────────┐
            │                       │                       │
   ┌────────▼────────┐   ┌─────────▼─────────┐   ┌────────▼────────┐
   │  LSP Providers   │   │  Workspace Index   │   │   perl-dap      │
   │ (41 feature      │   │  (dual-indexed,    │   │  (DAP Bridge)   │
   │  microcrates)    │   │   98% coverage)    │   │  9 microcrates  │
   └────────┬────────┘   └─────────┬─────────┘   └────────┬────────┘
            │                       │                       │
            └───────────────────────┼───────────────────────┘
                                    │
                   ┌────────────────▼────────────────┐
                   │         perl-parser             │
                   │  (v3 recursive-descent,         │
                   │   ~100% Perl 5.8-5.40 syntax)   │
                   └────────────────┬────────────────┘
                                    │
                   ┌────────────────▼────────────────┐
                   │          perl-lexer             │
                   │   (mode-aware tokenizer)        │
                   └─────────────────────────────────┘
```

> **Key Architectural Decisions**: See [Architecture Decision Records](docs/adr/README.md) for design rationale, including [microcrate architecture (ADR-0008)](docs/adr/0008-microcrate-architecture.md), [dual indexing (ADR-0009)](docs/adr/0009-dual-indexing-strategy.md), and [incremental parsing (ADR-0010)](docs/adr/0010-incremental-parsing-architecture.md).

## Workspace Layout

```text
crates/
  perl-lsp/           LSP server binary
  perl-dap/           DAP server binary
  perl-parser/        Parser entry points and high-level APIs
  perl-lexer/         Tokenizer
  perl-lsp-*/         LSP feature crates (completion, diagnostics, navigation, ...)
  perl-module-*/      Module resolution microcrates
  perl-dap-*/         DAP components
  perl-ts-*/          Tree-sitter integration
  perl-workspace-*/   Workspace discovery and indexing
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

# Check local tooling (dev environment doctor)
just doctor

# Full local gate (requires Nix)
nix develop -c just ci-gate
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for our community standards, [SUPPORT.md](SUPPORT.md) for how to get help, and [CLAUDE.md](CLAUDE.md) for the full command reference.

## Documentation

### User & Developer Guides

- [Book](book/) -- comprehensive user and developer guide (mdbook)
- [docs/](docs/README.md) -- documentation index
- [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) -- server architecture
- [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) -- debugger setup and usage
- [Stability Policy](docs/reference/STABILITY.md) -- API versioning and compatibility
- [features.toml](features.toml) -- canonical LSP feature catalog

### Strategic Documentation

For project direction, planning, and architectural decisions:

| Document | Purpose |
|----------|---------|
| [**ROADMAP.md**](ROADMAP.md) | Version milestones and deliverables (v0.10→v1.0+) |
| [**NOW_NEXT_LATER.md**](NOW_NEXT_LATER.md) | Current quarter priorities |
| [**TECHNICAL_VISION.md**](TECHNICAL_VISION.md) | Long-term technical direction (3-5 years) |
| [**Strategic Documentation Index**](docs/STRATEGIC_DOCUMENTATION.md) | Navigation hub for all strategic docs |
| [**Architecture Decision Records**](docs/adr/README.md) | Key design decisions and rationale |

### Key Architecture Decision Records

| ADR | Title | Description |
|-----|-------|-------------|
| [ADR-0008](docs/adr/0008-microcrate-architecture.md) | Microcrate Architecture | 115+ small crates following SRP |
| [ADR-0009](docs/adr/0009-dual-indexing-strategy.md) | Dual Indexing | 98% reference coverage |
| [ADR-0010](docs/adr/0010-incremental-parsing-architecture.md) | Incremental Parsing | <1ms update target |
| [ADR-0015](docs/adr/0015-supply-chain-security.md) | Supply Chain Security | SBOM + SLSA Level 2 |
| [ADR-0019](docs/adr/0019-security-first-dap.md) | Security-First DAP | Enterprise-grade debugger security |

## License

Dual licensed under MIT OR Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
