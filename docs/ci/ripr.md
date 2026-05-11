# ripr — Static Oracle-Gap Detection

`ripr` adds **mutation-testing-lite oracle-gap detection at static-analysis prices**. It
sits between coverage and runtime mutation testing on the verification ladder: more
oracle-aware than coverage, far cheaper than mutation testing.

> Companion: [verification-ladder.md](verification-ladder.md),
> [labels.md](labels.md), [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml).
> Workflow: [`.github/workflows/ripr.yml`](../../.github/workflows/ripr.yml).

---

## Rust 1.95 rollout note

During the Rust 1.95 / 0.14.0 rollout, `ripr` remains advisory. The rollout map in [perl-lsp-rust-1.95-rollout.md](perl-lsp-rust-1.95-rollout.md) keeps normal PRs on ripr plus existing gates, reserves mutation testing for targeted/nightly/release lanes, and calls out routing follow-up work for docs-only and fixture-only changes.

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

- Every PR that touches `crates/**`, `xtask/**`, `Cargo.toml`, `Cargo.lock`, or `ripr.toml`.
- Skipped on docs-only PRs.
- Manual via `workflow_dispatch`.
- Forced via the `ripr` label (PR 07 wires this through PR Plan).

---

## Behavior

- `continue-on-error: true` — does **not** block merges.
- Uploads `target/ripr/ripr.json` and `target/ripr/ripr.sarif`.
- Posts a per-PR step summary via `scripts/ci/ripr_summary.py`.

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

---

## Promotion path

| PR | What happens |
|---:|---|
| 06 | Advisory only — this PR. |
| 18 | Narrow soft-gate for new high-confidence findings on production Rust diffs. Acknowledged via `ripr-waive` / `full-ci` / `ci-budget-ack` labels. |

The soft-gate at PR 18 only fires when:

- Classification is `reachable_unrevealed` or `weakly_exposed`.
- Production Rust changed and no nearby test changed.
- Finding is not in `policy/ripr-suppressions.toml`.
- Confidence is high.

ripr is **never** used as proof; it is used as a **prompt**.

---

## Toolchain

`rust-toolchain.toml` pins `1.93.1`. While the repository is on that toolchain,
the workflow installs `ripr` `0.4.0`, the latest compatible advisory version for
the Rust 1.93 line. The Rust 1.95 rollout can unpin or bump this after the repo
toolchain moves.

---

## Running locally

```bash
cargo install ripr --version 0.4.0 --locked
ripr doctor
ripr check --base origin/master
ripr check --base origin/master --json > target/ripr/ripr.json
python3 scripts/ci/ripr_summary.py \
    --report target/ripr/ripr.json \
    --summary /dev/stdout
```
