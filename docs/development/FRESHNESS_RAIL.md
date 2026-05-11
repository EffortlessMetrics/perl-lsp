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

## Stale binary resolution (test-harness)

Source-tree staleness is one failure class — but test harnesses can be stale in
a different way: the binary they invoke may be from a different SHA than the
current build.

### Symptom

A test passes or fails unrelated to the current code state because the harness
resolves a binary from an older build. The test is real; the binary is
historical.

### Canonical incident — #8624

`scenario_14_no_lib_cancellation` failed on every PR branch and on master.
Root cause was NOT a regression in `no lib` semantics. It was in the test
harness:

> `resolve_binary()` in `crates/perl-lsp-ux-tests/src/lib.rs` mishandled
> `CARGO_TARGET_DIR`, treating it as a workspace root and looking under
> `CARGO_TARGET_DIR/target/debug/perl-lsp`. That path never existed in agent
> worktrees, so resolution fell through to a stale v0.13.1 binary in the main
> checkout. The v0.13.1 binary predated the position-aware `no lib`
> cancellation fix (#8525), so it correctly-for-itself but incorrectly-for-the-test
> resolved `GoneModule`, fired PL700 instead of PL701, and failed the strict
> assertions.

Fix: **#8659** — `resolve_binary()` now looks in `CARGO_TARGET_DIR/debug/` directly.

### Detection

- Pre-test hook: assert `target/debug/perl-lsp` mtime is newer than the
  workspace SHA's commit time, OR build it explicitly before any harness run.
- `cargo xtask freshness-check --binaries` (future extension to the existing
  freshness-check command per **#8619**) — out of scope for this rail doc;
  tracked there.

### Mitigation

`resolve_binary()` in `crates/perl-lsp-ux-tests/src/lib.rs` must NEVER fall
through to a binary outside the current build's `target/` tree:

- Respect `CARGO_TARGET_DIR` if set: look in `$CARGO_TARGET_DIR/debug/perl-lsp`.
- If `CARGO_TARGET_DIR` is unset, use the workspace's `target/debug/` only
  when it was just built in this session.
- Refuse fallback to ancestor / parent workspace binaries — they are
  almost always stale.

### Related

- **#8624** — the regression issue.
- **#8659** — the fix PR.
- **#8546** — umbrella freshness tooling.
- **#8619** — `cargo xtask freshness-check` implementation (covers the
  `--binaries` extension as a follow-up).
- **#8485** — sibling stale-source-checkout incident.

---

## Do not combine

Do **not** roll this rail's PRs into:

- Other `cargo xtask` work (semantic-scorecard, semantic-shadow-compare, etc.). Each xtask deserves its own focused PR.
- The control-plane lock or worktree-manager work. Freshness is a read-time gate; those are write-time concerns.
- Issue-body-truth policy changes that are not direct dependencies of #8557. The "issue body = current truth" spec lands once; further refinements are their own PRs.

## Lane assignment

**Builder (sonnet)** — phase 2 implementation contract in #8619. The four phase-1 spec PRs (#8556, #8557, #8558, #8553) are docs-only and land on their own normal review cadence; this rail does not gate on their content beyond requiring them merged before #8619 starts.
