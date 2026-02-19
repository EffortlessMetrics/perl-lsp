# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-incremental-parsing` is a **Tier 3 crate** (two-level internal dependencies) providing incremental parsing infrastructure for efficient document updates in the LSP server.

**Purpose**: Minimize re-parsing overhead when Perl documents change by reusing unaffected AST subtrees, lexer checkpoints, and cached token streams.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-incremental-parsing        # Build this crate
cargo test -p perl-incremental-parsing         # Run tests
cargo clippy -p perl-incremental-parsing       # Lint
cargo doc -p perl-incremental-parsing --open   # View documentation
```

## Architecture

### Internal Dependencies

- `perl-parser-core` -- Parser, Node, NodeKind, SourceLocation, position types
- `perl-edit` -- Edit/EditSet types (re-exported as `edit`)
- `perl-lexer` -- PerlLexer, Token, TokenType, LexerMode, Checkpointable, CheckpointCache

### External Dependencies

- `anyhow` -- Error handling in `apply_edits`
- `lsp-types` -- `TextDocumentContentChangeEvent`, `Diagnostic`, `Range`
- `ropey` -- Rope text buffer for UTF-16 position conversion
- `serde_json` -- LSP JSON change event parsing in integration module
- `tracing` -- Debug-level logging for cache eviction

### Key Types and Modules

| Type / Module | File | Purpose |
|---|---|---|
| `IncrementalState` | `incremental/mod.rs` | Rope-backed state with lex/parse checkpoints; `apply_edits()` entry point |
| `LineIndex` | `incremental/mod.rs` | Byte-to-(line,col) mapping via binary search |
| `LexCheckpoint` / `ParseCheckpoint` / `ScopeSnapshot` | `incremental/mod.rs` | Checkpoint types for resuming lexing/parsing |
| `Edit` / `ReparseResult` | `incremental/mod.rs` | LSP change conversion and reparse result |
| `IncrementalDocument` | `incremental_document.rs` | `Arc<Node>` document with `SubtreeCache`, priority eviction, `ParseMetrics` |
| `SubtreeCache` / `SymbolPriority` | `incremental_document.rs` | LRU cache with content-hash and range-based lookup; priority-aware eviction |
| `SimpleIncrementalParser` | `incremental_simple.rs` | Lightweight parser tracking reused vs reparsed node counts |
| `CheckpointedIncrementalParser` | `incremental_checkpoint.rs` | Lexer-checkpoint parser with `TokenCache` and `IncrementalStats` |
| `AdvancedReuseAnalyzer` | `incremental_advanced_reuse.rs` | Multi-strategy reuse analysis (direct, position-shift, content-update, aggressive) |
| `ReuseConfig` / `ReuseAnalysisResult` | `incremental_advanced_reuse.rs` | Configuration and result types for reuse analysis |
| `IncrementalEdit` / `IncrementalEditSet` | `incremental_edit.rs` | Edit with byte-shift arithmetic and batch application |
| `DocumentParser` | `incremental_integration.rs` | `Full` / `Incremental` enum for transparent LSP integration |
| `IncrementalConfig` | `incremental_integration.rs` | Runtime config; reads `PERL_LSP_INCREMENTAL` env var |
| `IncrementalParserV2` | `incremental_v2.rs` | Production-grade parser with comprehensive tree reuse and metrics |
| (stub) | `incremental_handler_v2.rs` | Deprecated; handler moved to `perl-lsp` |

### Re-exports from `lib.rs`

- `perl_edit` as `edit`
- `Node`, `NodeKind`, `SourceLocation`, `Parser`, `ast`, `error`, `parser`, `position` from `perl-parser-core`
- Everything from `incremental` module (glob re-export)

## Usage

```rust
use perl_incremental_parsing::{IncrementalState, Edit, apply_edits};

// Create state from source
let mut state = IncrementalState::new(source.to_string());

// Convert LSP change to Edit, then apply
let edit = Edit::from_lsp_change(&change, &state.line_index, &state.source);
if let Some(edit) = edit {
    let result = apply_edits(&mut state, &[edit])?;
    // result.changed_ranges, result.reparsed_bytes
}
```

## Important Notes

- Falls back to full reparse for edits > 64KB, multi-line edits > 10 lines, or multiple simultaneous edits
- `IncrementalDocument` uses `SymbolPriority` (Critical > High > Medium > Low) for cache eviction -- package/use/sub nodes are evicted last
- `incremental_handler_v2.rs` is a deprecated stub; actual LSP handler lives in `perl-lsp`
- Incremental mode in `DocumentParser` is gated by the `PERL_LSP_INCREMENTAL` environment variable
- All tests are inline (`#[cfg(test)]` modules); no separate `tests/` directory
