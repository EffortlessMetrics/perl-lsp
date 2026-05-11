# CI Contributor UX Burndown

> **Substrate (already built)**: CI receipts contract (`docs/ci/receipt-contract.md`), lanes + LEM policy (`docs/ci/agent-leases.md`, `docs/ci/agent-receipts-and-freshness.md`), xtask substrate for CI orchestration, and the CI lane history aggregator (#8510, merged) which writes `ci-lane-history.json` on a schedule.
> **Connector gap**: contributors cannot see, on a PR, what CI actually ran, why specific lanes were chosen, or whether their local `just pr-fast` matches CI's decision. The connector is (a) a sticky PR comment that summarizes per-push lane decisions and outcomes, and (b) a local `ci-doctor` command that prints the same diagnostic CI emits — so contributors can resolve parity issues before pushing.
> **0.14.0 upside**: every PR becomes self-explaining ("here's what ran, here's why, here's what failed, here's the receipt"). Pre-push parity becomes a one-command check instead of guess-and-rerun. CI feedback loops shorten from minutes to seconds for the common "did I trigger the right lanes?" question.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1 — PR sticky summary (dry-run) | #4825 | yes (`builder-ready`) | — | `cargo xtask ci pr-summary --base origin/master --dry-run` |
| 2 — `ci-doctor` v1 | #4826 | yes (`builder-ready`) | — | `cargo xtask ci doctor` |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated.
- [ ] Claim boundary recorded.

## Claim boundary

**This rail proves**: contributors see clear, structured CI scope on every PR push, and can diagnose local/CI parity gaps with one command before pushing. The sticky summary distills receipts into human-readable lane outcomes; `ci-doctor` mirrors CI's decision logic locally.

**This rail does NOT prove**: that the underlying CI lane policy is *correct* (lane-selection rules, LEM thresholds, receipt schemas are governed by separate doctrine), nor that receipts are *complete* (gaps in receipt emission are owned by the receipt contract rail). UX cannot fix policy bugs; it only surfaces them faster.

## Receipts

```bash
# Phase 1 receipt: sticky-summary dry-run prints what would be posted.
cargo xtask ci pr-summary --base origin/master --dry-run

# Phase 2 receipt: ci-doctor prints the same diagnostic CI would.
cargo xtask ci doctor

# Per-phase issue status.
gh issue view 4825
gh issue view 4826
```

## Related

- Umbrella issue: #8630 (`rail: CI contributor UX (#4825 + #4826)`).
- Architecture / spec docs: `docs/ci/receipt-contract.md`, `docs/ci/agent-leases.md`, `docs/ci/agent-receipts-and-freshness.md`, the audit-agent spec comments on #4825 and #4826 (added by audit agent `a48016d8b1a6031cb`).
- Status doc: `docs/project/status/index.md`.
- Adjacent rails: `docs/development/FILE_POLICY_RAIL.md` (advisory mode failures should show up in the sticky summary), `docs/ci/codecov-rollout.md` (coverage status + sticky summary share the PR comment surface — sticky owns the "what ran" claim, Codecov stays informational), `docs/development/PERL_ORACLE_RAIL.md` (perl-oracle timeouts surface via CI doctor).

## Do not combine

- Do not combine with: Rust 1.95 lint cleanup, Codecov rollout, Perl-oracle work, file-policy promotion, dependency bumps.
- Do not bundle Phase 1 dry-run with sticky-comment posting in the same PR — dry-run lands first so reviewers can see the output without it appearing on every PR yet.
- Do not let `ci-doctor` evolve into a lane runner; it diagnoses, it does not execute CI lanes.
- Do not weaken the receipt contract to fit what the sticky summary needs — the summary reads receipts, never the inverse.

## Lane assignment

**Builder (sonnet)** owns both phases. Each is small (one xtask command + integration with existing receipt infrastructure), but the sticky-summary surface touches PR comment posting and rate-limiting; sonnet's care matters for "don't spam every PR push" edge cases.

Coordinate with audit agent `a48016d8b1a6031cb` — it is currently adding `builder-ready` labels and spec comments to #4825 and #4826. Use whatever spec it lands as the canonical builder input.
