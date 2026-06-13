---
tags: [multi-agent, dap, deep-review, fix-forward, merge-velocity]
repos: [perl-lsp-swarm]
related: ["#1240", "#1363", "#1364", "#898"]
portable: true
article_asset: true
search_terms: [handle_pause, send_interrupt_signal, session_present, signal_sent, fix-forward, merged-before-review, deep-review-latency, 3-green, has_session]
---

# PR merged on 3-green before in-flight deep-review completed; fix landed as fix-forward

**Date**: 2026-06
**Hazard class**: other (process / review-gate timing)
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)

## What happened

PR #1240 (fix(dap): protocol-safe errors for execution control without session, implementing
issue #898) merged on a 3-green CI result while a deep-review was still in flight. The
deep-review had already identified a correctness bug in `handle_pause`: the function used
`send_interrupt_signal(pid)`'s return value as its "is a session present?" sentinel. Signal
delivery can return `false` even when a session IS active (zombie process, Windows
`GenerateConsoleCtrlEvent` failure, Unix `kill(SIGINT)` on an already-exited PID). The
result: a live session with a signal delivery failure would emit "no Perl debug session is
active" — factually wrong. The fix commit `6ca0a76c` was stranded on the review branch
`fix/898-no-session-honest-errors` and never landed with #1240. Issue #1363 tracked it and
PR #1364 fixed it forward with the stranded commit as its starting point.

## Why

In a multi-thread high-velocity repo, ops agents merge in batches of 3 based on label state
(`merge-ready`, `ci-green`, `diff-audited`). A deep-review that starts after those labels
are set cannot stop a merge that ops has already queued — the review may complete after the
merge commit is pushed to main. The three required checks (`Perl LSP Rust Small Result`,
`ripr+ New Gap Gate`, `Codecov / Patch 95`) are CI signals, not review completion signals.
Ops acts on CI state; deep-review label acts only as a gate when set BEFORE the merge batch
starts.

## Fix

PR #1364 was filed as a clean fix-forward: `execution.rs` separated `session_present` (is a
session lock occupied? — gates the guidance message) from `signal_sent` (did
`send_interrupt_signal` succeed? — drives success vs "Failed to pause debugger").
Two integration tests in `crates/perl-dap/tests/pause_signal_delivery_tests.rs` proved the
new contract: no-session → guidance message; session-present + signal-fail → "Failed to
pause debugger."

## Spec impact

Adds a recovery pattern to the pipeline doctrine: when a deep-review fix is stranded on a
branch after its target PR merges, file a fix-forward issue immediately (don't defer), name
it as a follow-up to the original issue (e.g., "#898 follow-up"), and reference the stranded
commit in the PR body. This makes the fix traceable and ensures it is not lost between
sessions. The orphaned commit pattern (a fix commit stranded on a review branch) is distinct
from a normal revert and requires a dedicated forward PR.

## Portable lesson

In a multi-thread high-velocity repo, do not assume your deep-review gates the merge. Design
the pipeline for catch-and-fix-forward: when a deep-review finds a bug after a PR has
already merged, the correct response is a prompt fix-forward PR (not a revert, not a stale
branch). Consider landing review-identified fixes on the PR branch before the PR is labeled
`merge-ready` to close the window.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Process / review-gate timing (not a code hazard class)
- **Generalization**: Multi-thread merge velocity means reviews do not always gate merges; the pipeline must support catch-and-fix-forward as a first-class recovery path.

## Related PRs

- [#1240](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1240) — merged PR with stranded deep-review fix
- [#1363](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1363) — issue: tracking the stranded handle_pause fix
- [#1364](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1364) — fix-forward: separate session_present from signal_sent
- [#898](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/898) — original issue that #1240 closed
