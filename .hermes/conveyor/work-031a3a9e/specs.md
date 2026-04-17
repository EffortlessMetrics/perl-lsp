# Specs: Engineering Health Scorecard — Per-Subsystem Stratification

## Feature Description

Expand the perl-lsp engineering-health scorecard to break down existing metrics
by subsystem, using crate-level as the primary data granularity with subsystem
aggregation as a configuration-driven view layer.

## Scope: 4 Phases (Phase 5 Removed)

This spec covers Phases 1-4 only. Phase 5 (Release smoke dashboard) is removed
per ADR-2026-001 and deferred to a separate issue.

## Non-Goals

- **Per-crate mutation scores**: cargo-mutants does not emit per-mutant kill status.
  Scores require per-crate runs (hours of compute) or upstream cargo-mutants changes.
  Mutant **counts** are the achievable metric.
- **Memory metrics**: AST cache size, document store size, index size, peak RSS.
  No collection infrastructure exists; deferred to a follow-up.
- **Multi-editor smoke testing**: VSCode/Neovim/Emacs × Linux/macOS/Windows smoke
  infrastructure does not exist; deferred to a separate issue.
- **Post-release defect rate tracking**: Requires issue filing workflow changes.
- **Tiered CI policy (Tier A/B/C)**: Policy decision, not an implementation task.

## Phase 1: Per-Crate Mutation Counts (No Change to Data, Clarify Display)

### Summary
The `collect_per_crate_mutation()` function already reads `mutants.out/mutants.json`
and counts mutants per crate. No functional change to data collection. The display
in `quality.md` already says "Mutants listed" (not "Mutants killed"), which is
accurate. Phase 1 adds documentation clarifying what data is and isn't available.

### Changes
1. **`xtask/src/tasks/update_status/quality.rs`**: Add a doc comment to
   `collect_per_crate_mutation()` explaining:
   - The function counts **listed** mutants only
   - No per-mutant kill status is available in `mutants.json`
   - Per-crate mutation scores require per-crate cargo-mutants runs or upstream changes

2. **`docs/project/status/quality.md`**: Add a `QUALITY_MUTATION_NOTES` marker block
   explaining the limitation. Content:
   ```
   <!-- BEGIN: QUALITY_MUTATION_NOTES -->
   **Note**: Per-crate mutation scores (killed ÷ total) require per-crate
   `cargo mutants` runs. Currently only mutant **counts** are available from
   the workspace-level `mutants.out/mutants.json`.
   <!-- END: QUALITY_MUTATION_NOTES -->
   ```

### Acceptance Criteria
- [ ] `collect_per_crate_mutation()` has a doc comment explaining the limitation
- [ ] `quality.md` has a `QUALITY_MUTATION_NOTES` block explaining scores are not available
- [ ] Running `just status-update` produces no errors

## Phase 2: Per-Subsystem Latency Aggregation

### Summary
Add a `PERFORMANCE_BY_SUBSYSTEM` section to `quality.md` that reads
`benchmarks/results/latest.json` (produced by `cargo xtask bench-run --output
benchmarks/results/latest.json`) and aggregates timing data by subsystem category
using the categories defined in `.ci/benchmark-thresholds.yaml` (parser / lexer /
lsp / index).

### Changes
1. **`xtask/src/tasks/update_status/quality.rs`**: Add a
   `collect_latency_by_subsystem(root: &Path) -> BTreeMap<String, LatencyStats>`
   function that:
   - Reads `benchmarks/results/latest.json` (or returns empty if not present)
   - Parses benchmark results and groups by category
   - Returns `BTreeMap<category, LatencyStats>` where `LatencyStats` contains
     p50_ms, p95_ms, p99_ms

2. **`xtask/src/tasks/update_status/quality.rs`**: Add a
   `format_latency_table(stats: &BTreeMap<String, LatencyStats>) -> String`
   function that renders a markdown table with columns: Category | p50 (ms) |
   p95 (ms) | p99 (ms)

3. **`xtask/src/tasks/update_status/quality.rs`**: Extend
   `generate_quality_status()` to call the new functions and populate a
   `PERFORMANCE_BY_SUBSYSTEM` marker block in `quality.md`

4. **`docs/project/status/quality.md`**: Add the `PERFORMANCE_BY_SUBSYSTEM`
   marker block with placeholder content until benchmark data exists

### Acceptance Criteria
- [ ] `collect_latency_by_subsystem()` reads `benchmarks/results/latest.json`
- [ ] Returns empty map gracefully if file doesn't exist
- [ ] `format_latency_table()` renders a valid markdown table with Category,
  p50, p95, p99 columns
- [ ] `PERFORMANCE_BY_SUBSYSTEM` block appears in `quality.md` after `just status-update`
- [ ] Running `just status-update` with no benchmark data produces no errors

## Phase 3: Flaky Test Tracker via Debt-Ledger

### Summary
Extend `.ci/debt-ledger.yaml`'s `flaky_tests` array with `failure_count` and
`last_failed_at` fields. Write a Python updater script that runs post-test to
populate these fields.

### Changes
1. **`.ci/debt-ledger.yaml`**: Extend the `flaky_tests` entry schema to include:
   ```yaml
   flaky_tests:
     - name: "..."
       failure_count: 0   # Running count of observed failures
       last_failed_at: null  # ISO8601 of most recent failure, null if never failed
       # ... existing fields (added, issue, tier, quarantine_days, expires, owner, notes, failure_pattern, affected_platforms)
   ```

2. **`.ci/scripts/update-flaky-tracker.py`** (new file): A Python script that:
   - Accepts a JSON report of test results (from `just test-full`)
   - For each test that failed with a flaky pattern, increments `failure_count`
     and updates `last_failed_at` in `.ci/debt-ledger.yaml`
   - Runs as a post-hook after `just test-full` in nightly CI only
   - Is informational only (does not fail CI)

3. **`xtask/src/tasks/update_status/quality.rs`**: Extend
   `generate_quality_status()` to read `.ci/debt-ledger.yaml` and add a
   `FLAKY_TEST_BULLETS` section to `quality.md` when `flaky_tests` is non-empty.
   Format: `- **Flaky tests**: N quarantined (M failures in last 30 days)`

4. **`docs/project/status/quality.md`**: Add the `FLAKY_TEST_BULLETS` marker block

### Acceptance Criteria
- [ ] `.ci/debt-ledger.yaml` schema accepts `failure_count` (integer) and
  `last_failed_at` (ISO8601 string or null)
- [ ] `update-flaky-tracker.py` exists in `.ci/scripts/` and is executable
- [ ] Running `update-flaky-tracker.py --help` shows usage
- [ ] `update-flaky-tracker.py` updates `.ci/debt-ledger.yaml` in-place
- [ ] `FLAKY_TEST_BULLETS` section appears in `quality.md` when `flaky_tests` is non-empty
- [ ] Running `just status-update` with empty `flaky_tests` produces no errors

## Phase 4: Per-Subsystem Test Counts

### Summary
Group the existing per-crate test counts (from `collect_per_crate_test_counts()`)
by subsystem using `.ci/subsystem-mapping.yaml` and display in `quality.md`.

### Changes
1. **`.ci/subsystem-mapping.yaml`** (new file): Configuration mapping each crate
   to a `StatusSubsystem` variant. Format:
   ```yaml
   crate_to_subsystem:
     perl-quote: Quality
     perl-parser: Parser
     perl-lsp: Lsp
     perl-dap: Dap
     perl-workspace-index: Workspace
     # ... all 124 crates
   ```

2. **`xtask/src/tasks/update_status/tests.rs`** or **`quality.rs`**: Add a
   `collect_subsystem_test_counts(root: &Path) -> BTreeMap<Subsystem, TestCounts>`
   function that:
   - Calls `collect_per_crate_test_counts(root)` to get per-crate counts
   - Reads `.ci/subsystem-mapping.yaml`
   - Groups by subsystem using the mapping
   - Returns `BTreeMap<Subsystem, TestCounts>` where `TestCounts` has fields:
     `test_count`, `ignore_count`

3. **`xtask/src/tasks/update_status/quality.rs`**: Extend
   `generate_quality_status()` to call the new function and add a
   `SUBSYSTEM_TEST_BULLETS` section to `quality.md` showing a table:
   ```
   | Subsystem | Tests | Ignored |
   |-----------|-------|---------|
   | Parser    | 1234  | 12      |
   | LSP       | 5678  | 34      |
   | ...       | ...   | ...     |
   ```

4. **`docs/project/status/quality.md`**: Add the `SUBSYSTEM_TEST_BULLETS` marker
   block with the per-subsystem test table

### Acceptance Criteria
- [ ] `.ci/subsystem-mapping.yaml` exists and maps all 124 crates to a subsystem
- [ ] `collect_subsystem_test_counts()` returns aggregated counts per subsystem
- [ ] `SUBSYSTEM_TEST_BULLETS` section appears in `quality.md` after `just status-update`
- [ ] Running `just status-update` produces no errors
- [ ] All 124 crates appear in exactly one subsystem (validation in the Rust code)

## Dependencies

| Phase | Dependency | Status |
|-------|-----------|--------|
| 1 | `mutants.out/mutants.json` from nightly CI | Exists conceptually, not yet run |
| 2 | `benchmarks/results/latest.json` from `cargo xtask bench-run` | `benchmarks.rs` confirmed to produce this |
| 3 | `.ci/debt-ledger.yaml` schema | Existing, needs extension |
| 4 | `.ci/subsystem-mapping.yaml` | New file |

## Shared Infrastructure

### Marker Blocks in `quality.md`
All new sections use fenced marker blocks (BEGIN/END) for `just status-update` to
replace:

```
<!-- BEGIN: QUALITY_MUTATION_NOTES -->
... content ...
<!-- END: QUALITY_MUTATION_NOTES -->

<!-- BEGIN: PERFORMANCE_BY_SUBSYSTEM -->
... content ...
<!-- END: PERFORMANCE_BY_SUBSYSTEM -->

<!-- BEGIN: FLAKY_TEST_BULLETS -->
... content ...
<!-- END: FLAKY_TEST_BULLETS -->

<!-- BEGIN: SUBSYSTEM_TEST_BULLETS -->
... content ...
<!-- END: SUBSYSTEM_TEST_BULLETS -->
```

### Anti-Drift Workflow
- `just status-update`: Regenerates all `docs/project/status/*.md` files
- `just status-check`: Verifies files are up to date, errors if stale
- All changes maintain this workflow — no breaking changes to the xtask interface
