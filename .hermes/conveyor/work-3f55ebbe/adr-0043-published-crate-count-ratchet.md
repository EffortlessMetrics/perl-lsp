# ADR-0043: Published Crate Count Ratchet Gate

**Status**: Accepted
**Date**: 2026-04-19
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0041](./0041-microcrate-collapse.md) (Microcrate Collapse), [Issue #4416](https://github.com/EffortlessMetrics/perl-lsp/issues/4416)

## Context

Following the microcrate collapse (ADR-0041), the workspace will publish approximately 30-31 crates instead of the original 132. The collapse reduces the published surface via a series of waves. As each wave lands, the count in `[workspace.metadata.publish.allow]` decreases.

To prevent regression—accidentally adding crates back to the allowlist without architectural review—a ratchet gate is needed. This gate monitors the published crate count and prevents it from increasing above the established baseline.

### Why a ratchet (not a fixed target)?

The count will decrease over time as collapse waves land. A fixed target (e.g., "must equal 30") would fail during the entire transition period. A ratchet (e.g., "must not exceed current baseline") allows the ceiling to tighten automatically as waves land, but never loosens.

## Decision

We implement `cargo xtask published-crate-count` as a file-based ratchet gate:

1. **Reads `[workspace.metadata.publish.allow]`** via `cargo metadata --no-deps` to get the current published crate count.

2. **Reads the baseline from `xtask/published-crate-baseline.txt`** — a single integer file committed to the repo.

3. **Comparison behavior**:
   - `current > baseline` → **FAIL** (gate blocks merge; clear remediation message printed)
   - `current < baseline` → **INFO** + auto-write new baseline (ratchet tightens automatically)
   - `current == baseline` → **PASS** silently

4. **Baseline file is the source of truth** for the allowed count. Each collapse wave PR that reduces the count will see the baseline tighten automatically; the diff is committed as part of that wave.

### Implementation

- Module: `xtask/src/tasks/count_ratchet.rs`
- CLI: `cargo xtask published-crate-count`
- Baseline file: `xtask/published-crate-baseline.txt` (format: single integer with trailing newline)
- No `--update` flag needed — the ratchet auto-tightens when count decreases

### CI Integration

The gate is wired via `just ci-published-crate-count` and should be added to `.ci/gate-policy.yaml` under the appropriate tier with `quarantine: true` until the collapse completes.

**Initial baseline value**: 98 (matching the count at time of implementation).  
**Current count at time of writing**: 81 (post-Wave C/D collapses).  
**Post-collapse target**: ~30-31 (per ADR-0041 as amended).

## Alternatives Considered

### Option 1: Hardcoded constant in source code

A constant `PUBLISHED_CRATE_TARGET: u32 = 31` in the xtask module would avoid a separate baseline file.

**Pros**:
- Single source of truth (source code)
- No file drift risk
- Easier to verify in code review

**Cons**:
- Updating the target requires a code change + review + merge cycle
- The actual post-collapse count is uncertain during transition; waves may reveal the target should be slightly different
- Deviates from established `parser_corpus_sweep` pattern (uses `--baseline` file)

**Verdict**: Rejected. The file-based approach allows the baseline to be updated by the xtask itself when count decreases (no code change needed), and matches the existing ratchet pattern.

### Option 2: Hardcoded constant with `--update` flag to modify source

The `--update` flag would update the constant value in source code via text replacement.

**Pros**:
- Keeps constant in source
- Allows automated baseline updates

**Cons**:
- Modifying source via CLI is unusual and could cause git history confusion
- More complex implementation
- Same uncertainty problem as Option 1

**Verdict**: Rejected. File-based baseline is simpler and matches established patterns.

### Option 3: No ratchet (trust process)

Rely on code review and architectural discipline to prevent allowlist creep.

**Pros**:
- No tooling overhead
- Flexibility to add crates when genuinely needed

**Cons**:
- Human discipline fails; the 132→30 collapse exists because human discipline alone couldn't prevent microcrate creep
- ADR-0041 explicitly calls for this ratchet as a mitigation

**Verdict**: Rejected. The ADR-0041 decision requires automated enforcement.

## Consequences

### Positive

- **Permanent guard against regression**: The published count cannot increase without a deliberate baseline update in a reviewed commit.
- **Auto-tightening during collapse**: Each wave automatically tightens the ceiling; no manual target updates needed.
- **Clear failure message**: When the gate fails, it prints exactly what happened and how to fix it (either remove the crate from allowlist or intentionally update baseline).
- **Simple implementation**: Uses existing `run_cargo_metadata()` utility; no new dependencies.

### Negative

- **Baseline file must be committed**: The auto-tightening behavior writes to `xtask/published-crate-baseline.txt`, which must be committed. This is intentional — the baseline change is part of the collapse wave's git history.
- **Quarantine needed during transition**: Until collapse completes (~30-31 crates), the gate should be in `quarantine: true` mode in gate-policy.yaml to avoid blocking merges during the transition.

### Neutral

- **Count includes all allowlist entries**: The gate counts all entries in `workspace.metadata.publish.allow`, including non-`perl-` prefixed crates like `tree-sitter-perl-c` and `tree-sitter-perl-rs`. This is correct behavior — the allowlist is the source of truth.
- **Ratchet never fires during collapse**: Because the count only decreases during collapse waves, the ratchet would never fail during transition. It becomes meaningful only after collapse completes.

## Implementation Notes

The implementation in `count_ratchet.rs` uses:
- `serde_json::from_slice` to parse `cargo metadata --no-deps` output (no `cargo_metadata` crate dependency)
- `run_cargo_metadata(true)` where `true` = `--no-deps` flag
- Standard `fs::read_to_string` / `fs::write` for baseline file I/O
- Unit tests for `check_count`, `parse_baseline`, and `write_baseline` functions

The gate is currently wired into `justfile` via `ci-published-crate-count` recipe. Formal CI integration via `.ci/gate-policy.yaml` is pending (see work item `work-3f55ebbe`).
