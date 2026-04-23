# ADR-008: Metrics Scorecard Architecture

**Status**: Proposed
**Date**: 2026-04-19
**Decision Makers**: Metrics Infrastructure Team
**Technical Story**: [Issue #4099 - Metrics research synthesis: reference-model findings from rust-analyzer, gopls, pyright, clangd](https://github.com/EffortlessMetrics/perl-lsp/issues/4099)

## Context and Problem Statement

Issue #4099 is a metrics research synthesis that surveys how four mature language-server projects (rust-analyzer, gopls, pyright, clangd) expose telemetry and observability. The research produced 9 prioritized recommendations mapped to a 7-scorecard design. Before implementation can begin, three architectural decisions must be resolved:

1. **Runtime→xtask data path** — `SloTracker` (the latency instrumentation) lives in `ProductionIndexCoordinator`, a runtime LSP component. `workspace_stats.rs` is a build-time `cargo xtask` command. Without a data path, the highest-ROI task (Task 1) is blocked.

2. **`SloTracker` broadcast bug** — `record_operation` in `slo/mod.rs` records to all 8 operation trackers simultaneously instead of just the one that ran. Statistics will be corrupted before they reach xtask output.

3. **Recommendation→scorecard mapping** — The 9 recommendations from the research don't map to the 7 scorecards with an explicit table. Tasks may duplicate effort or leave scorecards orphaned.

## Decision Drivers

- **Separation of concerns** — The LSP server is a long-running process; xtask commands are build-time and may run without a server present.
- **Non-regression** — Adding metrics instrumentation must not measurably slow latency-sensitive LSP operations (completion, hover).
- **Corpus dependency** — Phase 2 tasks (#4065 diagnostics, #4066 editor intelligence) require labelled corpus fixtures that don't exist yet.
- **Schema stability** — `.ci/metrics/*.json` files are machine-readable contracts consumed by downstream tooling.

## Considered Options

### Option 1: Snapshot-Receipt Data Path (CHOSEN)

**Architecture**: The LSP server emits a `CoordinatorStatisticsReceipt` JSON to `.ci/metrics/receipts/<session>.json` when a metrics session ends. `workspace_stats.rs` reads and aggregates receipts from this directory.

```
LSP Server (runtime)
  └─▶ CoordinatorStatisticsReceipt JSON
        └─▶ .ci/metrics/receipts/<session>.json
              └─▶ cargo xtask metrics workspace-stats (build-time)
```

**Pros:**
- ✅ Clean separation between runtime and build-time
- ✅ Receipts are versioned, diffable, and can be committed for reproducibility
- ✅ Matches existing pattern in `sweep_stats.rs` (already reads corpus-sweep receipts)
- ✅ Allows multi-session aggregation
- ✅ `ProductionIndexCoordinator::statistics()` returns an owned struct (no lock held in return value)

**Cons:**
- ❌ Adds file-write on LSP server shutdown (must be fail-open/non-blocking)
- ❌ Receipt accumulation requires a cleanup policy (deferred to Phase 4)
- ❌ Introduces a new schema that must remain stable

### Option 2: In-Process LSP Request (`metrics/snapshot`)

**Architecture**: Add a new LSP method `metrics/snapshot` that returns the current `CoordinatorStatistics` synchronously.

**Pros:**
- ✅ No new file format or persistence layer

**Cons:**
- ❌ Polls the request namespace; pollutes production LSP traffic
- ❌ Requires a running LSP server for xtask to work (violates "xtask is build-time" convention)
- ❌ Adds latency to production code paths

### Option 3: Shared-Memory Ring Buffer

**Architecture**: Slab allocator in a POSIX shared memory segment (`/dev/shm`) that both the LSP server and xtask can read without IPC overhead.

**Pros:**
- ✅ Zero-copy, very low latency

**Cons:**
- ❌ Complex lifecycle management (who creates, who destroys, what happens on crash)
- ❌ OS-specific (Linux-only in practice)
- ❌ Over-engineered for the use case

## Decision: Snapshot-Receipt Data Path

We adopt Option 1: the snapshot-receipt pattern. The LSP server writes `CoordinatorStatisticsReceipt` JSON files to `.ci/metrics/receipts/` on metrics session end. `workspace_stats.rs` reads and aggregates these receipts.

### Supporting Decisions

**Decision A: Bug-first Phase 1**

`SloTracker::record_operation` must be fixed to record only to the matching `OperationType` tracker before any Phase 1 task begins. This bug would corrupt all statistics; shipping Phase 1 with it unfixed would produce misleading metrics.

**Decision B: Regime Tagging**

`SloTracker` operations must carry a `Regime` tag:
- `Cold` — operations during LSP server startup and initial indexing
- `Warm` — operations after indexing settles, normal interactive use
- `Incremental` — operations triggered by in-editor edits

This matches how rust-analyzer and pyright report cold/warm/incremental phase timings separately.

**Decision C: 9→7 Scorecard Mapping**

| Recommendation | Scorecard | Issue |
|---|---|---|
| Rec 1+2: Phase-timing CLI | Workspace/indexing (#4068) | #4099 |
| Rec 3: Heavy-hitter top-N | Parser (#4063) + Workspace (#4068) | #4099 |
| Rec 4: Workspace first-class scorecard | Workspace/indexing (#4068) | #4068 |
| Rec 5: `@INC` conformance matrix | Module resolution (#4067) | #4067 |
| Rec 6: Diagnostics + editor intelligence | Diagnostics (#4065) + Editor intelligence (#4066) | #4065, #4066 |
| Rec 7: Hierarchical memory | Engineering health (#4070) | #4070 |
| Rec 8: Product vs execution separation | Engineering health (#4070) | #4070 |
| Rec 9: Release-health model | Engineering health (#4070) | #4070 |

**Decision D: Phase 2 Scoped as Design-Only**

Recommendations 5 and 6 (@INC conformance, diagnostics correctness, editor intelligence) depend on labelled corpus fixtures that don't exist yet. Scoping them as "implement" would produce stalled work items. Phase 2 is scoped to design (spec + test-harness skeleton) until fixtures are available.

## Consequences

### Benefits

- Clean separation between runtime (LSP server) and reporting (xtask)
- Receipts can be committed to git for reproducibility and diffing
- Multi-session aggregation is possible by reading multiple receipt files
- Matches existing corpus-receipt pattern in `sweep_stats.rs`
- The `record_operation` bug fix is a prerequisite, not a parallel concern

### Tradeoffs / Risks

- **Receipt accumulation** — Without a cleanup policy, `.ci/metrics/receipts/` will grow indefinitely. A retention policy (e.g., keep last N sessions) should be added in Phase 4.
- **Schema stability** — `CoordinatorStatisticsReceipt` schema must be documented in `docs/project/metrics/SCHEMA.md`. Breaking changes require a migration story.
- **Shutdown ordering** — The receipt must be written before the LSP server fully shuts down. If the write fails, it must be fail-open (don't crash the server on metrics failure).
- **Test gap** — No existing test validates receipt format or the `workspace_stats.rs` aggregation logic. A unit test for the receipt schema and a fixture for `workspace_stats.rs` should be added in Phase 1.

## Out of Scope

- Actual opt-in telemetry collection (Phase 4 design only, per issue #4099)
- Changes to the LSP wire protocol or client-side code
- VSCode extension changes
- `features.toml` capability catalog changes
