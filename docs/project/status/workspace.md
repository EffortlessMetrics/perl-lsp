# Workspace & Indexing Scorecard

> Generated sections are updated by `cargo xtask update-status --only workspace`.
> Do not hand-edit between the `<!-- BEGIN: -->` and `<!-- END: -->` markers.
> Narrative sections above and below are human-owned.

## Overview

The workspace index is the substrate that every editor feature depends on: goto-definition
lands in the right file, completion only offers live symbols, and rename touches every
real reference. If the index is stale, all those features silently degrade — the feature
catalog still shows 100% because the *code paths* work, but they work on bad data.

This scorecard measures three properties of the index substrate:

| Signal | What a failure means |
|--------|----------------------|
| Stale-index defect rate | Deleted/renamed symbols appear in goto-def or completion |
| Incremental reindex latency | Saves feel sluggish; editor lags behind user edits |
| Multi-root correctness | Only the first workspace folder is searched; second root is invisible |

## Stale-Index Defect Rate

<!-- BEGIN: WORKSPACE_STALE_RATE -->
| **Stale-index defect rate** | 0 / 7 scenarios tested | 0% | see `cargo test -p perl-workspace-index -- scorecard` |
<!-- END: WORKSPACE_STALE_RATE -->

## SLO Targets

<!-- BEGIN: WORKSPACE_SLO_TABLE -->
| Operation | SLO Target | Source |
|-----------|-----------|--------|
| Index initialization (P95) | < 5 000 ms | `perl-workspace-index-slo` |
| Incremental reindex (P95) | < 100 ms | `perl-workspace-index-slo` |
| Definition lookup (P95) | < 50 ms | `perl-workspace-index-slo` |
| Completion (P95) | < 100 ms | `perl-workspace-index-slo` |
| Hover (P95) | < 50 ms | `perl-workspace-index-slo` |
<!-- END: WORKSPACE_SLO_TABLE -->

## Multi-Root Test Coverage

<!-- BEGIN: WORKSPACE_MULTIROOT -->
| **Multi-root integration tests** | 8 / 8 tests | 8 / 8 | `just ci-workspace-multiroot` (nightly gate) |
<!-- END: WORKSPACE_MULTIROOT -->

## Fixture Workspaces

<!-- BEGIN: WORKSPACE_FIXTURES -->
| Scale | Path | File count | Purpose |
|-------|------|-----------|--------|
| small | `test_corpus/workspaces/small/` | 10 | Smoke + SLO P95 baseline |
| medium | `test_corpus/workspaces/medium/` | 100 | Typical project scale |
| large | `test_corpus/workspaces/large/` | 1000 | Enterprise scale |
| xlarge | `test_corpus/workspaces/xlarge/` | ~10 000 (generated) | Stress / limit discovery |
<!-- END: WORKSPACE_FIXTURES -->

## Metrics Bullets

<!-- BEGIN: WORKSPACE_METRICS_BULLETS -->
- **Stale-index defect rate**: 0 stale-symbol defects across 7 tested deletion/rename scenarios (unit tests in `crates/perl-workspace-index/tests/workspace_scorecard.rs`)
- **Incremental reindex SLO**: P95 target = 100ms (from `perl-workspace-index-slo`); measured in `scorecard_incremental_reindex_latency_within_slo`
- **Multi-root tests**: 8 integration tests in `crates/perl-lsp-rs/tests/multi_root_workspace_tests.rs` activated in nightly CI gate via `just ci-workspace-multiroot` (PR #4137)
- **Fixture workspaces**: 4 scales at `test_corpus/workspaces/` (10 / 100 / 1000 committed + xlarge generated on demand)
<!-- END: WORKSPACE_METRICS_BULLETS -->

## Open Work

- PR 3 of 3 (per plan-reviewer option A): stale-index defect harness with realistic LSP session replay (didOpen → didChange → didSave → delete → assert)
- Promote `ci-workspace-multiroot` from nightly to merge gate once 10 consecutive nightly passes are confirmed
- Surface real P50/P95 latency numbers from benchmarks (`perl-workspace-index/benches/workspace_index_benchmark.rs`) into this file via `xtask update-status --only workspace`
- Add a direct signal row for the shipped `workspace/configuration` handler (#3515) in the next `xtask update-status --only workspace` refresh

## How to Update

```bash
cargo xtask update-status --only workspace   # regenerate all marked sections
cargo xtask update-status --check --only workspace   # verify (CI gate)
cargo test -p perl-workspace-index -- scorecard_  # run the scorecard tests
just ci-workspace-multiroot                  # run multi-root integration tests (nightly)
```
