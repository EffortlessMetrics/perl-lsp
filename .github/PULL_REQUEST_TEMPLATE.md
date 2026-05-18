<!--
PR title convention: end with a real issue ref, e.g.
  fix(crate): description (#NNNN)

Replace NNNN with the tracking issue number this PR addresses.
The validate-title CI check enforces this format — placeholder refs
like (#0000) or (#9999) will fail CI.
-->

## Summary
<!-- What changed and why. Link the issue: Fixes #NNN -->

## Source-of-truth links
<!-- Use n/a when this PR is outside the source-of-truth stack. -->
Proposal:
Spec:
ADR:
Plan item:
Active goal:

## Scope
- [ ] Proposal / why
- [ ] Spec / behavior contract
- [ ] ADR / durable decision
- [ ] Plan / sequencing
- [ ] Active goal / current execution state
- [ ] Runtime / implementation
- [ ] Policy ledger
- [ ] Support-tier update
- [ ] Generated status / receipt

## Non-goals
<!-- What this PR explicitly does not do. -->

## Proof
<!-- Commands run, results, unavailable proof, and substitute evidence. -->

## Claim boundary
<!-- What may be claimed after this PR? What may not be claimed yet? -->

## Rollback
<!-- How to revert safely. -->

## Changes
<!-- List changed files and what each change does -->

## Test
<!-- What test was added? Does it fail before the fix and pass after? -->

## Verification
- [ ] `cargo xtask fmt` — clean
- [ ] I used a narrow orthogonal pass first (freshness check, truth-check, or targeted repro) before the broader gate.
- [ ] `cargo clippy -p <crate> --tests` — clean
- [ ] `cargo test -p <crate>` — pass
- [ ] This PR introduces UX-visible changes. I have verified that error messages are actionable and the UX test harness still passes.

## Retained State
<!-- Complete this when the PR adds or changes a long-lived map, cache, queue, background task, session holder, or subprocess lifecycle. Otherwise write "N/A". -->
- [ ] Owner, key type, bound, and cleanup event are documented.
- [ ] Key normalization is handled.
- [ ] Close-only behavior is distinct from delete/folder-removal behavior.
- [ ] Delayed background work cannot repopulate stale state.
- [ ] A regression test, receipt, snapshot counter, or debug counter covers the state.

## What I considered but didn't do
<!-- Alternative approaches, related issues found, scope decisions -->

## What's next
<!-- Follow-up work, edge cases to address, related issues to file -->

## CI cost / verification note
<!-- See docs/ci/cost-and-verification-policy.md and docs/ci/lem-budgeting.md. -->
- [ ] I used the cheapest relevant proof first.
- [ ] I did not request broad CI unless this PR's risk surface needs it.
- [ ] Any high-cost CI label (`full-ci`, `ci-budget-ack`, `ci-budget-override`) is explained in the PR body.
- [ ] New CI work states the failure mode it catches and its estimated LEM.

## Agent
<!-- If created by swarm: agent type, issue number, model tier -->
