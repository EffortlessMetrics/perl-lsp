# ripr — Static Oracle-Gap Detection

> **Context**: This document is part of perl-lsp's [Industrialized AI](why-industrialized.md) CI architecture. The choices here are responses to operating at 1000+ PRs/day, not premature optimization.

> **Doctrine**: `ripr` is static mutation-exposure analysis. It catches the
> same class of findings mutation testing catches — weak test/oracle
> exposure — but earlier and cheaper because it is static and PR-time.
> Mutation testing remains the slower runtime backstop for findings that
> static analysis cannot predict. `ripr` shifts mutation signal left.

`ripr` adds **mutation-testing-lite oracle-gap detection at static-analysis prices**. It
sits between coverage and runtime mutation testing on the verification ladder: more
oracle-aware than coverage, far cheaper than mutation testing.

> Companion: [verification-ladder.md](verification-ladder.md),
> [labels.md](labels.md), [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml).
> Workflow: [`.github/workflows/ripr.yml`](../../.github/workflows/ripr.yml).

---

## Current routing posture

During the proof-lane rollout, the `ripr` workflow now blocks PRs that introduce
new diff-scoped RIPR gaps or fail to produce current RIPR receipts. Repo-wide
RIPR+ total zero remains a burn-down target until the final enforcement slice.
The workflow still runs for every PR when it is ready for review so docs-only,
policy-only, workflow-only, and code PRs all carry current proof receipts.

## What ripr does

For each changed Rust function, ripr asks the mutation-testing-shaped question
**statically**: is the changed behavior exposed to a meaningful test discriminator?

It does **not**:

- Run mutants.
- Emit `killed` / `survived` counts.
- Replace mutation testing.

When reporting `ripr` findings, use ripr's own classifications:

| Classification | Meaning |
|---|---|
| `exposed` | reachable + nearby discriminating test |
| `weakly_exposed` | reachable, weakly-discriminating test only |
| `reachable_unrevealed` | reachable, no discriminating test found |
| `no_static_path` | analysis could not find a reachable path |
| `infection_unknown` | could not classify infection |
| `propagation_unknown` | could not classify propagation |
| `static_unknown` | analysis bottomed out |

Do **not** translate these into `killed` / `survived`. They mean something different.

---

## When it runs

- Every PR targeting `master` or `main`.
- No path filter is applied: docs-only, policy-only, workflow-only, and code
  PRs all run the RIPR proof workflow so every merge has current proof receipts.
- Draft PRs are skipped while draft, then the explicit `ready_for_review`
  trigger runs the workflow before the PR can merge.
- Manual via `workflow_dispatch`.
- Labels can still route deeper or more expensive evidence lanes elsewhere, but
  this proof workflow is no longer label-gated.

---

## Behavior

- Produces diff-scoped PR evidence under `target/ripr/pr/`.
- Produces the repo-wide RIPR+ baseline receipt at
  `target/receipts/quality/ripr-plus.json`.
  The repo-wide receipt applies `policy/ripr-suppressions.toml` path
  suppressions before computing the unresolved total so non-production retained
  surfaces such as `archive/**` do not count against the final zero target.
- Produces review guidance under `target/ripr/review/`.
- Runs `cargo xtask quality-gate --mode enforce-new-ripr`, which blocks new
  severe RIPR gaps and stale or missing repo-wide, diff-scoped, or
  review-guidance receipts.
- Applies the documented suppression policy to diff-scoped PR evidence as well
  as repo-wide RIPR+ receipts. Suppressed paths remain visible in receipts, but
  they do not count as new blocking gaps.
- In CI, review guidance has an explicit timeout. If guidance cannot produce an
  actionable receipt for a new gap, the quality gate reports the missing repair
  packet instead of hiding the failure.
- Emits non-blocking warning annotations from `comments[]` only.
- Produces mutation-routing evidence under
  `target/xtask/impacted-evidence/`.
- Uploads the `ripr-pr-evidence` artifact with required-artifact semantics.
- Appends `target/ripr/pr/summary.md` and
  `target/receipts/quality/quality-gate-ripr.md` to the GitHub step summary.

---

## Suppressions

Suppressions live in [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml).
Each suppression requires:

- `id` — stable identifier
- `kind` — e.g. `generated_or_non_production_surface`
- `paths` and/or `classification` — what to suppress
- `owner` — accountable person/team
- `reason` — why this is suppressed
- `created`, `review_after`, `expires` — dates

The suppression file is read by `ripr.toml`'s `[suppressions] path` setting.
The `cargo xtask ripr-pr` and `cargo xtask ripr-plus` wrappers also apply the
same path suppressions when they compute diff-scoped and repo-wide receipts.
Suppressed files are reported separately from active unresolved gaps.

Current suppressed non-production proof surfaces include:

- archived source under `archive/**`;
- generated status docs under `docs/project/status/**`;
- executable editor UX receipt tests under
  `crates/perl-lsp-ux-tests/tests/**`.

---

## Promotion path

| PR | What happens |
|---:|---|
| 1 | Unfiltered ready-for-review workflow routing plus current RIPR receipts; no CI enforcement. |
| 8 | Blocking new-gap gate for diff-scoped RIPR PR evidence and receipt freshness. |
| Later | Total RIPR+ unresolved count reaches zero, then full `quality-gate --mode enforce` becomes blocking. |

This slice blocks new RIPR gaps and stale or missing RIPR proof receipts. It does
not require repo-wide RIPR+ total zero; that remains exception-backed until the
burn-down closes.

---

## Toolchain

`rust-toolchain.toml` pins `1.95.0`. The workflow installs `ripr` `0.5.0` as
the current advisory version for this lane.

---

## Running locally

```bash
rtk cargo install ripr --version 0.5.0 --locked
rtk ripr doctor
rtk cargo xtask ripr-pr --base origin/HEAD --head HEAD
rtk cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json
rtk cargo xtask ripr-review-comments --base origin/HEAD --head HEAD
rtk cargo xtask impacted-evidence
rtk cargo xtask ripr-pr-summary
rtk cargo xtask ripr-annotations
rtk cargo xtask quality-gate --mode enforce-new-ripr --ripr-receipt target/receipts/quality/ripr-plus.json --ripr-pr-receipt target/ripr/pr/repo-exposure.json --review-receipt target/ripr/review/comments.json --ripr-base origin/HEAD --ripr-head HEAD --receipt target/receipts/quality/quality-gate-ripr.json --summary target/receipts/quality/quality-gate-ripr.md
rtk cargo xtask ripr-pr --base origin/HEAD --head HEAD --check
rtk cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json --check
rtk cargo xtask ripr-review-comments --base origin/HEAD --head HEAD --check
rtk cargo xtask impacted-evidence --check
rtk cargo xtask ripr-pr-summary --check
rtk cargo xtask ripr-annotations --check
rtk cargo xtask quality-gate --mode enforce-new-ripr --ripr-receipt target/receipts/quality/ripr-plus.json --ripr-pr-receipt target/ripr/pr/repo-exposure.json --review-receipt target/ripr/review/comments.json --ripr-base origin/HEAD --ripr-head HEAD --receipt target/receipts/quality/quality-gate-ripr.json --summary target/receipts/quality/quality-gate-ripr.md --check
```
