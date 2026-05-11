# Freshness / Issue-Spec Discipline Burndown

> **Substrate (already built — as docs-only spec PRs)**: PR #8556 documents the `freshness-check` surfaces; PR #8557 codifies "issue body = current truth, comments = research log"; PR #8558 specifies the Perl subprocess ambient-input contracts; PR #8553 records the prefix-vs-exact fixture rule and `.`-wildcard inc-root semantics.
> **Connector gap**: the actual `cargo xtask freshness-check` implementation plus the Claude pre-tool stale-read hook that delegates to it. The spec docs declare the contract; the connector is the running tool that enforces it.
> **0.14.0 upside**: silent stale-checkout failure mode goes away. Agents (and humans) get a hard stop before they make forward claims about code state from a checkout that is N commits behind master, eliminating an entire class of "scope rewrite from re-reading master" rework.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1a. Spec — freshness-check surfaces | [#8556](https://github.com/EffortlessMetrics/perl-lsp/pull/8556) | docs-only spec | #8556 | spec land |
| 1b. Spec — issue body = current truth | [#8557](https://github.com/EffortlessMetrics/perl-lsp/pull/8557) | docs-only spec | #8557 | spec land |
| 1c. Spec — Perl subprocess ambient-input contracts | [#8558](https://github.com/EffortlessMetrics/perl-lsp/pull/8558) | docs-only spec | #8558 | spec land |
| 1d. Spec — prefix-vs-exact fixture rule | [#8553](https://github.com/EffortlessMetrics/perl-lsp/pull/8553) | docs-only spec | #8553 | spec land |
| 2. Implementation — `cargo xtask freshness-check` + Claude hook | [#8619](https://github.com/EffortlessMetrics/perl-lsp/issues/8619) | not yet | _pending_ | `cargo xtask freshness-check --base origin/master` |

> **Path note**: the per-tool spec at `docs/devex/freshness-check.md` is filed via PR #8556. This rollout doc lives in `docs/development/` to colocate with the other rail rollouts. Do not move the per-tool spec.

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated (`docs/project/status/ci_hardening.md` regenerated post-merge).
- [ ] Claim boundary recorded.

## Claim boundary

This rail proves that **both surfaces — repo-native `cargo xtask freshness-check` and the Claude pre-tool stale-read hook — exist, run, and refuse to proceed when the working tree is behind `origin/master` past a configured threshold**.

This rail does **NOT** prove:

- That every external agent (codex, factory-droid, aider, dependabot) integrates the hook. The xtask is callable by any of them, but adoption is a separate per-agent concern.
- That the staleness threshold is correctly tuned. Tuning is an operational follow-up, not a closeout gate.
- That a clean freshness-check guarantees correctness of downstream claims. It only guarantees the checkout is fresh; semantic claims about that fresh checkout are still the agent's responsibility.

## Receipts

```bash
# Phase 2 closeout
cargo xtask freshness-check --base origin/master
```

Exit status zero means: the working tree is at or ahead of `origin/master` within the configured threshold. Non-zero means: stop, refresh, retry. The Claude hook delegates to this exact invocation, so one passing receipt covers both surfaces.

## Related

- Umbrella issue: [#8546 — tooling: stale-checkout warning](https://github.com/EffortlessMetrics/perl-lsp/issues/8546) (amended 2026-05-11 to two surfaces)
- Tracker for this rollout doc: #8632
- Spec PRs: [#8556](https://github.com/EffortlessMetrics/perl-lsp/pull/8556), [#8557](https://github.com/EffortlessMetrics/perl-lsp/pull/8557), [#8558](https://github.com/EffortlessMetrics/perl-lsp/pull/8558), [#8553](https://github.com/EffortlessMetrics/perl-lsp/pull/8553)
- Implementation issue: [#8619 — tooling(devex): implement cargo xtask freshness-check (#8546)](https://github.com/EffortlessMetrics/perl-lsp/issues/8619)
- Architecture / spec docs: `docs/devex/freshness-check.md` (per-tool spec); `xtask/src/bin/` (where the xtask will live)
- Status doc: [docs/project/status/ci_hardening.md](../project/status/ci_hardening.md)
- Adjacent rails:
  - All other rails depend on freshness for correct issue-spec discipline; this rail is foundational, not parallel

## Do not combine

Do **not** roll this rail's PRs into:

- Other `cargo xtask` work (semantic-scorecard, semantic-shadow-compare, etc.). Each xtask deserves its own focused PR.
- The control-plane lock or worktree-manager work. Freshness is a read-time gate; those are write-time concerns.
- Issue-body-truth policy changes that are not direct dependencies of #8557. The "issue body = current truth" spec lands once; further refinements are their own PRs.

## Lane assignment

**Builder (sonnet)** — phase 2 implementation contract in #8619. The four phase-1 spec PRs (#8556, #8557, #8558, #8553) are docs-only and land on their own normal review cadence; this rail does not gate on their content beyond requiring them merged before #8619 starts.
