# ADR-2026-001: Engineering Health Scorecard — Per-Subsystem Stratification

## Status
Proposed

## Context

GitHub issue #4070 requests expanding the perl-lsp engineering-health scorecard with
per-subsystem breakdown of existing metrics. The scorecard currently shows aggregate
numbers (total Tier A tests, global mutation count, aggregate latency) but provides
no visibility into per-subsystem health (parser / semantic / lsp / dap / workspace /
module-resolution).

Three prior agents (research, verification, plan review) established:

1. **Existing infrastructure is sound**: `xtask/src/tasks/update_status/` is a
   Rust-native status generator with 7 subsystem files (`lsp.md`, `tests.md`,
   `parser.md`, `quality.md`, `editor_ux.json`, `workspace.md`, `dap.md`). The
   `just status-update` / `just status-check` anti-drift workflow is production-ready.

2. **cargo-mutants cannot produce per-crate mutation scores**: The `mutants.json`
   output contains only listed mutants (no `status` field). The `outcomes.json`
   contains aggregate killed/total counts only at workspace level, not per-crate.
   Per-crate mutation scores are not available without running `cargo mutants` 124
   times (once per crate) or upstream cargo-mutants changes.

3. **Three taxonomy systems conflict**: The issue's 6 subsystems, the
   `StatusSubsystem` enum's 6 variants, and `benchmark-thresholds.yaml`'s 4
   categories (parser/lexer/lsp/index) do not align. A resolution is required.

4. **Phase 3 would duplicate existing infrastructure**: The plan proposed creating
   `.ci/flaky-tests.json` but `.ci/debt-ledger.yaml` already has `flaky_tests: []`
   with a production-quality schema (name, added, issue, tier, quarantine_days,
   expires, owner, notes, failure_pattern, affected_platforms) and gate integration
   (`just debt-report` / `just debt-check`).

5. **Phase 5 is a separate project**: "Release smoke pass rate across supported
   editors × platforms" implies multi-editor plugin test infrastructure that does
   not exist. The current `lsp-smoke` is a single-threaded deterministic test.

## Decision

### 1. Authoritative Subsystem Taxonomy: `StatusSubsystem` Enum

Use `xtask/src/tasks/update_status/mod.rs`'s `StatusSubsystem` enum as the
authoritative taxonomy. It is the only subsystem system with:
- A Rust type (compile-time enforcement)
- CLI parsing support (`--only quality`)
- 7 mature subsystem files

The issue's 6 subsystem names (parser / semantic / lsp / dap / workspace /
module-resolution) map to the enum variants via a new configuration file:

**`.ci/subsystem-mapping.yaml`** (new file):
```yaml
# Maps issue terminology → StatusSubsystem enum variants
# module-resolution has no dedicated variant; resolution lives in perl-semantic-analyzer
parser: Parser
semantic: Quality   # NOTE: Quality tracks mutation+UX, not semantic analysis
lsp: Lsp
dap: Dap
workspace: Workspace
module-resolution: Quality  # Closest match; resolution lives in perl-semantic-analyzer
```

**Consequence**: "semantic" → `Quality` is a semantic mislabeling (Quality tracks
mutation testing + UX, not semantic analysis). This is documented but accepted as
a necessary compromise — changing the enum variant name would break the existing
CLI and 7 subsystem files.

### 2. Primary Granularity: Crate-Level, Not Subsystem-Level

Store all metric data at **crate granularity** (124 workspace members, unambiguous
Rust packaging boundary). Subsystem aggregation becomes a **configuration-driven
view layer** via `.ci/subsystem-mapping.yaml`.

**Why**: Subsystem boundaries are inherently ambiguous (e.g., `perl-semantic-analyzer`
serves both "semantic" and "module-resolution"). Crate-level data is unambiguous and
can be re-aggregated into subsystems without re-running data collection.

**Example**: Instead of `collect_per_subsystem_mutation()`, keep
`collect_per_crate_mutation()` which returns `BTreeMap<String, usize>` (crate →
mutant count). Subsystem view is computed at render time by looking up each crate
in `.ci/subsystem-mapping.yaml`.

### 3. Mutation Data: Counts Only, Not Scores

**Decision**: Accept that the current cargo-mutants output provides **mutant counts**
(listed), not **mutation scores** (killed/total).

| What | Available | Notes |
|------|-----------|-------|
| Workspace-level aggregate score | Yes | From `outcomes.json` |
| Per-crate mutant count | Yes | From `mutants.json` |
| Per-crate mutation score | **No** | Requires per-crate cargo-mutants runs (hours of compute) or upstream cargo-mutants changes |

**Action**: Keep `collect_per_crate_mutation()` as-is. The column in `quality.md`
already says "Mutants listed" (not "Mutants killed"), which is accurate. Document
in the code and `quality.md` that per-crate mutation scores require a future
enhancement.

### 4. Flaky Test Tracking: Extend Debt-Ledger, Not Parallel File

**Decision**: Use `.ci/debt-ledger.yaml`'s existing `flaky_tests: []` array as the
canonical store. Do **not** create `.ci/flaky-tests.json`.

Extend the existing entry schema with two new optional fields:
```yaml
flaky_tests:
  - name: "lsp::test_completion_timeout"
    failure_count: 3      # NEW: running count of failures
    last_failed_at: null  # NEW: ISO8601 of most recent failure
    added: "2026-01-24"
    issue: "#198"
    tier: "quarantine"
    quarantine_days: 14
    expires: "2026-02-07"
    owner: "maintainer-username"
    notes: "Timing-dependent, needs server init fix"
    failure_pattern: "timeout waiting for completion"
    affected_platforms:
      - "windows"
      - "wsl"
```

Add a Python updater script in `.ci/scripts/update-flaky-tracker.py` that:
- Runs after test suite (post-hook in `just test-full`)
- Detects flaky failures (tests that fail on retry but pass on re-run)
- Updates `failure_count` and `last_failed_at` in `.ci/debt-ledger.yaml`
- Is **not** a merge gate (informational only)

**Why not Tier A**: Flaky tracking is informational. Quarantining a flaky test
removes it from the merge gate; tracking it separately is for visibility.

### 5. Latency Metrics: Read from `benchmarks/results/latest.json`

**Decision**: Phase 2 reads latency data from `benchmarks/results/latest.json`,
produced by `cargo xtask bench-run --output benchmarks/results/latest.json`.

Confirmed via `xtask/src/tasks/benchmarks.rs` line 87:
```rust
let output_path = output.unwrap_or_else(|| root.join("benchmarks/results/latest.json"));
```

The `format_benchmarks()` function reads this file. The results are aggregated by
subsystem category (parser/lexer/lsp/index) using `.ci/benchmark-thresholds.yaml`
category definitions.

**Phase 2 implementation**: Add a `PERFORMANCE_BY_SUBSYSTEM` marker block to
`quality.md` that reads `benchmarks/results/latest.json` and aggregates timing data
by category using `benchmark-thresholds.yaml` categories.

**Memory metrics** (AST cache size, document store size, peak RSS) are **out of
scope** for this change. The issue title mentions "latency/memory" but memory
metrics have no existing collection infrastructure and would require a separate
implementation spike.

### 6. Phase 5 Removed: Release Smoke Dashboard

**Decision**: Remove "Release smoke pass rate across editors × platforms" from
this change entirely. File as a separate GitHub issue.

**Why**: The current `lsp-smoke` is a single-threaded deterministic test. Multi-editor
smoke testing (VSCode + Neovim + Emacs × Linux + macOS + Windows) requires:
- Editor plugin test infrastructure for each editor
- Platform-specific CI runners
- Results aggregation system
This is 3-6 months of CI infrastructure work minimum, not a metrics instrumentation
task.

## Consequences

### Benefits

1. **Preserves existing anti-drift workflow**: All changes extend `xtask/src/tasks/update_status/` — `just status-update` / `just status-check` continue to work unchanged.

2. **Avoids duplicate tracking systems**: Reuses `.ci/debt-ledger.yaml` for flaky tests instead of creating a parallel file with different schema.

3. **Crate-level primary granularity is unambiguous**: No disputes about which subsystem a crate belongs to. The `.ci/subsystem-mapping.yaml` is explicit and editable without code changes.

4. **Achievable scope**: Phases 1-4 are implementable. Phase 5 is deferred to a properly scoped follow-up issue.

### Tradeoffs

1. **"Quality" mislabeling persists**: `StatusSubsystem::Quality` tracks mutation
   testing + UX scenarios, not semantic analysis. The issue's "semantic" subsystem
   maps to "Quality" as the closest available variant.

2. **Mutation scores unavailable without CI investment**: Per-crate mutation scores
   require 124 × cargo-mutants runs or upstream cargo-mutants changes. The scorecard
   will show mutant **counts** until one of those happens.

3. **Memory metrics deferred**: The issue mentions "latency/memory" but memory
   metrics have no collection infrastructure. This is deferred to a follow-up.

4. **`flaky` mapped to `brokenpipe` in categorization**: The
   `ignored_tests.rs` categorization folds `#[ignore = "flaky:..."]` into
   "brokenpipe". The flaky tracker (`.ci/debt-ledger.yaml`) is a separate system
   for observability, not for ignore categorization.

## Alternatives Considered

### A: Adopt the issue's 6-subsystem taxonomy as authoritative

**Rejected**: The issue's taxonomy (parser / semantic / lsp / dap / workspace /
module-resolution) has no Rust type, no CLI support, and no existing infrastructure.
Introducing it would create a third taxonomy competing with the `StatusSubsystem`
enum and `benchmark-thresholds.yaml` categories.

### B: Create new `subsystem` field on all workspace crates

**Rejected**: Requires modifying all 124 `Cargo.toml` files. The mapping would
be intrinsic to the source code, not configurable. Changes to subsystem boundaries
would require code changes and PR reviews.

### C: Per-crate cargo-mutants runs for mutation scores

**Rejected (for now)**: Running `cargo mutants` per-crate (124 separate runs) would
take hours of compute time per run. Not feasible for merge-gate or even nightly
CI. Revisit if cargo-mutants adds per-crate output modes.

### D: Create `.ci/flaky-tests.json` as the plan proposed

**Rejected**: The `.ci/debt-ledger.yaml` already has a production-quality schema,
gate integration (`just debt-report` / `just debt-check`), and weekly summaries.
Creating a parallel file with a different schema means two files to maintain and
no gate integration for the new data.
