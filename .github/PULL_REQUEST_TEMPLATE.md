<!--
PR title convention: end with a real issue ref, e.g.
  fix(crate): description (#NNNN)

Replace NNNN with the tracking issue number this PR addresses.
The validate-title CI check enforces this format — placeholder refs
like (#0000) or (#9999) will fail CI.
-->

## Objective
<!-- One sentence: what this PR is trying to prove or change. Link the issue: Fixes #NNN -->

## Summary
<!-- What changed and why. Keep this scoped to one concern. -->

## Lane
<!-- Pick one. See docs/swarm/review-rules.md. -->
- [ ] trust
- [ ] substrate
- [ ] reliability

## Claim Boundary
<!-- What changes, and what does this PR explicitly not claim? -->

## Non-goals
<!-- What this PR intentionally does not do. Name adjacent lanes or future work that remain out of scope. -->

## Promotion Discipline
<!-- Required for trust-lane PRs. Write N/A for substrate or reliability PRs. -->
- Surface:
- Fact class:
- Promotion rule:
- Fallback rule:
- Blocker rule:
- Receipt:

## Behavior
- [ ] no behavior change
- [ ] preview only
- [ ] scoped pilot
- [ ] live behavior change

## Risk Surfaces
- [ ] edit-producing
- [ ] provider behavior
- [ ] subprocess
- [ ] path/module resolution
- [ ] public API
- [ ] parser/lexer core

## Changes
<!-- List changed files and what each change does -->

## Test
<!-- What test was added? Does it fail before the fix and pass after? -->

## Verification
- [ ] Lane and risk surface are declared above.
- [ ] Trust-lane PRs name promotion, fallback, blocker, and receipt boundaries.
- [ ] `cargo xtask fmt` — clean
- [ ] I used a narrow orthogonal pass first (freshness check, truth-check, or targeted repro) before the broader gate.
- [ ] `cargo clippy -p <crate> --tests` — clean
- [ ] `cargo test -p <crate>` — pass
- [ ] This PR introduces UX-visible changes. I have verified that error messages are actionable and the UX test harness still passes.

## Local Proof Commands
<!-- List exact commands run locally, with pass/fail/not-run status. -->
- `<command>` — pass/fail/not run

## Quality Gates
<!-- Paste or summarize target/receipts/quality/quality-gate.md when this PR touches proof-gated code, receipts, coverage policy, RIPR policy, CI, or test evidence. Write N/A only when no proof-gated surface changed. -->
- new RIPR gaps:
- total RIPR+ gaps:
- patch coverage:
- project coverage:
- receipt freshness:
- exception status:
- local verify command:
- receipt command:

## RIPR / Coverage Effect
<!-- State the RIPR and coverage effect of this PR. Example: no new severe RIPR gaps; patch coverage unchanged; project coverage +0.1pp. -->

## Cleanup Performed
<!-- State scratch files, target outputs, receipts, temp files, branches, or worktrees cleaned after validation. -->

## Retained State
<!-- Complete this when the PR adds or changes a long-lived map, cache, queue, background task, session holder, or subprocess lifecycle. Otherwise write "N/A". -->
- [ ] Owner, key type, bound, and cleanup event are documented.
- [ ] Key normalization is handled.
- [ ] Close-only behavior is distinct from delete/folder-removal behavior.
- [ ] Delayed background work cannot repopulate stale state.
- [ ] A regression test, receipt, snapshot counter, or debug counter covers the state.

## What I considered but didn't do
<!-- Alternative approaches, related issues found, scope decisions -->

## Remaining Work
<!-- Follow-up work, edge cases to address, related issues to file -->

## CI cost / verification note
<!-- See docs/ci/cost-and-verification-policy.md and docs/ci/lem-budgeting.md. -->
- [ ] I used the cheapest relevant proof first.
- [ ] I did not request broad CI unless this PR's risk surface needs it.
- [ ] Any high-cost CI label (`full-ci`, `ci-budget-ack`, `ci-budget-override`) is explained in the PR body.
- [ ] New CI work states the failure mode it catches and its estimated LEM.

## Agent
<!-- If created by swarm: agent type, issue number, model tier -->
