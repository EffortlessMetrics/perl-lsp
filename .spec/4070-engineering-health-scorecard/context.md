# Context — Engineering Health Scorecard (#4070)

## Problem Statement

Developers and maintainers lack visibility into engineering health metrics (test counts, mutation scores, benchmark latency, flaky test tracking). These metrics are collected in CI but not surfaced to the dashboard. This makes it hard to identify health regressions early or track quality improvements over time.

## Scope: MVP (Phase 1)

This spec implements **minimum viable product** for engineering health scorecard:
1. Per-crate test count extraction and surfacing (new in this builder)
2. Per-crate mutation score extraction and surfacing (new in this builder)

Both metrics already exist as collected data; this builder makes them visible.

**Deferred to Phase 2** (out of scope):
- Flaky test tracking from `.ci/debt-ledger.yaml`
- Ignored test tracking via `cargo xtask ignored-tests`
- Release smoke test pass rate
- Memory usage trends
- Post-release defect rate

## Key Decisions

### 1. Per-Crate Grouping from Test Names

**Source**: `cargo test --list --format=terse` output
**Format per line**: `perl_parser::tests::test_foo: test`
**Extraction**: Crate name is the first segment before `::` (everything up to the first double-colon)

**Example**:
```
perl-parser::parser_tests::test_parse_assignment: test
perl-lsp-rs::completion::test_builtin_function: test
perl-workspace-index::indexing::test_symbol_visibility: test
```

**Implementation**: Use regex `^([^:]+)::` to extract crate name prefix.

### 2. Mutation Score Parsing

**Source**: `cargo mutants --json --output <dir>` produces `mutants.out/mutants.json`
**Format**: Single JSON array, each element has `"package": "perl-quote"` field
**Extraction**: Group array elements by package, count items per crate

**Example output structure**:
```json
[
  { "package": "perl-parser", "name": "...", "genre": "FnValue" },
  { "package": "perl-parser", "name": "...", "genre": "BinaryOperator" },
  { "package": "perl-quote", "name": "...", "genre": "FnValue" }
]
```

**Aggregation**: Count mutants per crate (perl-parser: 2, perl-quote: 1)

**Note from accuracy-scout**: `cargo mutants --json --output` uses a _directory_ argument, not a file. The output goes into `<directory>/mutants.out/mutants.json`. The builder must handle this path correctly.

### 3. Fallback on Missing Data

**Test counts**: Always available (cargo test --list is always run)
**Mutation scores**: May not be available if mutation run hasn't completed yet

**Behavior**: If `mutants.out/mutants.json` is missing or unparseable, gracefully fall back to aggregate 87% (the hard-coded value currently in `generate_quality_status()`). No error, no panic.

### 4. Status File Location and Format

**File**: `docs/project/status/quality.md` (new, parallel to existing `parser.md`, `lsp.md`, `tests.md`)
**Format**: Markdown with fenced tables (existing project standard)
**Markers**: Use `<!-- BEGIN: X_TABLE -->` / `<!-- END: X_TABLE -->` for automated updates (existing replace_block pattern)

**Update mechanism**: The `cargo xtask status-update` command (already exists at `xtask/src/tasks/update_status.rs`) calls `generate_quality_status()`, which will replace the markers. No new xtask subcommand needed.

### 5. Table Structure

**Test counts table**:
```
| Crate | Test Count | Status |
|-------|------------|--------|
| perl-parser | 234 | — |
| perl-lsp-rs | 156 | — |
```

**Mutation score table**:
```
| Crate | Mutation Score | Coverage |
|-------|----------------|----------|
| perl-parser | 123 | — |
| perl-quote | 45 | — |
```

"Status" and "Coverage" columns are placeholder rows (to be filled in by future phases with trend indicators, regressions, etc.). For MVP, just show dashes.

## Alternatives Considered and Rejected

| Alternative | Why rejected |
|-------------|-------------|
| Include flaky test tracking in MVP | Requires automation to parse test names into subsystems (30-50 lines). Deferred to phase 2 for focused MVP. |
| Include ignored test counts in MVP | Requires schema agreement with `.ci/debt-ledger.yaml`. Deferred pending #4105 (infrastructure) landing. |
| Include latency metrics in MVP | Benchmark results are in `benchmarks/results/latest.json` but require Criterion JSON parsing and p50/p95 extraction. Larger scope; defer. |
| Single metrics file for all subsystems | Would create rebase conflicts if multiple builders touch it. One file per subsystem keeps changes isolated. |
| Hard-code both metrics (no parsing) | Defeats the purpose of surfacing data. Metrics should be auto-populated from CI data. |
| Parse mutants output in post-processing (offline) | Risk: post-process script breaks, metrics don't update. Inline parsing in xtask ensures freshness on every status-update run. |

## Testing Strategy

**Unit tests** (in `cargo test -p xtask`):
1. Parse synthetic `cargo test --list` output, verify per-crate grouping
2. Parse synthetic mutants JSON, verify per-crate aggregation
3. Fallback on missing mutation data (no error)
4. Format tables with correct markdown structure

**Integration test** (manual):
```bash
cargo xtask status-update
cat docs/project/status/quality.md | grep -E "perl-parser|perl-lsp-rs"
# Should show per-crate test counts and mutation scores
```

**No new CI gates needed**: This data is surfaced to the dashboard, not enforced as a gate. The scorecard metrics infrastructure (#4105) will later add floor-enforcement if desired.

## Coordination with #4105

#4105 (metrics ratchet infrastructure) will later add floor-enforcement for engineering_health scorecard. For now, this builder just surfaces existing data without enforcement.

**No coupling**: This builder does not depend on #4105 landing first. The data flows independently.

**Coordination point**: Both feed into the same `docs/project/status/` dashboard eventually.

## Verification Path

1. **status-update runs without error**: `cargo xtask status-update`
2. **quality.md created**: `ls docs/project/status/quality.md`
3. **Tables populated**: `grep perl-parser docs/project/status/quality.md`
4. **Test counts visible**: `grep -E "[0-9]+" docs/project/status/quality.md` shows numeric counts
5. **Mutation scores visible**: `grep -E "[0-9]+" docs/project/status/quality.md` includes per-crate scores
6. **Clippy clean**: `cargo clippy -p xtask -- -D warnings`

## Scope Exclusions

- Does NOT implement the ratchet check (that's #4105)
- Does NOT add new measurements (all data already exists)
- Does NOT change CI workflow configuration (collection already happens)
- Does NOT add new xtask subcommands (status-update already exists)
- Does NOT modify the root README (status row already links correctly)
- Does NOT implement Phase 2 metrics (flaky, ignored, latency, memory, defect rate)

## Open Questions Resolved

- Q: Where does mutation data come from?
  A: `cargo mutants --json --output <dir>` is already run in CI (justfile line 284). Output is available to status-update.

- Q: How accurate is the per-crate grouping from test names?
  A: 100% accurate for tests written in the standard pattern (`perl_crate::tests::test_name`). Crate name is always the first segment before `::`.

- Q: What if a test belongs to multiple crates?
  A: Not possible in Rust module structure. Each test lives in one crate and appears under that crate's name in test list.

- Q: Should we show "trend" or "status" columns?
  A: Show placeholder columns for MVP (dashes). Trend would require historical data; that comes in Phase 2 when #4105 (stable-wins tracking) lands.

- Q: Why `docs/project/status/` vs `docs/project/metrics/`?
  A: Consistency with existing subsystem status files (parser.md, lsp.md, tests.md). All are in the same directory for unified dashboard.

## Historical Context

The scout report (issue body) identified almost all measurement infrastructure as already available. The build was deferred because the cargo mutants output format needed accuracy-scout verification (it did). This MVP focuses on the lowest-lift, highest-value metrics (test counts, mutation scores) before moving to more complex tracking (flaky tests, memory profiling).
