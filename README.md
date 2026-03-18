<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  A fast, native <strong>Perl language server</strong>, parser, and debugging toolkit written in Rust.
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

> **Status:** Public alpha (`v0.10.0`). Core features are already useful today, but the project is still moving quickly. Please [report issues](https://github.com/EffortlessMetrics/perl-lsp/issues) and rough edges.

## What this repository contains

This repository is more than a single binary crate. It is a Rust workspace that currently contains **121 crates** covering:

- **`perl-lsp`** — the Language Server Protocol server for editors.
- **`perl-dap`** — a Debug Adapter Protocol server for interactive debugging.
- **`perl-parser`** — the native recursive-descent Perl parser.
- **`perl-lexer`** — the mode-aware tokenizer that powers parsing.
- **LSP provider crates** — completion, navigation, diagnostics, formatting, semantic tokens, code actions, and more.
- **Workspace and symbol infrastructure** — indexing, URI handling, file safety, and cross-file analysis.

If you just want to **use** the editor integration, install `perl-lsp`. If you want to **hack on parser or editor infrastructure**, this workspace has the full stack.

## Why perl-lsp?

- **Native Rust implementation** — no Perl runtime required for parsing or core LSP features.
- **Modern editor support** — completion, hover, rename, code actions, formatting, diagnostics, semantic tokens, and symbol search.
- **Cross-file navigation** — dual indexing improves references and workspace discovery.
- **Fast incremental parsing** — optimized for interactive IDE workloads.
- **Integrated debugging** — `perl-dap` provides DAP support alongside the LSP stack.
- **Security-focused engineering** — path-safety checks, hardened input handling, SBOM generation, and provenance attestation.

## Quick start

### Install from crates.io

```bash
cargo install perl-lsp
perl-lsp --health
```

Expected output:

```text
ok X.Y.Z
```

### Install from source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health
```

### Prebuilt binaries

Download release artifacts from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

## Editor setup

### VS Code

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

Open any `.pl`, `.pm`, or related Perl file and the extension will launch `perl-lsp` automatically.

### Neovim (`nvim-lspconfig`)

```lua
require('lspconfig').perl_ls.setup {
  cmd = { 'perl-lsp', '--stdio' },
}
```

### Emacs (`eglot`)

```elisp
(add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
```

New to language servers? Start with the **[Getting Started guide](docs/tutorials/GETTING_STARTED.md)** for editor-specific setup, screenshots, and troubleshooting.

## Core commands

```bash
perl-lsp --stdio      # editor integration over stdio
perl-lsp --health     # installation sanity check
perl-lsp --version    # version output
perl-dap              # launch the debug adapter
cargo build -p perl-parser --release
cargo test --workspace --lib
```

## Feature overview

| Area | Highlights |
|------|------------|
| **Editing** | Completion, hover, signature help, formatting, on-type formatting, inlay hints |
| **Navigation** | Go to definition, find references, document symbols, workspace symbols |
| **Refactoring** | Rename and code actions |
| **Diagnostics** | Parser-driven feedback plus LSP diagnostics plumbing |
| **Debugging** | Breakpoints, stepping, variable inspection through `perl-dap` |
| **Performance** | Incremental parsing and native binaries optimized for IDE feedback loops |
| **Correctness & safety** | Unicode-safe UTF-8/UTF-16 conversion, path traversal prevention, hardened file handling |

The workspace tracks **53/53 advertised user-visible LSP capabilities** in [`features.toml`](features.toml). For computed project metrics, see [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Workspace architecture

```text
Editor / IDE
    |  JSON-RPC (stdio)
    v
perl-lsp
    |
    +-- LSP provider crates
    +-- Workspace indexing and symbol services
    +-- perl-dap (debug adapter bridge)
    |
perl-parser
    |
perl-lexer
```

### Major crate families

| Family | Count | Role |
|--------|------:|------|
| `perl-lsp-*` | 41 | LSP protocol, providers, feature governance, utilities |
| `perl-module-*` | 13 | Module resolution, naming, imports, and refactoring helpers |
| `perl-dap-*` | 9 | Debug adapter components |
| `perl-lsp-feature-*` | 8 | Feature-flag and governance infrastructure |
| `perl-workspace-*` | 6 | Workspace discovery and indexing |
| `perl-ts-*` | 5 | Tree-sitter interoperability and analysis tooling |

### Important crates

| Crate | Purpose |
|-------|---------|
| [`crates/perl-lsp`](crates/perl-lsp/) | Top-level LSP server binary |
| [`crates/perl-dap`](crates/perl-dap/) | Top-level DAP server binary |
| [`crates/perl-parser`](crates/perl-parser/) | Recursive-descent Perl parser |
| [`crates/perl-lexer`](crates/perl-lexer/) | Context-aware tokenizer |
| [`crates/perl-semantic-analyzer`](crates/perl-semantic-analyzer/) | Semantic analysis infrastructure |
| [`crates/perl-corpus`](crates/perl-corpus/) | Shared parser and LSP test corpus |

## Parser coverage and quality gates

The current parser is the **v3 native recursive-descent parser**. It targets broad Perl 5 syntax coverage across modern language constructs, including heredocs, regexes, quoting operators, substitution operators, and formats.

Coverage is enforced with multiple feedback loops:

- **Corpus tests** covering dedicated grammar scenarios and standalone fixtures.
- **System Perl corpus sweeps** against `.pl` and `.pm` files from installed Perl distributions.
- **Pinned common-file gates** that must stay clean on every pull request.
- **Ratcheting CI checks** that reject parser regressions.
- **CPAN corpus tooling** for measuring real-world package compatibility over time.

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

For roadmap details and current parse-rate tracking, see [PARSER_EDGE_CASE_ROADMAP.md](docs/project/PARSER_EDGE_CASE_ROADMAP.md) and [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Development

### Day-to-day commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --lib
cargo fmt --all
nix develop -c just ci-gate
```

### Development guidelines

- Prefer the crate closest to the change you are making: parser work in `crates/perl-parser`, LSP entrypoints in `crates/perl-lsp`, DAP work in `crates/perl-dap`, and provider logic in `crates/perl-lsp-*` crates.
- Production Rust code in this workspace avoids `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()`.
- Tests should prefer `Result<()>`-style flows and the project helpers where applicable.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contributor workflow details.

## Documentation map

| Resource | Description |
|----------|-------------|
| **[Getting Started](docs/tutorials/GETTING_STARTED.md)** | Installation, editor setup, and first-run walkthrough |
| [docs/README.md](docs/README.md) | Documentation index |
| [book/](book/) | User and developer book |
| [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | Server architecture and provider structure |
| [Current Status](docs/project/CURRENT_STATUS.md) | Computed metrics and health indicators |
| [Stability Policy](docs/reference/STABILITY.md) | API compatibility expectations |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Debugger setup and usage |
| [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md) | SBOMs, attestations, and release hardening |
| [ROADMAP.md](ROADMAP.md) | Version milestones and planning |

## Security

Release artifacts include SBOM generation in SPDX and CycloneDX formats plus **SLSA Level 2 provenance attestations**. The codebase also enforces strict safety and reliability guardrails, including a ban on `unsafe` code and panic-style production constructs in CI.

See [docs/reference/SUPPLY_CHAIN_SECURITY.md](docs/reference/SUPPLY_CHAIN_SECURITY.md) for the full policy and implementation details.

## Project history

This project started as a fork of [tree-sitter-perl](https://github.com/tree-sitter-perl/tree-sitter-perl) in July 2025. It has since evolved into a native Rust parser and a full LSP/DAP workspace for Perl development tooling.

## License

Dual licensed under either of the following, at your option:

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
