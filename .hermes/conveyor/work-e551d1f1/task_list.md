# Task List — work-e551d1f1: Metrics Scorecard Infrastructure

## Phase 1 — Developer Instrumentation (Weeks 1–2)

- [ ] **Task 1**: Fix `SloTracker::record_operation` bug — record to only the matching `OperationType` tracker, not all 8
- [ ] **Task 2**: Add `Regime` enum (Cold/Warm/Incremental) and tag operations in `SloTracker`
- [ ] **Task 3**: Implement `CoordinatorStatisticsReceipt` JSON emission on LSP server metrics session end
- [ ] **Task 4**: Implement `cargo xtask metrics workspace-stats` — read receipts, emit per-operation latency tables
- [ ] **Task 5**: Document `CoordinatorStatisticsReceipt` schema in `docs/project/metrics/SCHEMA.md`
- [ ] **Task 6**: Implement top-20 slowest-file reports in `parser_stats.rs` from benchmark actuals

## Phase 2 — Design (Weeks 2–4) — pending corpus fixtures

- [ ] **Task 7**: Design `@INC` conformance matrix test harness (design-only until fixtures exist)
- [ ] **Task 8**: Design diagnostics correctness scorecard (design-only until labelled corpus exists)
- [ ] **Task 9**: Design editor intelligence scorecard (design-only until labelled corpus exists)

## Phase 3 — Memory and Release Health (Weeks 4–6)

- [ ] **Task 10**: Wire `EstimateSize` implementations into `memory.rs` with clangd-style category buckets
- [ ] **Task 11**: Add post-merge regression and hotfix/follow-up PR metrics to `release_health.rs`
- [ ] **Task 12**: Add receipt retention policy to prevent unbounded `.ci/metrics/receipts/` growth

## Phase 4 — Polish and Optics (Weeks 6+)

- [ ] **Task 13**: Audit all `.ci/metrics/*.json` files for product-vs-execution metric separation
- [ ] **Task 14**: Design opt-in anonymous aggregate regression signal (gopls-style) — design doc only
