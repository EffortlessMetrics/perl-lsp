# PLSP-SPEC-0013: Agent build storage and gates

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: [PLSP-ADR-0001](../adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Linked plan: [0.14.0 Readiness Queue](../releases/0.14.0-readiness.md)
Status impact: local proof commands, gate receipts, storage hygiene, CI
hardening status, PR disposition comments

## Current implementation status

This spec is accepted as the agent proof and storage-hygiene contract. The
current repo already routes agent Cargo proof through `./scripts/cargo-safe`,
uses `./scripts/storage-doctor` as the storage receipt, and records gate timeout
classification as control-plane evidence rather than product behavior proof.

This spec governs how agents report local proof. It does not replace
trust-lane CI routing, provider receipts, parser status, release gates, or
support-tier evidence.

## Contract

Agent proof must use bounded build storage by default. The normal route for
Cargo-backed checks is:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe <cargo subcommand>
```

The `just agent-*` recipes are the preferred higher-level entry points when the
agent needs their composed behavior. They must continue routing heavy Cargo
work through `./scripts/cargo-safe`.

Raw `cargo`, raw `just`, or direct recipe commands are diagnostic tools, not the
default proof path for agent maintenance. They are allowed when an operator is
isolating a gate failure, reproducing an underlying command, or verifying a
non-Cargo recipe that cannot yet run through `cargo-safe`. Any such use must be
reported in the PR or disposition summary when it affects proof.

Storage hygiene is part of the proof contract. Agents must run:

```bash
./scripts/storage-doctor
```

after substantial local builds and after any diagnostic command that may create
repo-local build output. If a repo-local `target/` appears, the PR or closeout
summary must state whether it was removed or intentionally retained. Small
`target/receipts` output may exist while a receipt-producing gate is running,
but durable evidence should be copied into the owning docs/status location or
the local `target/` should be removed before handoff.

## Gate Timeout Classification

If a composed gate such as `just agent-pr-fast` or
`cargo xtask gates --tier pr-fast --receipt` fails because a gate wrapper times
out, the failure must be classified before it is attributed to product code.

Use this rule:

```text
If the wrapped gate times out but the underlying command passes directly on
current master, classify the result as a control-plane timeout, not a
product-code failure.
```

The operator must capture:

- gate name
- wrapper timeout and budget
- receipt path and log path when present
- direct reproduction command
- direct reproduction result and duration
- whether a small control-plane PR is needed before the merge train continues

Timeout-budget fixes must change the source of truth that owns the gate budget,
such as `.ci/gate-policy.yaml`, and should include a policy or profile test
when one exists. The budget should reflect observed local or hosted runtime
plus headroom, not a stale target duration.

## Environment Handling

`cargo-safe` may serialize heavy Cargo work through a build lock and may route
compiler cache state through `sccache`. If `sccache` or a stale local server
holds the `cargo-safe` lock, stopping the local `sccache` server is an
environment repair, not a repo change.

Increasing `CARGO_LOCK_WAIT` is also an environment repair when the command is
otherwise valid and the delay is caused by a local lock holder. Do not classify
that condition as a PR failure unless the command still fails after the build
environment is repaired.

On Windows-hosted worktrees, if direct `just` invocation cannot locate its shell
or otherwise fails before running the recipe body, rerun the same recipe through
the working shell route and report the distinction. A shell-launch failure is
not a product-code failure.

## Acceptance

A PR, merge summary, or queue-disposition comment satisfies this spec when it
includes the relevant items below:

```text
proof route:
raw command used:
storage-doctor:
repo-local target:
receipt/log paths:
timeout classification:
environment repair:
follow-up:
```

Use `none` only when the field is truly not applicable. If the proof route was
the default `cargo-safe` route and no repo-local build output was created, the
summary can say so tersely.

Control-plane PRs that change gate budgets must also state:

- which gate budget changed
- what observed runtime justified the new budget
- whether the underlying command passed directly
- which policy or profile check protects the budget from regressing

## Valid PR Shapes

Valid PRs under this spec include:

- docs-only PRs that encode this contract
- gate-policy PRs that adjust timeout or budget values after direct
  reproduction
- policy-test PRs that assert minimum timeout or budget floors
- `cargo-safe`, `storage-doctor`, or `xtask gates` fixes that keep build output
  bounded and receipts visible
- PR-summary or disposition-template changes that expose storage and timeout
  classifications

## Invalid PR Shapes

Invalid PRs include:

- treating a wrapper timeout as a product-code failure without direct
  reproduction
- masking product-code failures as timeouts after the underlying command fails
  directly
- running raw workspace Cargo as routine proof without storage follow-up
- leaving large repo-local build output without reporting it
- deleting or cleaning build artifacts destructively outside the intended
  workspace path
- changing gate budgets without preserving a source-of-truth policy value
- using this spec to skip branch-protection checks

## Proof Commands

Docs-only PRs for this spec must run:

```bash
git diff --check
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask ci-hygiene check-doc-paths docs/specs
./scripts/storage-doctor
```

If the PR touches docs outside `docs/specs`, run the narrowest available
additional docs checker for that surface. If only the repo-wide docs check
exists and it fails on pre-existing unrelated docs debt, report that failure
and the scoped check that passed.

Gate-policy or budget PRs must also run:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask gate-policy check
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask fmt --check
```

When a PR is fixing a timeout discovered in `pr-fast`, it should run the fixed
gate if feasible:

```bash
just agent-pr-fast
```

If the direct shell route for `just` is not usable on the host, use the working
shell route and report that distinction in the PR body.

## Non-goals

- Do not replace [PLSP-SPEC-0011](PLSP-SPEC-0011-trust-lane-ci-routing.md)
  trust-lane routing.
- Do not define release readiness, publish approval, or tag execution.
- Do not claim product behavior correctness from a passing control-plane gate.
- Do not require raw `cargo` or raw `just` to be forbidden in all diagnostic
  situations.
- Do not require large workspace cleanup outside the assigned repo or worktree.
- Do not hand-edit generated status.

## Claim Boundaries

A passing `cargo-safe` command proves only the command and touched surface named
in the PR summary. It does not prove release readiness, broad workspace health,
parser bucket movement, provider cutover, or support-tier promotion.

A fixed gate timeout proves the control plane can run the gate within the new
budget. It does not prove the underlying product behavior beyond the gate's own
scope.

Storage hygiene proves repo-local build output is bounded or cleaned at the
time checked. It does not prove global disk health or future agent sessions.
