# Context — Scorecard Metrics Ratchet Infrastructure (#4105)

## Problem Statement

Each scorecard (parser, diagnostics, editor-intelligence, workspace, module-resolution, DAP, engineering-health) needs a floor-metric enforcement layer that:

1. **Layer 1**: Define a committed baseline file (`.ci/metrics/baselines/<subsystem>.json`) with floor metrics (things that must not regress)
2. **Layer 2**: Check floor metrics on every PR via `check_floor_metrics()` — fail if regression exceeds tolerance
3. **Layer 3**: Track multi-run stability — only re-baseline improvements when they've been stable for N consecutive runs
4. **Layer 4**: Link issues to scorecards via labels — enable automatic issue-to-metric correlation

## Key Decisions

### Layer 1: Baseline File Format

**File location**: `.ci/metrics/baselines/<subsystem>.json` (committed, not generated)

**Content**: Serde-friendly JSON with three sections:
- `floor_metrics`: BTreeMap<String, Option<f64>> — the safety floor (e.g., clean_rate, crash_count)
- `improvement_metrics`: BTreeMap<String, Option<f64>> — tracked but not enforced (e.g., node_kind_coverage)
- `tolerance_pct`: f64 — wiggle room for noisy metrics (default 0.5% for corpus-size metrics, 0% for crash counts)

Null values in either map mean "not yet instrumented" and are silently skipped (no violation).

### Layer 2: Ratchet Check Logic

**Naming convention for metric directions** (critical for builder):
- Metrics ending in `_count`, `_nodes`, `_unreadable` are "lower-is-better" (e.g., crash_count=0, files_unreadable=48)
- All others are "higher-is-better" (e.g., clean_rate=0.971, pass_rate=1.0)

If a metric doesn't fit this pattern (e.g., `latency_p95_ms` is lower-is-better), the builder should add a `lower_is_better: Vec<String>` field to `SubsystemBaseline` rather than extending the suffix heuristic.

**Violation detection**:
```
is_violation = {
  lower_is_better: regression_pct > tolerance_pct && current > baseline
  higher_is_better: regression_pct > tolerance_pct && current < baseline
}
```

**Regression percentage**:
```
for lower_is_better: (current - baseline) / baseline.max(1.0)
for higher_is_better: (baseline - current) / baseline.max(epsilon)
```

### Layer 3: Stable-Wins Tracking

**Problem this solves**: Corpus-size metrics naturally oscillate ±2% due to discovery ordering and filesystem timing. A single improvement doesn't mean permanent progress. The stable-wins tracker records N-run windows and only approves re-baselining when improvements are consistent across N consecutive runs (N=3 per acceptance criteria).

**Storage**: `target/metrics/stable_wins/<subsystem>.json` (gitignored, ephemeral within a run)

**State structure**:
```rust
pub struct StableWinsState {
    pub subsystem: String,
    pub recent_runs: BTreeMap<String, Vec<MetricRun>>,
}

pub struct MetricRun {
    pub commit: String,
    pub value: f64,
    pub timestamp: String,
}
```

Each metric name maps to a circular buffer of recent runs. The buffer keeps `STABLE_WIN_THRESHOLD + 1` entries (3+1=4) to provide context.

### Layer 4: Scorecard Labels

**Purpose**: Link issues to the scorecards that measure them. Enables automation to answer "which issues improve this scorecard?"

**Label naming**: `scorecard/<subsystem>` (7 total)
- `scorecard/parser` — measured by parser scorecard (#4063)
- `scorecard/diagnostics` — measured by diagnostics scorecard (#4065)
- `scorecard/editor-intelligence` — measured by editor scorecard (#4066)
- `scorecard/module-resolution` — measured by module resolution scorecard (#4069)
- `scorecard/workspace` — measured by workspace scorecard (#4070)
- `scorecard/dap` — measured by DAP scorecard (#4071)
- `scorecard/engineering-health` — measured by engineering health scorecard (#4070 phase 2)

**Retroactive application**: Apply to existing issues that are anchors for each scorecard (e.g., #3496, #3499 get `scorecard/parser` because they're parser regression guards).

## Coordination with #4063 (Parser Scorecard)

#4063 adds **new measurement infrastructure** (error density, recovery salvage rate) via corpus audit.
#4105 adds **generalized ratchet enforcement** for any subsystem baseline.

**No conflict**: Different layers of abstraction.

**Coupling point**: The `target/receipts/metrics/parser.json` file format (defined by this spec, emitted by #4063's `parser-stats --json` command) bridges the two. If #4063 lands first, its output feeds directly into the ratchet check. If #4105 lands first, the fallback reads from existing `.ci/parser-corpus-baseline.json` until #4063 lands.

**Recommend sequencing**: Land #4105 first (generalized infrastructure), then #4063 (parser-specific measurements).

## Alternatives Considered and Rejected

| Alternative | Why rejected |
|-------------|-------------|
| Hand-edit baseline files on every merge | Error-prone, no audit trail, violates principle of computed metrics (CLAUDE.md) |
| Hard-code tolerance_pct in check logic | Different subsystems have different noise characteristics (parser corpus varies ±2%, mutation scores are stable) — tolerance should be configurable per baseline |
| Single `metrics.json` file with all subsystems | Would create a rebase nightmare; multiple PRs touching the same file → merge conflicts. One file per subsystem keeps changes isolated |
| Promote-baseline automatically on re-run | Risk: noisy upward spike gets locked in as floor. Manual approval via `promote-baseline` command with N-run stability check is safer |
| Store stable-wins in `.ci/` (committed) | Accumulates stale history; committed state grows unbounded. `target/` (gitignored) is correct for ephemeral metric history |
| Use Prometheus or external metrics backend | Adds operational complexity; keep metrics in-repo for auditability. This is a scoreboard for a single project, not multi-tenant infrastructure |

## Testing Strategy

**Unit tests** (in `cargo test -p xtask`):
- Load baseline JSON (round-trip Serde)
- Detect violations correctly on synthetic current-metrics
- Tolerance band prevents false positives on small drifts
- Null metrics are skipped (no error, no violation)
- Stable-improvements tracking across N runs

**Integration tests** (manual, after implementation):
- `cargo xtask metrics ratchet-check parser` exits 0 on master
- Synthetic current-metrics with regression triggers exit 1
- Synthetic current-metrics within tolerance triggers exit 0
- `--record` flag writes stable-wins state
- `promote-baseline` finds stable improvements after 3+ runs

**Gate verification**:
- `just ci-metrics-ratchet` succeeds
- `just pr-fast` succeeds with no-op baseline check
- `just ci-gate` includes ratchet

## Verification Path

1. **Baseline files present**: `ls .ci/metrics/baselines/`
2. **Code compiles**: `cargo build -p xtask`
3. **Commands available**: `cargo xtask metrics --help`
4. **Unit tests pass**: `cargo test -p xtask -- metrics::`
5. **Clippy clean**: `cargo clippy -p xtask -- -D warnings`
6. **Ratchet check works**: `cargo xtask metrics ratchet-check parser` (exit 0 on master)
7. **Gates wired**: `just ci-metrics-ratchet`, `just pr-fast`, `just ci-gate`
8. **Labels created**: `gh label list --json name`

## Scope Exclusions

- Does NOT implement per-crate breakdown of metrics (that's #4070 — engineering health scorecard)
- Does NOT add new metrics (all floor metrics come from existing baselines, improvement metrics from existing instrumentation)
- Does NOT change the parser corpus sweep or CPAN ratchet logic (those remain as-is, serve as reference implementations)
- Does NOT wire mutation scoring or latency metrics to the ratchet yet (#4070 phase 2)
- Does NOT auto-publish baseline diffs to CI logs (that's #4070 dashboard work)

## Downstream Impact

- All subsequent scorecard PRs (#4063, #4065, #4066, #4069, #4070, #4071) depend on this infrastructure
- Builders for those issues will reference the baseline loader and ratchet check as their foundation
- CI gates will fail cleanly if a PR regresses floor metrics (operator sees "system_clean_rate dropped below floor")

## Open Questions Resolved

- Q: Should tolerance be per-metric or per-subsystem?
  A: Per-subsystem. Different subsystems have different noise characteristics, but within a subsystem it's uniform.

- Q: Should violations print detailed diffs or just fail?
  A: Print each violation (metric name, baseline, current, regression %) so operator can investigate.

- Q: Should Layer 3 (stable-wins) block CI, or just advise?
  A: Just advise. It's used by the `promote-baseline` command (manual decision), not by the floor-check gate.

- Q: Why not merge Layer 2 and Layer 3 into one pass?
  A: Separate concerns. Layer 2 (floor enforcement) runs on every PR and must be fast. Layer 3 (stability tracking) is optional and runs with `--record` flag. Keeping them separate keeps the critical path lean.

## Historical Context

This issue grew out of #4077's discovery that the parser's existing `enforce_ratchet()` logic was overly sensitive to corpus variance. The plan-reviewer's extensive section on "Edge cases" in the issue body documents the real operational pain points that this spec addresses.

The floor vs improvement distinction (Layer 1) comes from observing that some metrics should block PRs (crashes must not increase) while others are just tracked for trending (error_density is interesting but not blocking).
