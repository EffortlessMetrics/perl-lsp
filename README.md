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

`perl-lsp` is a workspace of Rust crates delivering a complete Perl 5 tooling stack: an LSP server (`perllsp`) implementing all 102 capabilities catalogued in `features.toml` (87 LSP + 10 DAP + 5 extension features), a DAP debug adapter, a recursive-descent parser, a context-aware lexer, and a semantic analyzer — packaged as a single native binary you can drop into any editor. It runs on Windows, macOS, and Linux.

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

- **Full LSP surface** — completions, diagnostics, hover, go-to-definition, find references, rename, formatting, semantic tokens, inlay hints, code actions, code lens, workspace symbols; every capability in `features.toml` has an implementation wired up (see [what the numbers mean](#what-the-numbers-mean-and-dont))
- **Native debug adapter** — DAP breakpoints, stepping, stack frames, variable inspection, and evaluate; no wrapper script required
- **Fast native parser** — recursive-descent v3 parser with a context-aware lexer; validated against a curated CPAN corpus
- **Semantic analysis** — symbol resolution, scope tracking, Moose/Moo method modifiers and role composition
- **Refactoring** — extract variable, extract subroutine, workspace-scoped rename, subroutine inlining
- **Diagnostics** — dead code highlighting, strict/warnings diagnostics, perlcritic integration with walk-up discovery
- **Zero-Perl dependency** for IDE features — the server is a single native binary
- **Windows first-class** — install, path handling, and shell interactions are part of the release surface

## Architecture

The native Rust parser stack is the architectural center of the workspace — the LSP server, diagnostics, hover, completion, and every other IDE feature read from it directly. Tree-sitter integration is an interop surface layered over that core, not a dependency of it.

### Entry points for external consumers

Different users walk in through different doors. Pick the one that matches your use case:

| You want to… | Use |
| --- | --- |
| Get Perl IDE features in an editor | VS Code extension (`perl-lsp-rs`) or the `perllsp` binary from [Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) |
| Perl syntax support for tree-sitter consumers (Neovim, Helix, GitHub) | [`tree-sitter-perl-c`](crates/tree-sitter-perl-c) — conventional C grammar binding |
| Query a Perl AST from Rust with tree-sitter-style ergonomics | [`tree-sitter-perl-rs`](crates/tree-sitter-perl-rs) — Rust-native facade over the v3 parser *(in development)* |
| Parse Perl from Rust directly with full fidelity | [`perl-parser`](crates/perl-parser) (+ [`perl-lexer`](crates/perl-lexer)) — the recursive-descent v3 parser |
| Tokenize Perl only | [`perl-lexer`](crates/perl-lexer) — context-aware tokenizer, no parse tree |
| Resolve symbols and track scopes over a parsed Perl AST (including Moose/Moo method modifiers) | [`perl-semantic-analyzer`](crates/perl-semantic-analyzer) — scope tracking, symbol extraction, role composition |
| Index and search Perl symbols across a whole project | [`perl-workspace-index`](crates/perl-workspace-index) — cross-file symbol index and refactoring orchestration |
| Validate Perl regex patterns for ReDoS, catastrophic backtracking, or embedded code execution | [`perl-regex`](crates/perl-regex) — safety/complexity checks (not a regex parser) |
| Generate Perl fixtures or test against a curated corpus | [`perl-corpus`](crates/perl-corpus) — library plus `perl-corpus` CLI for proptest strategies, edge cases, and deterministic codegen |
| Debug Perl from a DAP-speaking editor | [`perl-dap`](crates/perl-dap) via the `perllsp` binary's DAP server mode |

### Workspace layers

| Layer | Crates | Role |
| --- | --- | --- |
| LSP server binary | `crates/perllsp`, `crates/perl-lsp` | Protocol loop, request dispatch |
| Debug adapter | `crates/perl-dap` | DAP server for stepping, breakpoints, evaluate |
| **Parser stack (center)** | `crates/perl-parser`, `crates/perl-lexer`, `crates/perl-parser-core` | Recursive-descent v3 parser and context-aware lexer — all IDE features read from this |
| Semantic analysis | `crates/perl-semantic-analyzer` | Scope tracking, symbol resolution, Moose/Moo handling |
| Workspace indexing | `crates/perl-workspace-index` | Cross-file symbol index |
| LSP feature providers | `crates/perl-lsp-*` | Per-feature crates (hover, definition, rename, …) |
| Tree-sitter interop | `crates/tree-sitter-perl-c`, `crates/tree-sitter-perl-rs` | See split below |

**Tree-sitter split.** Two crates share the family name but play different roles:

- **`tree-sitter-perl-c`** — the conventional C grammar binding, maintained for compatibility with tree-sitter consumers and as a reference point for comparison. Not on the LSP's critical path.
- **`tree-sitter-perl-rs`** — a Rust-native facade over the v3 parser that exposes tree-sitter-compatible ergonomics without the C grammar. In development.

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
nix develop -c just ci-gate   # required before merge
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor workflow.

## Status

**Current release: v0.12.3** — public alpha. The 0.12.x line is building parser corpus confidence, diagnostic hardening, and distribution coverage toward the v0.13.0 public alpha announcement. See [docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the milestone ladder and [docs/project/status/index.md](docs/project/status/index.md) for live metrics.

### What the numbers mean (and don't)

The project tracks a few distinct metrics that are easy to conflate. Each one scopes a different question:

| Metric | Current | What it measures | What it does **not** measure |
| --- | --- | --- | --- |
| LSP/DAP capability coverage | 102 / 102 | Every capability catalogued in `features.toml` has an implementation wired up | Per-capability correctness, completeness on edge cases, or subjective UX quality |
| Parser corpus — CPAN top 1000 | 95.3% (8931 / 9372) | File-level clean parse rate: share of files the parser processes without recording errors | Semantic fidelity of the AST, cross-file analysis, or any LSP-level correctness |
| Parser corpus — Ubuntu system Perl | 97.1% (6890 / 7095) | Same, against the Ubuntu system-installed Perl compatibility baseline | Same |
| Parser corpus — project corpus | 100.0% (91 / 91) | Deterministic regression baseline that must stay clean | Same |
| End-to-end UX confidence | *qualitative* | Currently covered by manual editor smoke workflows and open-issue burn-down — not a published number | Anything about parser breadth, protocol catalog size, or capability count |

The last row is the important one: *none* of the automated metrics above measure whether a real editing session feels good. That's validated through workflow smoke tests and the list of known gaps below, not by a dashboard.

Live numbers live in [docs/project/status/parser.md](docs/project/status/parser.md); this table may lag a merge cycle.

### Known gaps toward solid UX

In-flight structural work for the v0.13.0 alpha. None of these block basic use, but each one is a real rough edge in a realistic workflow:

- **Parser error recovery while typing** — incomplete `sub`/`if`/`while` blocks and unclosed delimiters cascade into spurious errors; scope visibility is limited after an error node ([#3496](https://github.com/EffortlessMetrics/perl-lsp/issues/3496), [#3499](https://github.com/EffortlessMetrics/perl-lsp/issues/3499))
- **Symbol visibility across imports and inheritance** — parent-class methods (`@ISA` / `use parent` / `use base`), dynamic `require` and `Module->import`, tag imports (`POSIX :tag`), and `use constant` aren't fully resolved by the workspace index ([#3482](https://github.com/EffortlessMetrics/perl-lsp/issues/3482), [#3472](https://github.com/EffortlessMetrics/perl-lsp/issues/3472), [#3475](https://github.com/EffortlessMetrics/perl-lsp/issues/3475), [#3476](https://github.com/EffortlessMetrics/perl-lsp/issues/3476))
- **Pragma tracker** — dynamic pragmas inside `eval STRING` aren't conservatively handled yet ([#3489](https://github.com/EffortlessMetrics/perl-lsp/issues/3489)); `use if CONDITION, PRAGMA` (#3485) and `use feature 'switch'` (#3398) were fixed in the same cycle
- **Workspace / multi-root correctness** — in a multi-root VS Code workspace the server still behaves as "first folder wins" for config loading, indexing, module resolution, and symbol search; `workspace/configuration` isn't implemented yet, so per-folder config and dynamic scoping don't propagate ([#3513](https://github.com/EffortlessMetrics/perl-lsp/issues/3513), [#3515](https://github.com/EffortlessMetrics/perl-lsp/issues/3515))

## Security

Release artifacts include SBOM generation and provenance attestations. See [docs/reference/SUPPLY_CHAIN_SECURITY.md](docs/reference/SUPPLY_CHAIN_SECURITY.md).

## License

Dual licensed under MIT or Apache-2.0: [LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE)
