<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://crates.io/crates/perl-lsp-rs"><img src="https://img.shields.io/crates/v/perl-lsp-rs.svg" alt="crates.io" /></a>
  <a href="https://crates.io/crates/perl-lsp-rs"><img src="https://img.shields.io/crates/d/perl-lsp-rs.svg" alt="Downloads" /></a>
  <a href="https://docs.rs/perl-lsp-rs"><img src="https://docs.rs/perl-lsp-rs/badge.svg" alt="docs.rs" /></a>
  <a href="https://github.com/EffortlessMetrics/perl-lsp/releases"><img src="https://img.shields.io/github/v/release/EffortlessMetrics/perl-lsp?display_name=tag" alt="GitHub release" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/MSRV-1.92-blue" alt="MSRV" /></a>
  <a href="https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"><img src="https://img.shields.io/visual-studio-marketplace/v/EffortlessMetrics.perl-lsp-rs" alt="VSCode Marketplace" /></a>
  <a href="https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs"><img src="https://img.shields.io/open-vsx/v/EffortlessMetrics/perl-lsp-rs" alt="Open VSX" /></a>
</p>

---

Perl has lacked a proper modern LSP implementation. Other languages — Rust, TypeScript, Go, Python — have mature language servers with fast completions, reliable navigation, and full debugger integration. Perl's existing options were slow, incomplete, or required a working Perl runtime just to get basic editor features. `perl-lsp` fills that gap: a native Rust implementation of the Language Server Protocol and Debug Adapter Protocol for Perl 5, with its own parser and lexer, no Perl runtime required for IDE features.

## What It Is

`perl-lsp` is a workspace of Rust crates delivering a complete Perl 5 tooling stack: an LSP server (`perllsp`) covering 102 catalogued capabilities (87 LSP + 10 DAP + 5 extension features), a DAP debug adapter, a recursive-descent parser, a context-aware lexer, and a semantic analyzer — packaged as a single native binary you can drop into any editor. It runs on Windows, macOS, and Linux.

## Quick Start

**VS Code** — install the extension and you are done:

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

The extension auto-downloads the matching `perllsp` binary for your platform.

**Other editors** — download a prebuilt binary from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases), add it to your `PATH`, then point your LSP client at it:

```lua
-- Neovim (nvim-lspconfig)
require('lspconfig').perl_ls.setup { cmd = { "perllsp", "--stdio" } }
```

```elisp
;; Emacs (eglot)
(add-to-list 'eglot-server-programs
             '((perl-mode cperl-mode) . ("perllsp" "--stdio")))
```

```text
# Any generic LSP client
perllsp --stdio
```

Verify the install:

```bash
perllsp --health
```

For a full walkthrough, see [docs/tutorials/GETTING_STARTED.md](docs/tutorials/GETTING_STARTED.md).

> **Note:** Do not use `cargo install perl-lsp` — that name is owned by an unrelated project on crates.io. Use `cargo install --path crates/perllsp` to build from source.

## Key Features

- **Full LSP coverage** — completions, diagnostics, hover, go-to-definition, find references, rename, formatting, semantic tokens, inlay hints, code actions, code lens, workspace symbols — 102 catalogued capabilities at 100% coverage
- **Native debug adapter** — DAP breakpoints, stepping, stack frames, variable inspection, and evaluate; no wrapper script required
- **Fast native parser** — recursive-descent v3 parser with a context-aware lexer; validated against a curated CPAN corpus
- **Semantic analysis** — symbol resolution, scope tracking, Moose/Moo method modifiers and role composition
- **Refactoring** — extract variable, extract subroutine, workspace-scoped rename, subroutine inlining
- **Diagnostics** — dead code highlighting, strict/warnings diagnostics, perlcritic integration with walk-up discovery
- **Zero-Perl dependency** for IDE features — the server is a single native binary
- **Windows first-class** — install, path handling, and shell interactions are part of the release surface

## Architecture

The workspace has 134 crates organized into focused layers:

| Layer | Crates |
| --- | --- |
| LSP server binary | `crates/perllsp`, `crates/perl-lsp` |
| Debug adapter | `crates/perl-dap` |
| Parser and lexer | `crates/perl-parser`, `crates/perl-lexer` |
| Semantic analysis | `crates/perl-semantic-analyzer` |
| Workspace indexing | `crates/perl-workspace-index` |
| LSP feature providers | `crates/perl-lsp-*` |
| Tree-sitter interop (C FFI reference) | `crates/tree-sitter-perl-c` |

The native Rust parser (`perl-parser`), lexer (`perl-lexer`), and analysis stack are the architectural center. Tree-sitter compatibility is a valuable interoperability surface: `tree-sitter-perl-c` is the conventional C/tree-sitter grammar binding maintained for compatibility and comparison. A Rust-native facade with tree-sitter-compatible ergonomics over the v3 parser is in development as `tree-sitter-perl-rs` (superseding the legacy Pest-based harness of the same name).

See [docs/README.md](docs/README.md) for the full crate map and design notes.

## Documentation

| What you need | Where to go |
| --- | --- |
| First-time setup | [docs/tutorials/GETTING_STARTED.md](docs/tutorials/GETTING_STARTED.md) |
| Editor-specific config | [docs/how-to/EDITOR_SETUP.md](docs/how-to/EDITOR_SETUP.md) |
| All configuration options | [docs/reference/CONFIG.md](docs/reference/CONFIG.md) |
| Commands reference | [docs/reference/COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) |
| Upgrade guide | [docs/how-to/UPGRADING.md](docs/how-to/UPGRADING.md) |
| Troubleshooting | [docs/how-to/TROUBLESHOOTING.md](docs/how-to/TROUBLESHOOTING.md) |
| Current status and metrics | [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) |
| Release roadmap | [docs/project/ROADMAP.md](docs/project/ROADMAP.md) |
| Full docs index | [docs/INDEX.md](docs/INDEX.md) |

## Contributing

```bash
cargo test --workspace --lib
cargo fmt --all
cargo clippy --workspace
nix develop -c just ci-gate   # required before push
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor workflow.

## Status

**Current release: v0.12.3** — public alpha. The 0.12.x line is building parser corpus confidence, diagnostic hardening, and distribution coverage toward the v0.13.0 public alpha announcement. See [docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the milestone ladder and [docs/project/status/index.md](docs/project/status/index.md) for live metrics.

## Security

Release artifacts include SBOM generation and provenance attestations. See [docs/reference/SUPPLY_CHAIN_SECURITY.md](docs/reference/SUPPLY_CHAIN_SECURITY.md).

## License

Dual licensed under MIT or Apache-2.0: [LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE)
