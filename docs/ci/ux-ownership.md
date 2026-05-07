# UX Regression Lane Ownership

`perl-lsp` has two UX regression surfaces today:

| Workflow | Job | Trigger | Paths-filtered? |
|---|---|---|---|
| `ci.yml` | `ux-tests` | `pull_request`, `push`, `merge_group` | no (runs on all PRs) |
| `ux-regression-gate.yml` | `ux-regression-gate` | `pull_request` | yes (LSP / DAP / extension / `features.toml`) |

PR 03's inventory flagged this as a duplicate-intent pair. PR 15 is the **decision PR**:
which workflow owns merge-blocking UX, and what does the other one do.

> Companion: [inventory.md](inventory.md), [policy-ledgers.md](policy-ledgers.md).

---

## Decision (Option A)

**`ci.yml::ux-tests` owns merge-blocking UX. `ux-regression-gate.yml` becomes a
path-gated *secondary* surface.**

| Workflow | After PR 15 |
|---|---|
| `ci.yml::ux-tests` | Required, runs on every PR, blocks merge. (No change today.) |
| `ux-regression-gate.yml` | Path-filtered secondary. Still blocks PRs that touch UX-relevant paths, but it duplicates `ci.yml::ux-tests`'s coverage and is a candidate for retirement after PR 17 actuals show overlap. |

### Why Option A

`ci.yml` is already the merge-gate aggregator surface (`merge-gate` job aggregates UX,
memory, Windows, etc.). Splitting UX into a separate workflow would fragment branch
protection. Keeping UX in `ci.yml` is the lower-risk choice.

`ux-regression-gate.yml` predates this rollout and has its own structure (path-filtered,
own concurrency). Removing it is a **reversible** PR 17 follow-up that needs:

1. PR 08 actuals confirming `ci.yml::ux-tests` and `ux-regression-gate.yml` produce
   redundant signal on UX-touching PRs.
2. A migration window where the gate's evidence (receipts, JUnit) is still emitted by
   `ci.yml::ux-tests`.
3. Branch-protection update to drop the standalone workflow's required status (if any).

This PR does **not** delete `ux-regression-gate.yml`. It just records the ownership
decision and updates the policy ledgers to flag the duplicate.

---

## Whitelist update

`policy/ci-lane-whitelist.toml`'s `ux_tests` lane already records:

```toml
duplicate_of = ["workflow:ux-regression-gate"]
default_pr_exception = "ci-exception-ux-regression-default-pr"
```

This file adds the *direction* of the duplication: `ci.yml::ux-tests` is the canonical
owner; `ux-regression-gate.yml` is the duplicate. The exception entry's expiry
(2026-08-07) is the deadline by which PR 17's actuals must produce a retire-or-keep
decision.

---

## What this PR does not do

- Does **not** delete `ux-regression-gate.yml`.
- Does **not** change branch protection.
- Does **not** modify `ci.yml::ux-tests`.
- Does **not** change the workflow's path filter, `if:`, or trigger.

Pure governance: a written decision so PR 17 has a starting point.

---

## Action items for PR 17

After PR 08 actuals run for ≥ 2 weeks:

1. Diff `ci.yml::ux-tests` vs `ux-regression-gate.yml` evidence files on the same SHA
   to confirm they catch the same regressions.
2. If yes: retire `ux-regression-gate.yml` and remove the exception entry.
3. If no: identify what `ux-regression-gate.yml` catches that `ci.yml::ux-tests` does
   not, and either fold that into `ci.yml` or update this doc to record the divergence.
