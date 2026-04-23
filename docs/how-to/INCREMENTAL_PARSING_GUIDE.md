# Incremental Parsing Guide

## Current State

The LSP server advertises `TextDocumentSyncKind::Incremental` (value 2) to clients. This refers to **how document text is transferred**, not whether parsing itself is incremental.

### What "Incremental" Means Here

When a client sends `textDocument/didChange`, it uses LSP's incremental sync format: each change carries a `range` and replacement `text`, rather than the full document content. The server applies these range-based edits to its in-memory Rope buffer via [`apply_changes`](../../crates/perl-lsp-rs/src/textdoc.rs).

### What Happens After Each Edit

After applying the text edits to the Rope, the LSP server does a **full reparse** of the document. There is no AST subtree reuse — `Parser::new(source).parse()` runs on the complete source text every time. This is correct and produces accurate diagnostics and symbol information, but it means every keystroke triggers a full parse regardless of edit size.

See [`crates/perl-lsp-rs/src/runtime/text_sync.rs`](../../crates/perl-lsp-rs/src/runtime/text_sync.rs) for the implementation.

## The `perl-incremental-parsing` Crate

The `perl-incremental-parsing` crate exists and contains substantial incremental parsing infrastructure:

- **`IncrementalState`** — Rope-backed document state with lexer checkpoints and token cache reuse
- **`IncrementalDocument`** — `Arc<Node>`-based document with subtree cache and priority-aware eviction
- **`IncrementalParserV2`** — Production-grade parser with AST subtree reuse metrics
- **`DocumentParser`** — Enum wrapper (`Full` | `Incremental`) gated on `PERL_LSP_INCREMENTAL` env var

This crate is a feature-gated optional dependency of `perl-parser` (behind the `incremental` feature flag). However, it is **not wired into the `perl-lsp` server** — `perl-lsp` does not depend on this crate and does not invoke it on `didChange` events.

The infrastructure in `perl-incremental-parsing` represents real implementation work but is not connected to the running LSP. Wiring it in is tracked as future work.

## Performance Characteristics (Current)

The full-reparse approach is fast enough for typical Perl files in practice:

- The v3 recursive-descent parser is written in Rust and generally handles small-to-medium files in under a millisecond on modern hardware
- An AST content cache (`ast_cache`) avoids re-parsing when the same text is seen twice (e.g., on `didSave` after `didChange`)
- Files larger than the configured size limit (`PERL_LSP_MAX_FILE_SIZE_KB`, default 512 KB) are skipped entirely with no AST

No 65µs or 99.7% node-reuse benchmarks apply to the current LSP path — those numbers were measured against the `perl-incremental-parsing` crate's internal test suite, which is not connected to the server.

## Rope Integration

The server does use a Rope for efficient text management and UTF-16 position conversion. Key modules:

- [`crates/perl-lsp-rs/src/textdoc.rs`](../../crates/perl-lsp-rs/src/textdoc.rs) — `Doc` struct, `apply_changes`, UTF-16 position conversion
- [`crates/perl-lsp-rs/src/state/document.rs`](../../crates/perl-lsp-rs/src/state/document.rs) — `DocumentState` holding the Rope, AST, and `LineStartsCache`

The Rope is used correctly for applying incremental text edits. It is not used to avoid reparsing.

## Roadmap

True incremental parsing (reusing AST subtrees across edits) requires wiring `perl-incremental-parsing` into `perl-lsp`'s `handle_did_change` path. The infrastructure exists; the integration does not. Contributions are welcome.

## Testing Commands

```bash
# Run the LSP text-sync tests (exercises full-reparse path)
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Run incremental-parsing crate unit tests (not connected to LSP server)
cargo test -p perl-incremental-parsing

# Run perl-parser incremental tests (feature-gated)
cargo test -p perl-parser --features incremental
```
