<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  A fast, native <strong>Perl language server</strong> and parser toolkit written in Rust for modern Perl 5 development.
</p>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://crates.io/crates/perl-lsp"><img src="https://img.shields.io/crates/v/perl-lsp.svg" alt="crates.io" /></a>
  <a href="https://docs.rs/perl-lsp"><img src="https://docs.rs/perl-lsp/badge.svg" alt="docs.rs" /></a>
  <a href="https://codecov.io/gh/EffortlessMetrics/perl-lsp"><img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="codecov" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.92%2B-orange.svg" alt="Rust" /></a>
  <a href="https://crates.io/crates/perl-lsp"><img src="https://img.shields.io/crates/d/perl-lsp.svg" alt="Downloads" /></a>
</p>

> **Status:** `perl-lsp` v0.10.0 is the initial public alpha. Core editor workflows are in place, but interfaces and behavior may still evolve. Please [report issues](https://github.com/EffortlessMetrics/perl-lsp/issues) when you hit rough edges.

## What this repository contains

This repository is more than a single binary crate. It is a Rust workspace for Perl tooling with:

- **`perl-lsp`** — the Language Server Protocol binary for editor integration.
- **`perl-dap`** — a Debug Adapter Protocol bridge for Perl debugging workflows.
- **`perl-parser`** — the native recursive-descent Perl 5 parser.
- **`perl-lexer`** — the context-aware tokenizer that feeds the parser stack.
- **120+ focused crates** — workspace indexing, semantic analysis, formatting, diagnostics, module resolution, and supporting infrastructure.

If you want the user-facing setup guide first, jump to **[Quick start](#quick-start)**. If you want implementation details, jump to **[Architecture](#architecture)** or **[Documentation](#documentation)**.

## Why perl-lsp?

`perl-lsp` is designed for developers who want modern IDE ergonomics for Perl without depending on a Perl-based language server stack.

### Highlights

- **Native Rust binaries** for the language server, parser, and tooling.
- **No Perl runtime required** for parsing or core LSP behavior.
- **100% advertised LSP coverage** with `53/53` user-visible capabilities implemented.
- **100% tracked protocol compliance** with `97/97` features covered in the feature catalog.
- **Fast incremental parsing** tuned for interactive editor updates.
- **Cross-file navigation** through dual-indexed workspace symbol lookup.
- **Unicode-safe position handling** with UTF-8 and UTF-16 symmetry.
- **Supply-chain security features** including SBOM generation and SLSA Level 2 provenance for releases.

### Comparison at a glance

| | perl-lsp | Perl::LanguageServer | PLS |
|---|----------|---------------------|-----|
| **Implementation language** | Rust | Perl | Perl |
| **Requires Perl runtime** | No | Yes | Yes |
| **Advertised LSP coverage** | 53/53 | Partial | Partial |
| **Incremental parsing** | Yes | N/A | N/A |
| **Debug adapter** | Built-in bridge | Built-in | No |
| **Supply-chain attestations** | Yes | N/A | N/A |

## Quick start

### Install from crates.io

```bash
cargo install perl-lsp
perl-lsp --health
```

Expected output:

```text
ok 0.10.0
```

### Install from source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health
```

### Install a pre-built binary

Download the appropriate artifact from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases), place `perl-lsp` on your `PATH`, then run:

```bash
perl-lsp --health
```

### Connect your editor

- **VS Code**

  ```bash
  code --install-extension effortlessmetrics.perl-lsp-rs
  ```

- **Neovim** (`nvim-lspconfig`)

  ```lua
  require('lspconfig').perl_ls.setup {
    cmd = { "perl-lsp", "--stdio" },
  }
  ```

- **Emacs** (`eglot`)

  ```elisp
  (add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
  ```

Open a Perl file after installation and you should immediately get completions, diagnostics, hover information, and navigation.

For full walkthroughs, troubleshooting, and more editors, see:

- [Getting Started](docs/tutorials/GETTING_STARTED.md)
- [VS Code setup](docs/EDITORS/VS_CODE_SETUP.md)
- [Neovim setup](docs/EDITORS/NEOVIM_SETUP.md)
- [Emacs setup](docs/EDITORS/EMACS_SETUP.md)
- [Helix setup](docs/EDITORS/HELIX_SETUP.md)
- [coc.nvim setup](docs/EDITORS/COC_NEOVIM_SETUP.md)
- [Sublime Text setup](docs/EDITORS/SUBLIME_SETUP.md)

## Features

| Area | Included capabilities |
|------|------------------------|
| **Completion** | Symbols, keywords, modules, variables, snippets, built-in function signatures |
| **Navigation** | Go to definition, find references, workspace symbols, document symbols |
| **Hover** | Documentation, symbol details, and related type information |
| **Diagnostics** | Real-time parser and language diagnostics |
| **Refactoring** | Rename, code actions, formatting, import management |
| **Debugging** | Breakpoints, stepping, and variable inspection through the DAP bridge |
| **Performance** | Incremental parsing and native binaries designed for low-latency editor workflows |
| **Workspace intelligence** | Dual-indexed symbol lookup and cross-file navigation |
| **Encoding safety** | UTF-8 and UTF-16 aware position conversion |

The canonical capability inventory lives in [`features.toml`](features.toml). Current health and computed project metrics are published in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Parser coverage

The `perl-parser` v3 engine is a native recursive-descent parser that targets broad Perl 5 support across versions 5.8 through 5.40, including:

- heredocs
- regex and substitution operators
- quote-like operators
- formats
- package and symbol constructs
- modern Perl syntax used in real-world codebases

Coverage is improved through a combination of curated fixtures, system Perl sweeps, and ratcheting CI checks:

- **Corpus test suite** — `tree-sitter-perl/test/corpus/` plus standalone fixture programs.
- **System Perl sweep** — runs across installed `.pm` and `.pl` files to measure parse health on real code.
- **Pinned common-files gate** — ensures a curated module set continues to parse cleanly.
- **Ratcheting baselines** — regressions fail CI instead of silently lowering parse quality.
- **CPAN corpus workflow** — provides a path to measuring parser success against widely used distributions.

### Corpus commands

```bash
just corpus-sweep
just corpus-sweep-check
just corpus-sweep-update
just common-corpus-check
just cpan-corpus-fetch
just cpan-corpus-install
just cpan-corpus-baseline-update
just cpan-corpus-check
```

See [PARSER_EDGE_CASE_ROADMAP.md](docs/project/PARSER_EDGE_CASE_ROADMAP.md) and [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for parse-rate reporting and open edge cases.

## Architecture

```text
Editor / IDE
    |  JSON-RPC (stdio)
    v
perl-lsp  (LSP server)
    |
    +-- LSP providers (completion, hover, diagnostics, ...)
    +-- Workspace index (dual-indexed symbol lookup)
    +-- perl-dap (Debug Adapter Protocol bridge)
    |
perl-parser  (v3 recursive-descent)
    |
perl-lexer   (mode-aware tokenizer)
```

### Workspace shape

The repository is organized as a large Rust workspace with 121 crates at the current `HEAD`. Crates are intentionally small and focused so parsing, LSP features, DAP support, indexing, and utility layers can evolve independently.

Representative crate families include:

| Family | Count | Purpose |
|--------|------:|---------|
| `perl-module-*` | 13 | Module resolution and module-path handling |
| `perl-lsp-*` | 41 | LSP protocol support, providers, helpers, and feature modules |
| `perl-lsp-feature-*` | 8 | Feature governance and policy layers |
| `perl-dap-*` | 9 | Debug adapter components |
| `perl-ts-*` | 5 | Tree-sitter and parser-adjacent support |
| `perl-workspace-*` | 6 | Workspace discovery, indexing, and state management |

For deeper design rationale, see:

- [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)
- [Architecture Decision Records](docs/adr/README.md)
- [Workspace architecture notes](docs/project/WORKSPACE_ARCHITECTURE.md)

## Published crates

| Crate | Purpose |
|-------|---------|
| [`perl-lsp`](https://crates.io/crates/perl-lsp) | Language Server Protocol binary |
| [`perl-dap`](https://crates.io/crates/perl-dap) | Debug Adapter Protocol binary |
| [`perl-parser`](https://crates.io/crates/perl-parser) | Recursive-descent Perl parser library |
| [`perl-lexer`](https://crates.io/crates/perl-lexer) | Context-aware Perl tokenizer |
| [`perl-corpus`](https://crates.io/crates/perl-corpus) | Parser and LSP test corpus |

## Development

### Common commands

```bash
cargo build --workspace
cargo test --workspace --lib
cargo clippy --workspace --lib
cargo fmt --all
nix develop -c just ci-gate
```

### Recommended local workflow

1. Make your changes.
2. Run `cargo fmt --all`.
3. Run `cargo clippy --workspace --lib`.
4. Run targeted tests, or `cargo test --workspace --lib` if you need a broad check.
5. Before pushing, run `nix develop -c just ci-gate`.

Additional contributor guidance lives in [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Release artifacts include:

- **SBOM generation** in SPDX and CycloneDX formats.
- **SLSA Level 2 provenance attestations** for release verification.
- **CI ratchets** that enforce `unsafe`-free production code and prohibit `unwrap`, `expect`, and panic-family macros in production paths.
- **Hardening** against path traversal, command injection, and encoding boundary issues.

See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md) for implementation details.

## Documentation

| Resource | Description |
|----------|-------------|
| [docs/README.md](docs/README.md) | Documentation index |
| [Getting Started](docs/tutorials/GETTING_STARTED.md) | Installation, editor setup, and first-run walkthrough |
| [Book](book/README.md) | User and developer guide |
| [features.toml](features.toml) | Canonical LSP feature catalog |
| [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) | Computed project metrics |
| [ROADMAP.md](ROADMAP.md) | Version milestones and planning |
| [Stability Policy](docs/reference/STABILITY.md) | API versioning and compatibility |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Debugger setup and usage |

## History

This project began as a fork of [tree-sitter-perl](https://github.com/tree-sitter-perl/tree-sitter-perl) in July 2025. It has since evolved into a native Rust recursive-descent parser and a broader LSP/DAP toolkit for Perl development.

## License

Dual licensed under MIT or Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
