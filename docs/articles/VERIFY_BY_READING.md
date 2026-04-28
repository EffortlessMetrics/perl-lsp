# Verify By Reading: Every Prior Comment Is a Hypothesis

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp
**Cross-references**: [VERDICT_OVERRIDE_PATTERNS.md](../contributing/VERDICT_OVERRIDE_PATTERNS.md), [WHEN_RECEIPTS_LIE.md](WHEN_RECEIPTS_LIE.md)

---

## TL;DR

Prior agent comments, reviewer sign-offs, scout findings, external AI advisories, and even tool return values are **hypotheses about state at the time they were written** — not authoritative statements about current state. Trusting them without verification is a specific failure mode that accumulates silent drift.

On the 2026-04-19 Wave G1 session, five distinct manifestations of this failure mode surfaced — all recoverable, all caught by verifying against current state rather than trusting prior reports. The hardening principle that emerged: **when the cost of re-reading is low, re-read.**

---

## The Five Incidents

### 1. External AI advisories, stale by minutes

Two external AI advisories were pasted into the session by the user. Both referenced PR state from 30+ minutes earlier — PRs that had merged by the time the advisory was written. The external AI's page cached the GitHub state; our actions didn't propagate back to it. The advisory's recommendations ("finish PR #4504") referred to already-merged work.

**Recovery:** `gh pr list --state open` before acting on external advice. 3 seconds; catches the staleness every time.

### 2. Agent comment cites old SHA

Accuracy-scout on #4501 read the original scout's comment from before Wave G1a merged. The original said "no inter-provider dependencies among G1b crates." Post-G1a, the actual dependency structure had shifted — G1b crates that used to depend on G1a-absorbed crates now depended on `perl-lsp-rs-core`. Accuracy-scout caught the shift by re-reading the live `Cargo.toml` files and wrote a corrected dependency map in its verification comment.

**Recovery:** built into the accuracy-scout skill. Every claim is re-verified against current master, not against the issue body's snapshot.

### 3. Tool return value says "Updated"; state reverted

`TaskUpdate` on the internal task-tool returned `"Updated task #N status"` on every call. But subsequent `TaskGet` returned the old value unchanged. Orchestrator trusted the return value for ~15 attempts before verifying with `TaskGet` and realizing the updates never persisted.

Root cause is harness-backend (tracked as issue #4509), but the hardening lesson is process-level: **tool success reports ≠ state change.** When the cost of verifying is small (one extra tool call), do it.

**Recovery:** after each `TaskUpdate`, call `TaskGet` to verify. Or accept that the tool is broken and stop using it (which is what happened — orchestrator moved to GitHub labels + `git log` as the real state source).

### 4. Prior diff-auditor marked "CLEAN"; bit-rot shipped

Wave F (PR #4493) merged on 2026-04-18. Its diff-auditor pass returned CLEAN. But two integration test files — `crates/perl-lsp-protocol/tests/comprehensive_unit_tests.rs` and `crates/perl-dap/tests/security_dap_path_traversal_hardened_tests.rs` — contained references to absorbed crates. The bit-rot shipped on master.

Why diff-auditor missed it: push-triggered CI runs `cargo test --workspace --lib`, not `--all-targets`. Integration tests under `crates/*/tests/` never compiled on CI. Diff-auditor inspected the diff but didn't try to compile it against the other tests. This was caught on 2026-04-19 when an orchestrator happened to run `cargo check --workspace --tests` manually, discovered the bit-rot, and filed issue #4502 → merged as PR #4503.

The "CLEAN" verdict was locally correct — the diff was coherent and scope-clean. But it was bounded by what diff-auditor looked at. That boundary wasn't written on the verdict.

**Recovery:** bounded verdicts must name their bounds. "CLEAN within scope X" is more honest than "CLEAN." Future diff-auditor skill should explicitly state what was checked and what wasn't.

### 5. Ops report "main checkout clean on master"; main was on temp branch

Multiple ops merge agents reported post-merge status as "main checkout clean on master." Each report was accurate at the time the agent wrote it — the agent had done `git checkout master` before exiting. But between reports, **other non-isolated agents had switched main to a temp working branch** (a known issue, #4342). The orchestrator, trusting the last ops report, was surprised to find main on `temp-fmt` branch later.

Agents report what they **intended**, not what persists through subsequent environment changes. The state they verified is only valid for that instant.

**Recovery:** verify main's branch before any branch-sensitive operation. `git branch --show-current` is free; use it.

## The Principle

**Verify-by-reading.** Every time you're about to route based on a prior comment, label, or tool return:

1. Ask: "Does this refer to current state?"
2. If the cost of re-reading is low, re-read.
3. If the comment conflicts with your own live observation, trust your observation.
4. Write a correction comment when you catch a stale claim — future agents (and future you) need the trail.

## Why This Is a Separate Principle From Laziness

Laziness would say "do minimum work." Verify-by-reading says "do the cheap verification that catches silent drift." These sound similar but have different failure modes:

- Laziness fails by skipping verification when it would have caught something.
- Verify-by-reading fails only when the verification itself is wrong.

The practical difference: laziness argues "this has been right before, so don't bother checking." Verify-by-reading argues "this was right **when it was written**, and what I care about is **now**, so check." The check is the discipline.

## Not Universally Applicable

There are costs to over-checking. Running `cargo metadata` before every tool call is absurd. The principle applies when:

1. The source of claim is **prior-in-time** (an old comment, a cached page, a tool return).
2. The cost of re-reading is **low** (one gh call, one git command, one file read).
3. The cost of being wrong is **non-trivial** (routing to the wrong agent, merging a bad PR, trusting a stale receipt).

When all three conditions apply, re-read. When any fails, don't bother.

## Related Patterns

This is a specific case of a more general principle in verification: **the validity interval of any check is bounded.** A receipt from 2h ago is not a receipt about now. A label from last PR is not a label about this PR. A test that passed on master yesterday does not say the test passes on master today.

The perl-lsp harness encodes this via receipt freshness (`MAX_AGE_SECONDS=3600` in the task-completed hook) and via receipt-to-SHA binding (the label-receipt system). Both are mechanical enforcements of the verify-by-reading principle — "this label is bound to SHA X; if current HEAD is Y, the label doesn't apply."

## Related

- [VERDICT_OVERRIDE_PATTERNS.md](../contributing/VERDICT_OVERRIDE_PATTERNS.md) — Pattern 2 is the operator playbook version of this article
- [WHEN_RECEIPTS_LIE.md](WHEN_RECEIPTS_LIE.md) — prior session's enumeration of the same principle
- [DOWNSTREAM_CATCHES_UPSTREAM.md](DOWNSTREAM_CATCHES_UPSTREAM.md) — related: downstream layers catch upstream comments that looked authoritative
- Session retrospective: [../forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md)
