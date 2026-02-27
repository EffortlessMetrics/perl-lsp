# perl-incremental-parsing

Incremental parsing infrastructure for efficient re-parsing of Perl source code in response to document edits, designed for LSP integration.

## Overview

This crate provides multiple incremental parsing strategies that minimize re-parsing overhead by reusing unaffected AST subtrees when documents change. It converts LSP `textDocument/didChange` events into efficient partial re-parses using lexer checkpoints, subtree caching with LRU eviction, and content-based node hashing.

## Key Types

- **`IncrementalState`** -- Rope-backed document state with lexer/parse checkpoints and the `apply_edits` entry point
- **`IncrementalDocument`** -- `Arc<Node>`-based document with subtree cache, priority-aware eviction, and per-cycle `ParseMetrics`
- **`SimpleIncrementalParser`** -- Lightweight parser tracking reused vs. reparsed node counts
- **`CheckpointedIncrementalParser`** -- Lexer-checkpoint-driven parser with token cache reuse
- **`AdvancedReuseAnalyzer`** -- Multi-strategy reuse analyzer (structural, position-shifted, content-updated, aggressive matching)
- **`DocumentParser`** -- Enum wrapper (`Full` | `Incremental`) for transparent LSP integration
- **`IncrementalEdit` / `IncrementalEditSet`** -- Edit representation with byte-shift arithmetic and batch application

## Part of the `perl-lsp` Workspace

This crate is a Tier 3 member of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace. It depends on `perl-parser-core`, `perl-edit`, and `perl-lexer`.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
