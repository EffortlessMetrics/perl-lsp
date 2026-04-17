# Task List — work-e5cf7a7c: Diagnostic Stability (Generation Guard)

## PR 1: Generation-Aware Publication Guard

### Implementation
- [ ] Add generation snapshot capture to `publish_parse_errors_fast` in `diagnostics.rs`
- [ ] Add staleness comparison before publishing (return early if generation changed)
- [ ] Add `tracing::debug` log when skipping stale publish
- [ ] Verify implementation mirrors `publish_diagnostics` guard pattern (diagnostics.rs:495–506)

### Testing
- [ ] Add unit/integration test for stale-generation skip behavior
- [ ] Run `cargo test -p perl-lsp-rs` to verify no regressions
- [ ] Manual verification: type `my $x = ` rapidly and confirm no spurious error flicker

### Review
- [ ] Self-review code matches the spec exactly
- [ ] Ensure tracing messages are informative but not noisy

## PR 2: Incremental AST Reuse (Deferred — blocked on API design)

### Pre-requisite Design
- [ ] Investigate `IncrementalDocument::apply_edits()` return type
- [ ] Design API to expose `ParseResult<Vec<ParseError>>` from incremental parse
- [ ] Verify semantic analysis consistency across reused subtrees
- [ ] Create separate design issue for perl-incremental-parsing API changes

### Implementation (future)
- [ ] Use `incremental_doc.root` as document AST when incremental parse succeeds
- [ ] Fall back to full parse when incremental fails
- [ ] Ensure parse errors from incremental parse are preserved for diagnostics
- [ ] Add test verifying incremental AST is used after ranged edits

### Review (future)
- [ ] Verify parse error diagnostics are not lost
- [ ] Verify semantic analysis is consistent with reused AST
