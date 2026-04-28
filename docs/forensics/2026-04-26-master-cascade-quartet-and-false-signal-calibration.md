# Master Cascade Quartet + False-Signal Calibration

**Window**: 2026-04-26 session, ~02:00–06:00 EDT
**Audience**: orchestrator, anyone running a master-bit-rot scout, anyone tempted to cascade-update a "shared" failure
**Purpose**: capture the four genuine master-cascade fixes shipped this session (#6789, #6803, #6807, #6810), the 3:1 false-vs-real bit-rot signal ratio, and the per-crate-vs-workspace gap that produces the recurring fmt cascade.

---

## The four genuine master fixes

Each fix unblocked ~30+ blocked PRs via cascade-update.

| PR | File fixed | Fix shape | Why it broke master |
|----|-----------|-----------|---------------------|
| **#6789** | `crates/perl-parser/src/incremental/incremental_checkpoint.rs` | `cargo fmt` 5-line touch-up at lines 268, 298, 741, 950, 1006 | Pre-edition-2024 formatting introduced by recently-merged #6690; per-crate fmt passed but workspace-wide xtask fmt failed |
| **#6803** | `crates/perl-pragma/tests/comprehensive_unit_tests.rs` | `cargo fmt` 6-line touch-up at lines 1239, 1246 | Test additions from #6351/#6333 with extra blank line + multi-line `vec![]` that should fit on one line |
| **#6807** | `crates/perl-semantic-analyzer/tests/semantic_pipeline_fuzz_tests.rs` | `cargo fmt` 1-line touch-up at lines 3, 9 (use-import ordering) | rustfmt's group-ordering rule wants direct-module imports before submodule imports; previous merge violated this |
| **#6810** | `crates/perl-parser/src/incremental/incremental_checkpoint.rs` | `///` blank line + `# Implementation notes` heading inserted at line 212 | `clippy::doc_lazy_continuation` — only surfaces on workspace-wide `cargo clippy --workspace --lib`, not on `-p perl-parser --lib` |

Each was a narrow fix (≤6 lines in 3 of 4 cases). Each unblocked ~30 PRs. Net: ~120 PR-equivalents of cascade-unblock from ~14 lines of master-side fix work.

---

## The structural cause — per-crate vs workspace gap

Most CI gates run per-crate:

- `cargo fmt -p <crate> --check`
- `cargo clippy -p <crate> --lib`
- `cargo test -p <crate>`
- `cargo build -p <crate>`

Master breakage is workspace-wide. The two scopes don't align:

- A PR can pass per-crate fmt but introduce drift that only `cargo xtask fmt` (workspace-wide, per-crate iteration) exposes
- A PR can pass per-crate clippy but introduce a `doc_lazy_continuation` warning that only `cargo clippy --workspace --lib -- -D warnings` exposes
- The xtask fmt aborts at first failing crate — so when one crate's drift is fixed, the next falls into the visible position. This is the "cascade" pattern: each fmt fix exposes the next.

The structural fix lands in PR #6808 + #6811: ops master-green protocol now requires workspace-wide CI verification before merge, not just per-crate. Per the directive: **keep master green and require green to merge**.

The other half of the prevention: PR Smoke and Compile All Targets (bit-rot guard) gates exist precisely to catch workspace-wide breakage at PR time — but the agents that route on those gates' results sometimes misread their failures (see the false-cascade calibration below).

---

## The 4-fmt-fix cascade in sequence

These four fixes had to land sequentially because xtask fmt aborts at the first failing crate. The order was:

1. **#6789** lands → `xtask fmt --check` now exits at the second failing file (perl-pragma)
2. **#6803** lands → exits at the third (semantic-analyzer)
3. **#6807** lands → finally clean for fmt
4. **#6810** lands → clean for the parallel clippy lint cascade (different gate, different lint, surfaced once others were unblocked)

Each fix exposed the next. Without serializing the fixes, the picture would have looked like "all 30+ PRs still failing" until ALL fixes landed. Per the operating tempo, each fix landed within ~10 minutes of detection — this is the right cadence for narrow master fixes.

---

## The 3:1 false-vs-real bit-rot signal ratio

Of four "master bit-rot signals" investigated this session, **three were false** (3:1 false:real ratio).

| Signal | Verdict | Actual cause |
|--------|---------|--------------|
| Compile All Targets failures on #6246, #5985, #5881 | **FALSE** | 3 different per-PR errors in different files that looked similar at the aggregator level. PR-side bugs each, not master regression. |
| Windows Guardrails (module-separator-regressions) failures on #6006, #5559 | **FALSE** | Stale CI from PRE-rebase state. Master fix already in via #5593 (commit 9a2304e37). The signal was reading old comments instead of current statusCheckRollup. |
| `incremental_parsing_benchmarks.rs` Compile All Targets failures on #5513, #5509, #5502 | **FALSE** | Fresh-root stranded PRs. Compile errors were in `xtask/src/tasks/metrics/lsp_stats.rs` (orphaned intermediate state from pre-rebuild master), not master-side bit-rot. Cascade-update cannot fix; cherry-pick recovery is the playbook. |
| `perl-pragma/tests/comprehensive_unit_tests.rs` PR Smoke failure | **REAL** | Genuine fmt drift introduced by #6351/#6333 test additions. Fixed by #6803. |

**Pattern**: false signals outnumber real ones at this stage of the methodology, and the false signals look identical to real ones at the aggregator level (statusCheckRollup, label-based queries). The "3+ PRs failing identically" detection rule needs strengthening.

---

## Calibration update for the bit-rot detection rule

Prior calibration (per `feedback_master_bit_rot_recurrence_pattern.md`): "3+ PRs failing identically on the same gate = master signal."

Updated 2026-04-26 calibration: the rule produces 3 false positives for every 1 true positive at this stage. Refine to:

**Master bit-rot signal** requires ALL THREE of:

1. 3+ PRs failing the SAME gate with the SAME test name(s) on their LATEST CI run (latest-per-check filter, per `feedback_status_check_rollup_stale_entries.md`)
2. The failure REPRODUCES on a fresh master clone with the same command (`cargo xtask fmt --check`, `cargo clippy --workspace --lib -- -D warnings`, etc.)
3. The PRs DO NOT touch the failing file (rules out per-PR causation)

Add an exclusion: if the affected PRs share NO merge-base with current master, it's a fresh-root strand, not bit-rot — different playbook (cherry-pick recovery, not master fix).

The bit-rot scout agent should always:

- Pull fresh master and reproduce locally before pushing a fix PR
- Check merge-base via `git merge-base origin/master <pr-head>` to rule out fresh-root strand
- Verify the failing file isn't in the PR's own diff (`gh pr diff <num> --name-only | grep <failing-file>`)

These three checks would have caught all three false signals this session at near-zero cost.

---

## The recurring nature of fmt drift

Late-session observation from the pr-respond sweep: even after #6789 + #6803 + #6807 landed, master continues to develop new fmt drift in the SAME files as new merges add code. Specifically flagged:

- `crates/perl-pragma/tests/comprehensive_unit_tests.rs:1257` (different line than #6803's fix at 1239) — needs blank line before next `#[test]`
- `crates/perl-parser/src/incremental/incremental_checkpoint.rs` lines 268, 298, 741, 950, 1006 — possibly RE-introduced after #6789's fix (or never fully clean)
- `crates/perl-semantic-analyzer/tests/semantic_pipeline_fuzz_tests.rs` use-stmt ordering — possibly re-introduced after #6807

This is a continuous pattern, not a one-time cleanup. The structural fix would be:

- A pre-commit hook that runs `cargo xtask fmt --check` on the changed files (not just per-crate)
- Or a CI gate that fails on workspace-wide xtask fmt drift (PR Smoke does this, but the gate's failures get misclassified — see the false-signal calibration above)
- Or stricter pre-merge enforcement in ops (landed in PR #6811: ops requires workspace-wide CI SUCCESS before merge)

Until one of these structural fixes is fully reliable, expect a recurring need for narrow master fmt fixes — probably 2-4 per high-throughput session at current cadence. Plan for them; don't be surprised by them.

---

## Operational implications

- **Master fmt fix is a recurring pattern, not exceptional.** Treat it as routine ops, not crisis response.
- **Each fix is small (~5-15 lines).** The cost is detection + dispatch, not the fix itself.
- **The cumulative impact is huge** (~30 PRs unblocked per fix). Investing 10 minutes per fix pays back across the next merge wave.
- **The false-signal problem is the actual bottleneck.** Before dispatching a master-fix builder, verify on fresh master locally — saves hours when the signal is false.
- **Cascade-update is the right unblock IFF master is fixed.** Cascade-update against unfixed master just delays the failure; verify master green before sweeping.

---

## Related forensics + memory entries

- `feedback_master_bit_rot_recurrence_pattern.md` — the "3+ PRs failing identically" detection rule (this doc updates the calibration)
- `feedback_master_bit_rot_cascade_fixes.md` — narrow fix → admin-merge → cascade pattern
- `feedback_xtask_fmt_false_cascade.md` — the original false-signal pattern at the fmt gate (this doc generalizes to compile + clippy + Windows gates)
- `feedback_status_check_rollup_stale_entries.md` — latest-per-check filter requirement
- `feedback_master_bitrot_cascade_8plus_pattern.md` — 8+ master fixes per cluster-merge session pattern (matches this session's count)
- `2026-04-26-sign-off-is-routing-methodology-strengthening.md` — companion doc on the methodology fix that prevents bypassing these signals
- `2026-04-25-defense-in-depth-verification-architecture.md` — the broader gate framework these fixes operate within
- PR #6789, #6803, #6807, #6810 — the four landed fixes
- PR #6808, #6811 — the methodology layer that enforces master-green-required-for-merge

---

## Applies to

Reference this doc when:
- Investigating any "shared" CI failure across multiple PRs
- Considering whether to dispatch a master-fix builder
- Sizing operational expectations for a high-throughput merge session (expect 2-4 master fmt fixes per session at current cadence)
- Designing the next pre-commit or CI hook to prevent the recurring fmt drift pattern
- Calibrating the master-bit-rot scout agent's detection threshold
