# Spec: Diagnostic Stability — Generation-Aware Fast-Path Publication

## Feature Description

Add a generation-snapshot staleness guard to `publish_parse_errors_fast` in `diagnostics.rs`. When the document's generation counter advances during the fast-path parse-error computation, skip publishing diagnostics for the stale version.

## Background

`publish_parse_errors_fast` (called immediately on every document change) publishes parse errors without checking whether the document version has since advanced. This causes intermediate/incomplete parse states (e.g., `my $x = `) to produce spurious error diagnostics that flicker as the user types.

The existing `publish_diagnostics` function (full diagnostics after debounce) already has this guard at lines 495–506. This spec extends the same pattern to the fast path.

## Acceptance Criteria

### AC1: Stale parse errors are not published
When `publish_parse_errors_fast` is called for document version N, but the document's generation has already advanced to N+1 or higher by the time the function attempts to publish, the function returns early without sending diagnostics to the client.

### AC2: Guard mirrors existing `publish_diagnostics` pattern
The implementation captures `doc.generation` at the start of the snapshot (matching the pattern at `diagnostics.rs:495–506`) and compares it to the current generation before publishing. If they differ, `tracing::debug` logs the skip and returns.

### AC3: No regression in non-stale case
When the generation has not advanced during computation, diagnostics are published normally. The guard does not affect the happy path.

### AC4: Test covers stale-generation skip path
A unit or integration test verifies that when generation advances during `publish_parse_errors_fast` computation, the function returns early and does not publish.

## Non-Goals

- This spec does NOT implement incremental AST reuse (deferred to separate PR)
- This spec does NOT change the 250ms debounce timing
- This spec does NOT modify `IncrementalDocument` or `IncrementalState`

## Dependencies

- `crates/perl-lsp/src/runtime/diagnostics.rs` — `publish_parse_errors_fast` function (line 548)
- Document `generation: Arc<AtomicU32>` field (already exists on `DocumentState`)
- `parking_lot::Mutex` for document state access (already used)

## Implementation Sketch

In `publish_parse_errors_fast` (`diagnostics.rs:548`), add near the start of the snapshot block (~line 555):

```rust
let gen_at_snapshot = generation.load(Ordering::SeqCst);
// ... existing snapshot logic ...
// Before publish call (~line 622):
if generation.load(Ordering::SeqCst) != gen_at_snapshot {
    tracing::debug!("Skipping stale parse error publish (generation advanced during computation)");
    return;
}
```

This mirrors the existing guard at `diagnostics.rs:495–506` in `publish_diagnostics`.

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-lsp/src/runtime/diagnostics.rs` | Add generation snapshot and comparison in `publish_parse_errors_fast` |
| `crates/perl-lsp/tests/` | Add integration test for stale-generation skip behavior |
