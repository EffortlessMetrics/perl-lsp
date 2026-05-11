# Rust 1.95 — Proactive Integrity Guards Rail

> **Companion to** [`RUST_1_95_ROLLOUT.md`](RUST_1_95_ROLLOUT.md) and
> [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md).
> Those docs track the **toolchain + workspace-allow burn-down** and the
> **strong-clippy-lints activation** rails. This doc tracks the
> **proactive CI integrity guards** rail — the checkers that turn
> the recurring "policy says X / workflow does Y / docs claim Z" drift
> class into a machine-checked invariant rather than a reactive review
> burden.
>
> Each row below is one PR, branch from clean `origin/master`, **one
> guard per PR**. A scout can file each row as a tracking issue; a
> builder can implement without re-discovering the failure mode.

## Why a separate rail

The toolchain rail (`RUST_1_95_ROLLOUT.md`) and the strong-clippy rail
(`STRONG_CLIPPY_LINTS_ROLLOUT.md`) both ratchet **what code we accept**.
This rail ratchets **what policy claims we accept**.

The class of bug each row catches:

```text
policy/ci-lanes.toml says lane X has workflow Y / job Z.
The workflow Y file was renamed (or job Z was renamed).
CI keeps running, no test fails, but every job in that workflow now
categorises as `lane.unknown` in the actuals lane.
The forecast/actual loop silently under-counts that lane until someone
opens an artifact and notices.
```

That class — "policy is internally inconsistent with the live YAML /
ledger / workflow surface" — is recurrent and reactive-only today. The
six rows below cover the documented failure modes.

These guards complement, not replace, the existing
`cargo xtask check-lint-policy` (which validates Clippy lint inheritance
and the active/planned ledger shape).

## Current vs target

| Layer | Current | Target | Status |
| --- | ---: | ---: | --- |
| Label declarations ↔ workflow `if:` consumption | unchecked | machine-checked via `cargo xtask check-label-enforcement` | todo (PG-1) |
| Risk-pack `selected_lanes` / `labels` referential integrity | unchecked | machine-checked via `cargo xtask check-risk-pack-integrity` | todo (PG-2) |
| Lane `workflow` / `workflow_name` / `job_name` → real YAML + jobs (with matrix expansion) | unchecked | machine-checked via `cargo xtask check-lane-mappings` | todo (PG-3) |
| Workflow allowlist ledger | absent (`policy/workflow-allowlist.toml` does not exist) | present + path / `external_actions` / `secrets_used` checks | todo (PG-4) |
| CI Actuals emitter + subscription coverage | absent (no `ci-actuals.yml`) | emitter exists + `cargo xtask check-actuals-coverage` keeps subscriptions in sync with lane declarations | todo (PG-5) |
| Broad-glob justification on `policy/ci-non-rust-allowlist.toml` | unchecked beyond field presence | non-empty/trimmed `broad_glob_reason` enforced | todo (PG-6) |

## Remaining PR ladder

Each row is one PR. Branch from clean `origin/master`. **Do not combine.**
For each row, the proven-pattern reference column points at the
sibling-repo PR that already shipped the same shape — useful for
estimating churn and verifying schema choices.

| # | Branch | Title | Scope summary | Proven-pattern reference |
| --- | --- | --- | --- | --- |
| PG-1 | `policy/check-label-enforcement` | `policy(ci): check declared routing labels against workflow usage` | New `cargo xtask check-label-enforcement` cross-references `policy/ci-budget.toml [labels]` against `.github/workflows/*.yml` job-level `if:` blocks (`contains(github.event.pull_request.labels.*.name, '<label>')`). Encodes the declared-but-not-enforced set explicitly. | shiplog#182 |
| PG-2 | `policy/check-risk-pack-integrity` | `policy(ci): check risk-pack referential integrity` | New `cargo xtask check-risk-pack-integrity` verifies every `[[risk_pack]].selected_lanes` value resolves to a `[lane.*]` table in `policy/ci-lanes.toml` and every `[[risk_pack]].labels` value resolves to a label in `policy/ci-budget.toml [labels]`. | shiplog#183 |
| PG-3 | `policy/check-lane-mappings` | `policy(ci): check CI lane workflow/job mappings` | New `cargo xtask check-lane-mappings` verifies every `[lane.*]`'s declared `workflow` path exists, `workflow_name` matches the YAML's top-level `name:`, and `job_name` resolves to a real job display name (with `${{ matrix.<var> }}` expansion supporting both simple-list and `matrix.include` forms). | shiplog#184 |
| PG-4 | `policy/workflow-allowlist-ledger` | `policy(files): add workflow-allowlist + check-workflows coverage` | Net-new ledger `policy/workflow-allowlist.toml` with one entry per `.github/workflows/*.yml` declaring `owner`, `reason`, `permissions`, `secrets_used`, and pinned `external_actions`. Add `cargo xtask check-workflows` that validates path-set coverage in both directions and `uses:` matches the declared `external_actions`. Cross-link from `docs/POLICY_ALLOWLISTS.md` and `docs/FILE_POLICY.md`. | shiplog#149 |
| PG-5 | `ci/add-ci-actuals-emitter` | `ci: add CI Actuals workflow + subscription coverage check` | New `.github/workflows/ci-actuals.yml` (workflow_run-driven) joins per-job timings to lanes, emits `target/ci/ci-actuals.json` against a v1 schema. New `cargo xtask check-actuals-coverage` verifies every lane's `workflow_name` is subscribed (or explicitly listed in a new `[actuals_exemptions].not_subscribed` section of `ci-lanes.toml`). | shiplog#148 + shiplog#185 |
| PG-6 | `policy/check-broad-glob-justifications` | `policy(files): tighten broad-glob justification check` | Tighten the existing non-rust-allowlist check to reject empty / whitespace-only `broad_glob_reason` (`""`, `"   "`, tab-only, etc.). Refactor the rule into a pure helper testable without a git-initialised workspace. | shiplog#187 |

## Acceptance gate (every PR)

Same as `RUST_1_95_ROLLOUT.md`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --lib --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --test-threads=1
cargo xtask check-lint-policy
git diff --check
```

Plus, for each guard PR:

```bash
cargo xtask <new-check-name> --mode blocking-allowlist
```

The new check must report `no findings` on the PR head and on `master`
post-merge. A PR that introduces a new checker should not also introduce
the first finding — that's a separate refactor PR.

## Do not (per rollout doctrine)

- Combine more than one guard into a single PR.
- Combine a guard with a refactor that would surface findings (the
  checker's first run must be `no findings`).
- Weaken policy ledgers to make a checker green.
- Add `#[allow(clippy::*)]` to silence findings from existing checkers
  (`check-lint-policy` enforces this).
- Hide skipped lanes as passed.
- Make any of these checkers branch-protection blocking before they have
  shipped clean on `master` for at least one PR cycle (per the
  `required-check-migration.md` rule once that doc exists).
- Activate a checker in `blocking-allowlist` mode against `master`
  surfaces without first running it in `advisory` mode for one cycle to
  catch surprise findings.

## Sequencing

The rows can mostly land in parallel — each guard is independent.
Suggested order if a single agent is implementing them serially:

1. **PG-6** first. Smallest change; tightens an existing rule rather
   than adding a new one. Validates the refactor pattern.
2. **PG-1**. Pure new checker; small surface; mirrors a proven shape.
3. **PG-2**. Same shape as PG-1.
4. **PG-3**. Larger because of the matrix-expansion logic; valuable
   because it catches the most common drift class.
5. **PG-4**. Net-new ledger + checker; bigger surface but no other
   dependencies.
6. **PG-5**. Largest. Adds a new workflow + checker + policy section.
   May need to wait until at least one of PG-1 / PG-2 / PG-3 is live so
   the subscription coverage check has something to verify against.

## Bot / CI / self-review operating rules

Same as `RUST_1_95_ROLLOUT.md` "Do not" + "Self-review template", with
two additions specific to this rail:

- **Each new checker lands in `advisory` mode first** in the same PR
  that adds the checker. Promotion to `blocking-allowlist` is a
  separate one-line PR after the checker has shipped clean for one PR
  cycle on `master`.
- **The new checker's CI step lives in a single `Policy gates` job**
  in `ci.yml` (or whichever workflow is canonical for policy
  enforcement). A scratch step in another workflow defeats the unified
  "where do policy gates run" answer.

## References

- [`RUST_1_95_ROLLOUT.md`](RUST_1_95_ROLLOUT.md) — the toolchain +
  workspace-allow burn-down rail (sibling).
- [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md) —
  the strong-clippy-lints activation rail (sibling).
- [`../CLIPPY_POLICY.md`](../CLIPPY_POLICY.md) — Clippy doctrine,
  `check-lint-policy` flow, suppression style.
- [`../FILE_POLICY.md`](../FILE_POLICY.md) — companion policy for
  non-Rust files (the canonical home for PG-4 and PG-6).
- [`../POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md) — shared header
  schema across policy ledgers (PG-4 introduces a new ledger that
  must honour the same shape).
- [`../ci/perl-lsp-rust-1.95-rollout.md`](../ci/perl-lsp-rust-1.95-rollout.md)
  — the initial (historical) rollout plan.
- Sibling-repo proven pattern (shiplog Rust 1.95 / 0.5.0 rollout):
  the proactive-guards quartet landed as shiplog#182 / #183 / #184 /
  #185 + the workflow-allowlist work as shiplog#149 and the
  broad-glob tightening as shiplog#187. Branch shapes, finding kinds,
  unit-test patterns, and CI step wiring are directly translatable.
