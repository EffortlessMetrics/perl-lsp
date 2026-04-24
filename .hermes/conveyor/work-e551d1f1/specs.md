# Specs: Metrics Scorecard Infrastructure — work-e551d1f1

## Feature Description

This spec covers the implementation of perl-lsp's layered scorecard metric stack, as defined in issue #4099 (reference-model findings from rust-analyzer, gopls, pyright, clangd) and umbrella issue #4062. The infrastructure provides developer-facing observability tooling: phase timings, heavy-hitter reports, memory accounting, and release-health dashboards.

The architectural decision record (ADR-008) governs the data-path between runtime instrumentation and build-time reporting.

## Feature Behavior

### Data Path Architecture

The LSP server emits `CoordinatorStatisticsReceipt` JSON files to `.ci/metrics/receipts/<session>.json` on metrics session end. Build-time `cargo xtask metrics` commands read and aggregate these receipts.

```
LSP Server (runtime)
  └─▶ CoordinatorStatisticsReceipt JSON
        └─▶ .ci/metrics/receipts/<session>.json
              └─▶ cargo xtask metrics workspace-stats (build-time)
```

### Core CLI Commands

#### `cargo xtask metrics workspace-stats`

Reads all `.ci/metrics/receipts/*.json`, emits:
- Per-operation latency table (p50/p95/p99 in µs) for each of the 8 `OperationType` variants
- SLO compliance % per operation type (threshold from `SloTracker` config)
- Cold/warm/incremental breakdown per operation
- Top-20 slowest individual operations with session ID and regime tag
- JSON output to `.ci/metrics/workspace.json` on `--json` flag

#### `cargo xtask metrics parser-stats`

Reads `benchmarks/results/*.json`, emits:
- Top-20 slowest files by mean parse time
- Aggregate p50/p95/p99 across all benchmarked files
- JSON output to `.ci/metrics/parser.json` on `--json` flag

#### `cargo xtask metrics memory`

Reads hierarchical memory breakdown (see AC-7), emits:
- Per-subsystem memory breakdown: parser structures, AST cache, semantic model, workspace index, document store, completion caches, module-resolution caches, DAP session state
- Process-wide RSS from `sysinfo`
- JSON output to `.ci/metrics/memory.json` on `--json` flag

#### `cargo xtask metrics release-health`

Reads `.ci/debt-ledger.yaml` and CI baseline JSON, emits:
- Flaky test count and list
- Known-issue count
- Merge-gate pass rate
- Post-merge regression count (issues opened within 7 days of merge)
- Hotfix/follow-up PR rate
- JSON output to `.ci/metrics/release-health.json` on `--json` flag

### Scorecard Organization

Seven scorecard JSON files under `.ci/metrics/`:

| Scorecard | File | Status |
|---|---|---|
| Parser (#4063) | `.ci/metrics/parser.json` | Partially done |
| Workspace/indexing (#4068) | `.ci/metrics/workspace.json` | Stub |
| Module resolution (#4067) | `.ci/metrics/module-resolution.json` | Stub |
| Diagnostics (#4065) | `.ci/metrics/diagnostics.json` | Stub |
| Editor intelligence (#4066) | `.ci/metrics/editor-intelligence.json` | Partially done |
| Engineering health (#4070) | `.ci/metrics/engineering-health.json` | Partially done |
| Release health (#4070) | `.ci/metrics/release-health.json` | Done |

### Regime Tagging

Every `SloTracker` operation carries a `Regime` tag:
- `Cold` — startup and initial indexing
- `Warm` — post-index, interactive use
- `Incremental` — edit-triggered operations

## Acceptance Criteria

### AC-1: `SloTracker` records to the correct operation tracker only

Given `SloTracker::record_operation(index_file)`, only the `index_file` tracker receives the record. No other tracker (e.g., `find_definition`, `find_references`) receives a spurious record.

**Verification**: Unit test that calls `record_operation` for each `OperationType` variant and asserts only the matching tracker incremented.

### AC-2: `CoordinatorStatisticsReceipt` JSON schema is documented

`docs/project/metrics/SCHEMA.md` exists and documents:
- The `CoordinatorStatisticsReceipt` schema with all fields and types
- The receipt directory: `.ci/metrics/receipts/`
- The retention policy (keep last 10 sessions, oldest deleted first)
- Examples for each operation type

**Verification**: `docs/project/metrics/SCHEMA.md` exists and contains the `CoordinatorStatisticsReceipt` schema definition.

### AC-3: `workspace_stats.rs` emits per-operation latency tables

`cargo xtask metrics workspace-stats` reads `.ci/metrics/receipts/*.json` and emits:
- p50/p95/p99 latency per operation type in µs
- SLO compliance % per operation type
- Cold/warm/incremental regime breakdown

**Verification**: Run with fixture receipts in `.ci/metrics/receipts/`, assert table output contains all 8 operation types.

### AC-4: Phase-timing output distinguishes cold/warm/incremental

The `workspace_stats.rs` output groups latency statistics by regime. A single operation type (e.g., `index_file`) produces three separate latency tables (Cold/Warm/Incremental).

**Verification**: Feed a receipt with mixed-regime operations, assert output has separate p50/p95/p99 per regime.

### AC-5: Top-20 slowest-file reports exist for parser metrics

`cargo xtask metrics parser-stats` emits a top-20 list of slowest files by mean parse time, sourced from `benchmarks/results/*.json`.

**Verification**: Run against fixture benchmark results, assert top-20 list is non-empty and ordered descending by mean time.

### AC-6: `@INC` conformance matrix is scoped as design-only

The spec explicitly marks `@INC` conformance matrix (#4067) as "design-only" pending labelled corpus fixtures. No implementation code is required for this item in Phase 1.

**Verification**: Spec document explicitly states this item is design-only.

### AC-7: Hierarchical memory report is bucketed by clangd-style categories

`cargo xtask metrics memory` output is organized into these buckets:
- Parser structures
- AST cache
- Semantic model
- Workspace index
- Document store
- Completion caches
- Module-resolution caches
- DAP session state

**Verification**: Memory JSON output has these 8 top-level keys.

### AC-8: Phase 2 tasks (diagnostics, editor intelligence) scoped as design-only

The spec explicitly marks diagnostics correctness (#4065) and editor intelligence (#4066) as "design-only" pending labelled corpus fixtures. No implementation code is required for these items in Phase 1.

**Verification**: Spec document explicitly states these items are design-only.

## Non-Goals

- Changes to the LSP wire protocol or client-side code
- Actual opt-in telemetry collection (design only, per issue #4099 Phase 4)
- VSCode extension changes
- `features.toml` capability catalog changes

## Dependencies

- `perl-workspace-index` `SloTracker` (already implemented, needs bug fix + regime tagging)
- `ProductionIndexCoordinator::statistics()` (already implemented, returns owned `CoordinatorStatistics`)
- `perl-percentile` crate (already available)
- `perl-parser-bench` benchmark runner (already implemented, produces `benchmarks/results/`)
- `perl-corpus` gold fixtures (needed for Phase 2, not Phase 1)
- `.ci/debt-ledger.yaml` and CI baseline JSON (used by `release_health.rs`, already maintained)

## Schema Stability

`.ci/metrics/*.json` files are machine-readable contracts. All changes to these schemas require:
1. A new ADR documenting the migration
2. A deprecation period (minimum one release) before removing fields
3. A schema version field (`"$schema_version": "1.0"`) in every scorecard JSON
