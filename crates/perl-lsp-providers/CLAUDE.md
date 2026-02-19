# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Crate**: `perl-lsp-providers`
- **Version**: 0.9.0
- **Tier**: 4 (three-level dependencies)
- **Purpose**: Central aggregation crate that re-exports all LSP feature provider microcrates and provides IDE integration shims for the `perl-lsp` server.

## Commands

```bash
cargo build -p perl-lsp-providers        # Build
cargo test -p perl-lsp-providers         # Test
cargo clippy -p perl-lsp-providers       # Lint
cargo doc -p perl-lsp-providers --open   # View docs
```

## Architecture

### Dependencies

**Core analysis crates**:
- `perl-parser-core` -- parsing infrastructure (re-exports `Node`, `NodeKind`, `Parser`, etc.)
- `perl-semantic-analyzer` -- semantic analysis
- `perl-workspace-index` -- cross-file symbol indexing
- `perl-refactoring` -- refactoring utilities
- `perl-incremental-parsing` -- incremental parse updates
- `perl-lexer` -- tokenization
- `perl-position-tracking` -- source position mapping

**LSP feature provider microcrates** (each re-exported as a top-level module):
- `perl-lsp-completion` -> `completion`
- `perl-lsp-navigation` -> `navigation`
- `perl-lsp-diagnostics` -> `diagnostics`
- `perl-lsp-code-actions` -> `code_actions`
- `perl-lsp-rename` -> `rename`
- `perl-lsp-semantic-tokens` -> `semantic_tokens`
- `perl-lsp-inlay-hints` -> `inlay_hints`
- `perl-lsp-formatting` -> `formatting`
- `perl-lsp-tooling` -> `tooling`

### Key Modules

| Path | Purpose |
|------|---------|
| `lib.rs` | Re-exports microcrates as top-level modules; re-exports core parser types |
| `ide/` | IDE integration stubs (call hierarchy, cancellation, execute command) |
| `ide/lsp/` | Deprecated LSP stub (empty, points to `perl-lsp`) |
| `ide/lsp_compat/` | Legacy compatibility shims for ~20 LSP feature modules |

### Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `lsp-compat` | Yes | Enables `lsp-types` dependency and LSP protocol type shims |

### Platform-specific

- `nix` dependency (signal handling) is unix-only via `cfg(unix)`
- `execute_command` and `lsp_document_link` modules excluded on `wasm32`

## Usage

```rust
// Preferred: top-level re-exports
use perl_lsp_providers::completion;
use perl_lsp_providers::diagnostics;
use perl_lsp_providers::navigation;

// Core parser types re-exported from perl-parser-core
use perl_lsp_providers::{Parser, Node, NodeKind};

// Deprecated legacy paths (still compile)
use perl_lsp_providers::ide::lsp_compat::completion;
```

## Important Notes

- This crate does NOT define a `Providers` struct or aggregator type; it is purely a re-export facade
- The `ide/lsp/` and `ide/` submodules are deprecated stubs pointing users to `perl-lsp`
- The `tooling_export` module is deprecated in favor of `tooling`
- Used by `perl-lsp` as its primary provider dependency
