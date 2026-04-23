# ADR/Spec Findings — work-e551d1f1

## What This ADR Decides

This ADR records the foundational architectural decisions for perl-lsp's layered scorecard metric stack (issue #4099, umbrella #4062). The most critical decision is the **data-path architecture** between the runtime LSP server (where `SloTracker` collects statistics) and the build-time `cargo xtask metrics` commands (where scorecards are emitted).

## Key Decision

**Decision: Adopt a snapshot-receipt data path for runtime→xtask statistics flow.**

The LSP server will emit a `CoordinatorStatisticsReceipt` JSON snapshot to a well-known path (`.ci/metrics/receipts/`) when a metrics session ends. The `workspace_stats.rs` xtask command reads and aggregates these receipts. This decouples runtime instrumentation from build-time reporting and avoids holding locks across process boundaries.

Additionally, the ADR formally adopts:
- A **cold/warm/incremental regime tag** on every `SloTracker` operation
- A **9-recommendations → 7-scorecards** explicit mapping table
- A **bug-first policy** requiring `record_operation` fix before Phase 1

## Alternatives Considered

1. **In-process xtask** — Run `workspace_stats` as an in-process LSP request (`metrics/snapshot`). Rejected: pollutes the LSP request namespace, adds latency to production code paths, violates the "xtask is build-time only" convention.

2. **Shared-memory ring buffer** — Slab allocator in a shared memory segment that both the LSP server and xtask can read. Rejected: complex lifecycle management, OS-specific (Linux-only in practice), and the receipt pattern is simpler and matches how `sweep_stats.rs` already works.

3. **Direct struct serialization from `ProductionIndexCoordinator`** — Rejected per plan-reviewer finding: the current `slo/mod.rs` has a bug where `record_operation` broadcasts to all 8 trackers instead of just the one that ran. The receipt pattern sidesteps this bug by reading the already-broadcast data, but the bug must still be fixed in Phase 1.

## Consequences

**Benefits**:
- Clean separation between runtime (LSP server) and reporting (xtask)
- Receipts are machine-readable artifacts that can be versioned, committed, and diffed
- Matches existing pattern in `sweep_stats.rs` which already reads corpus-sweep receipts
- Allows multi-session aggregation (run LSP multiple times, xtask merges receipts)

**Tradeoffs/Downsides**:
- Adds a file-write path in the LSP server shutdown sequence (must be non-blocking/fail-open)
- Receipts accumulate; a cleanup policy is needed (out of scope for Phase 1, add in Phase 4)
- Introduces a schema (`.ci/metrics/receipts/schema.json`) that must remain stable

## Acceptance Criteria (from specs.md)

1. `cargo xtask metrics workspace-stats` emits per-operation latency tables (p50/p95/p99) and SLO compliance % by reading receipt files from `.ci/metrics/receipts/`
2. `CoordinatorStatisticsReceipt` JSON schema is documented in `docs/project/metrics/SCHEMA.md`
3. `SloTracker::record_operation` is fixed to record to only the matching `OperationType` tracker (not all 8)
4. Cold/warm/incremental regime is tagged on every operation tracked by `SloTracker`
5. Phase 2 tasks (`@INC` conformance, diagnostics correctness, editor intelligence) are scoped as design-only until labelled corpus fixtures exist
