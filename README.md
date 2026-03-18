<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  A fast, native <strong>Perl language server</strong> written in Rust — bringing modern IDE features to Perl 5.
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

---

> **Note**: perl-lsp is in public alpha. Core features work well, but expect some rough edges. [Report issues](https://github.com/EffortlessMetrics/perl-lsp/issues).

## Quick Start

### 1) Install the language server

```bash
cargo install perl-lsp
perl-lsp --health  # should print: ok X.Y.Z
```

### 2) Hook it up to your editor

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

### 3) Open a Perl file

You should immediately get completions, hover, diagnostics, and navigation.

New to language servers? Start with the **[Getting Started guide](docs/tutorials/GETTING_STARTED.md)** for a full walkthrough, editor-specific setup notes, and troubleshooting tips.

## At a Glance

- **Native Rust binaries** for both LSP (`perl-lsp`) and debugging (`perl-dap`)
- **Modern editor support** with completion, diagnostics, hover, rename, formatting, and symbols
- **Deep Perl 5 coverage** across parser, lexer, semantic analysis, and workspace indexing
- **Cross-file navigation** powered by dual indexing of bare and qualified symbols
- **Unicode-safe and security-focused** implementation with hardened path and protocol handling
- **Monorepo architecture** with 100+ focused crates for parser, LSP, DAP, and tooling

## Features

| Area | What you get |
|------|---------------|
| **Completion** | Symbols, keywords, modules, variables, snippets, and built-in function signatures |
| **Navigation** | Go-to-definition, find references, workspace symbols, and document symbols |
| **Hover** | Inline documentation and semantic context |
| **Diagnostics** | Real-time syntax and analysis feedback while you type |
| **Refactoring** | Rename, code actions, formatting, and on-type formatting hooks |
| **Debugging** | Breakpoints, stepping, and variable inspection through the DAP bridge |
| **Performance** | Native Rust binaries with sub-millisecond incremental parsing updates |
| **Workspace intelligence** | Cross-file navigation via a dual-indexed symbol model |
| **Portability** | No Perl runtime required for parsing or for the LSP server itself |
| **Safety** | UTF-8/UTF-16-safe offsets plus hardened path and protocol handling |

Full LSP coverage: all 53 advertised capabilities are implemented in the public feature catalog. See [`features.toml`](features.toml) and [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for the current metrics and compatibility snapshot.

## Why perl-lsp?

| | perl-lsp | Perl::LanguageServer | PLS |
|---|----------|---------------------|-----|
| **Language** | Rust (native binary) | Perl | Perl |
| **Requires Perl runtime** | No | Yes | Yes |
| **LSP feature coverage** | 53/53 advertised | Partial | Partial |
| **Incremental parsing** | Yes (sub-ms updates) | N/A | N/A |
| **Debug adapter** | Built-in (DAP bridge) | Built-in | No |
| **Supply chain security** | SBOM + SLSA Level 2 | N/A | N/A |

## Parser Coverage

The v3 parser is a native recursive-descent implementation covering broad Perl 5 syntax
(5.8 through 5.40), including heredocs, regex, quoting constructs, formats, and more.
It is tested continuously against real-world Perl code to drive coverage improvements:

- **Corpus test suite** -- 600+ test sections in `tree-sitter-perl/test/corpus/` plus 70+ standalone `.pl` fixtures.
- **System Perl corpus sweep** -- benchmarked against all `.pm` and `.pl` files found in the system Perl installation. Current parse rates are tracked in [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) and the baseline file (`.ci/parser-corpus-baseline.json`).
- **Common-files gate** -- a curated set of core modules that must parse with zero errors on every PR (`.ci/common-corpus-manifest.txt`).
- **Ratcheting CI gate** -- the overall parse rate can only go up, never down. Regressions fail the build.
- **CPAN top 1000 goal** -- the long-term target is for 90%+ of the most-downloaded CPAN distributions to parse cleanly, driving parser improvements toward real-world Perl idioms.

For detailed parse rates and the edge-case roadmap, see [PARSER_EDGE_CASE_ROADMAP.md](docs/project/PARSER_EDGE_CASE_ROADMAP.md).

### Corpus Commands

```bash
just corpus-sweep          # Sweep system Perl corpus and print results
just corpus-sweep-check    # Check against baseline (fails on regression)
just corpus-sweep-update   # Update baseline with current results
just common-corpus-check   # Check pinned modules parse cleanly (PR gate)
just cpan-corpus-fetch     # Fetch CPAN top-1000 distribution list
just cpan-corpus-install   # Install CPAN corpus locally; auto-fetches the list, bootstraps cpanm, and reuses a local cache
just cpan-corpus-baseline-update  # Seed/update CPAN ratchet baseline
just cpan-corpus-check     # Check CPAN baseline + known-clean manifest
```

See [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for the latest computed metrics.

## Install

Choose the path that matches how you want to consume the project.

### From crates.io

Best when you want the published server quickly:

```bash
cargo install perl-lsp
perl-lsp --health  # should print: ok X.Y.Z
```

### From source

Best when you want the latest workspace code or plan to contribute:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health  # should print: ok X.Y.Z
```

### Pre-built binaries

Best when you do not want a local Rust toolchain. Download artifacts from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

## Architecture

```text
Editor / IDE
    |  JSON-RPC (stdio)
    v
perl-lsp  (LSP server)
    |
    +-- LSP Providers (completion, hover, diagnostics, rename, formatting, ...)
    +-- Workspace Index (dual-indexed symbol lookup)
    +-- perl-dap (Debug Adapter Protocol bridge)
    |
perl-parser  (v3 recursive-descent)
    |
perl-lexer   (mode-aware tokenizer)
```

This repository is a large Rust workspace with 100+ small, focused crates. The structure keeps responsibilities narrow: parser crates stay parser-focused, LSP crates stay feature-focused, and shared utilities remain reusable across the ecosystem.

### Core published crates

| Crate | Purpose |
|-------|---------|
| [`perl-lsp`](https://crates.io/crates/perl-lsp) | Language Server Protocol binary |
| [`perl-dap`](https://crates.io/crates/perl-dap) | Debug Adapter Protocol binary |
| [`perl-parser`](https://crates.io/crates/perl-parser) | Recursive-descent Perl parser library |
| [`perl-lexer`](https://crates.io/crates/perl-lexer) | Context-aware Perl tokenizer |
| [`perl-corpus`](https://crates.io/crates/perl-corpus) | Parser and LSP test corpus |

For the full tier system, architecture decision records, and design rationale, see:

- [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)
- [Architecture Decision Records](docs/adr/README.md)
- [Crate Architecture Guide](docs/reference/CRATE_ARCHITECTURE_GUIDE.md)

## Development

### Common commands

```bash
cargo build --workspace            # Build everything
cargo test --workspace             # Run all tests
cargo clippy --workspace --lib     # Lint library targets
cargo fmt --all                    # Format code
nix develop -c just ci-gate        # Full local gate (required before push)
```

### Where to work

- Parser changes: `crates/perl-parser/`
- LSP binary and CLI: `crates/perl-lsp/`
- LSP feature providers: `crates/perl-lsp-*/`
- DAP implementation: `crates/perl-dap/`
- Workspace indexing and module resolution: `crates/perl-workspace-*/`, `crates/perl-module-*/`

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributor workflows, [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards, and [SUPPORT.md](SUPPORT.md) for support channels.

## Security

Release artifacts include SBOM generation (SPDX and CycloneDX) and SLSA Level 2
provenance attestations. Production code enforces zero `unsafe`, zero `unwrap`/`expect`,
and zero `panic!`-family macros via CI ratchets. Path traversal and command injection
vectors are hardened.

See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md) for details.

## Documentation

| Resource | Description |
|----------|-------------|
| **[Getting Started](docs/tutorials/GETTING_STARTED.md)** | **Installation, editor setup, and first-run walkthrough** |
| [Book](book/) | Comprehensive user and developer guide |
| [docs/](docs/README.md) | Documentation index |
| [features.toml](features.toml) | Canonical LSP feature catalog |
| [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) | Live project metrics |
| [ROADMAP.md](ROADMAP.md) | Version milestones and planning |
| [Stability Policy](docs/reference/STABILITY.md) | API versioning and compatibility |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Debugger setup and usage |

## History

This project began as a fork of [tree-sitter-perl](https://github.com/tree-sitter-perl/tree-sitter-perl) in July 2025. It has since been rewritten as a native Rust recursive-descent parser and grown into a full-featured LSP/DAP toolkit with over 115 crates.

## License

Dual licensed under MIT or Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
