# Rust 1.95 → 0.14.0 Remaining Roadmap

> **Canonical post-landing source of truth** for the Rust 1.95 / 0.14.0
> quality rollout. The MSRV / toolchain / `clippy.toml`-msrv bumps have
> shipped (#8509); this doc tracks what remains from here to the 0.14.0
> release tag.
>
> The historical Rust-1.93-framed plan lives at
> [`docs/ci/perl-lsp-rust-1.95-rollout.md`](../ci/perl-lsp-rust-1.95-rollout.md)
> and now delegates to this doc.
>
> Sibling rails:
> - [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md) —
>   strong-clippy-lints activation rail (umbrella #8590).
> - [`RUST_1_95_PROACTIVE_GUARDS.md`](RUST_1_95_PROACTIVE_GUARDS.md) —
>   proactive CI integrity guards rail (umbrella #8662).
>
> Umbrella for this consolidation: **#8663**.

> Doctrine: `ripr` shifts mutation signal left — static, per-PR, and cheaper than
> runtime mutation testing, which remains the backstop for what static analysis
> cannot predict. See [`docs/ci/ripr.md`](../ci/ripr.md) for the canonical framing.

The doc has four sections:

1. [Already landed](#1-already-landed) — the current state facts and what shipped.
2. [Remaining implementation ladder](#2-remaining-implementation-ladder) — single canonical PR list.
3. [Per-rail acceptance contracts](#3-per-rail-acceptance-contracts) — objective / files / commands / forbidden / merge rule / follow-up per row.
4. [Claude / Codex operating contract](#4-claude--codex-operating-contract) — how each PR in this rollout is shaped.

---

## 1. Already landed

### Current state facts (truth sources)

| Surface | Live value | Source file |
|---|---|---|
| Workspace edition | `2024` | [`Cargo.toml`](../../Cargo.toml) |
| Workspace MSRV (`rust-version`) | `1.95` | [`Cargo.toml`](../../Cargo.toml) |
| Workspace version | `0.13.4` | [`Cargo.toml`](../../Cargo.toml) |
| Pinned toolchain channel | `1.95.0` | [`rust-toolchain.toml`](../../rust-toolchain.toml) |
| `clippy.toml` `msrv` | `"1.95"` | [`clippy.toml`](../../clippy.toml) |
| `clippy.toml` `allow-unwrap-in-tests` | `true` ← **debt** | [`clippy.toml`](../../clippy.toml) |
| `policy/clippy-lints.toml` `msrv` | `"1.93"` ← **stale, see M-1** | [`policy/clippy-lints.toml`](../../policy/clippy-lints.toml) |
| Workspace clippy allow set | 5 entries (`collapsible_match`, `manual_range_contains`, `useless_vec`, `vec_init_then_push`, `assertions_on_constants`) | `[workspace.lints]` in [`Cargo.toml`](../../Cargo.toml) |
| `ripr` lane | advisory; never blocking on PR | [`.github/workflows/ripr.yml`](../../.github/workflows/ripr.yml) + [`docs/ci/ripr.md`](../ci/ripr.md) |
| Mutation testing | targeted / nightly / release only; **not** default-PR | [`.github/workflows/`](../../.github/workflows/) + LEM ledgers |
| Release line | `0.13.4`, with `0.14.0` reserved as the **quality posture release vehicle** (not just MSRV bump) | [`CHANGELOG.md`](../../CHANGELOG.md) |

### Ratchet so far

The `@INC` rail wrapped up while the Rust 1.95 bump was being staged.
The lint-debt burn-down progressed in parallel:

| PR | Effect |
| --- | ------ |
| #8509 | Toolchain → 1.95.0; MSRV → 1.95; `clippy.toml` msrv → 1.95; nine 1.94 / 1.95 lints added to workspace allowlist with `priority = 1`. |
| #8511 | Cleaned 10 `unnecessary_sort_by` sites in `perl-{parser,refactoring}`. |
| #8520 | Removed `unnecessary_sort_by` workspace allow; fixed remaining site in `perl-lsp-rs/cli.rs`. |
| #8521 | Removed `useless_conversion` workspace allow. |
| #8522 | Removed `manual_checked_ops` workspace allow; fix in `runtime/workspace_progress.rs` (`checked_div`). |
| #8523 | Fixed `unnecessary_sort_by` in xtask `parser_stats.rs`. |
| #8538 | Removed `while_let_loop` workspace allow; refactored `selection_range.rs` `loop` → `while let`. |
| #8590 | Strong-clippy-lints umbrella issue (rows #8601–#8611). |
| #8657 / #8658 | NON_RUST_LADDER doc; file-policy companion docs. |
| #8661 | Proactive-guards rail doc (#8662 umbrella). |
| #8663 | This consolidation (current PR). |

### Existing policy / CI substrate (don't rebuild)

- `policy/ci-lanes.toml`, `policy/ci-risk-packs.toml`, `policy/ci-budget.toml`, `policy/ci-exceptions.toml`, `policy/ci-lane-whitelist.toml`, `policy/ci-non-rust-allowlist.toml` — CI policy ledgers.
- `policy/clippy-lints.toml`, `policy/clippy-debt.toml` — Clippy ledgers.
- `xtask check-lint-policy` — Clippy ledger gate.
- 39 GitHub workflows under `.github/workflows/` covering PR plan, methodology gate, ripr advisory, ci-gate-self-tests, post-merge ratchets, droid review, release orchestration, etc.

---

## 2. Remaining implementation ladder

Each row is one PR. Branch from clean `origin/master`. **Do not combine.**
Rows are independent unless a `Depends on` cell says otherwise.

| # | Issue | Branch | Title | Lane |
|---|---|---|---|---|
| M-1 | (file new) | `policy/clippy-lints-msrv-reconcile` | `policy(clippy): reconcile policy/clippy-lints.toml msrv to 1.95` | reconcile-toolchain (one-line fix; unblocks `cargo xtask check-lint-policy` on `master`) |
| C-1 | [#8561](https://github.com/EffortlessMetrics/perl-lsp/issues/8561) | `chore/clippy-collapsible-match` | `chore(clippy): clean collapsible_match` + remove workspace allow | clippy allow-set burn-down |
| C-2 | [#8562](https://github.com/EffortlessMetrics/perl-lsp/issues/8562) + [#8559](https://github.com/EffortlessMetrics/perl-lsp/issues/8559) | `chore/clippy-test-only-allows` | `chore(clippy): remove useless_vec / vec_init_then_push / assertions_on_constants allows` | clippy allow-set burn-down |
| C-3 | [#8563](https://github.com/EffortlessMetrics/perl-lsp/issues/8563) | `chore/clippy-manual-range-contains` | `chore(clippy): clean manual_range_contains in perl-ci-hygiene` | clippy allow-set burn-down |
| T-1 | [#8564](https://github.com/EffortlessMetrics/perl-lsp/issues/8564) | `policy/clippy-test-carveout` | `policy(clippy): remove allow-unwrap-in-tests carveout` | no-test-carveouts |
| R-1 | [#8565](https://github.com/EffortlessMetrics/perl-lsp/issues/8565) | `policy/rust-1.95-rustc-floor` | `policy(rust): tighten workspace rustc lint floor` | rustc-floor |
| SCL-* | [#8601–#8611](https://github.com/EffortlessMetrics/perl-lsp/issues/8590) | (per-lint branches) | strong-clippy-lint activations (Tier 1–3) | strong-clippy (sibling rail; see `STRONG_CLIPPY_LINTS_ROLLOUT.md`) |
| DF-1 | (file new) | `docs/clippy-disallowed-fields-seams` | `docs(clippy): define protected-field seams for disallowed_fields` | disallowed-fields prep (mirrors sibling-repo doc pattern) |
| DF-2 | (file new) | `policy/clippy-protected-fields-scaffold` | `policy(clippy): add policy/clippy-protected-fields.toml ledger` | disallowed-fields prep |
| DF-3 | (file new) | `refactor/cache-internals-accessors` | `refactor(cache): introduce accessors for the cache-internals seam` | disallowed-fields prep (smallest-class refactor) |
| DF-4 | (file new) | `policy/clippy-disallowed-fields-activate-cache` | `policy(clippy): activate disallowed_fields for cache-internals seam` | first disallowed-fields slice |
| N-1 | [#8567](https://github.com/EffortlessMetrics/perl-lsp/issues/8567) | `policy/no-panic-design` | `docs/policy(panic): design no-panic exact-identity baseline` | no-panic |
| N-2 | [#8569](https://github.com/EffortlessMetrics/perl-lsp/issues/8569) | `feat/no-panic-xtask` | `feat(xtask): no-panic baseline + check command` | no-panic |
| N-3 | [#8571](https://github.com/EffortlessMetrics/perl-lsp/issues/8571) | `policy/no-panic-baseline-init` | `policy(panic): generate no-new-debt baseline` | no-panic |
| F-1 | [#8574](https://github.com/EffortlessMetrics/perl-lsp/issues/8574) | `policy/file-allowlist-tightening` | `policy(files): narrow non-rust-allowlist coverage` | file-policy |
| PG-1..PG-6 | [#8662](https://github.com/EffortlessMetrics/perl-lsp/issues/8662) | (per-row branches) | proactive integrity guards (label enforcement, risk-pack referential integrity, lane mappings, workflow-allowlist, ci-actuals coverage, broad-glob justification) | proactive-guards (sibling rail; see `RUST_1_95_PROACTIVE_GUARDS.md`) |
| C-CI | (file new) | `ci/policy-gate-wiring` | `ci: wire policy checkers into a Policy gates job` | ci-policy-wiring (consolidates all the `cargo xtask check-*` calls into one CI job) |
| C-RM | (file new) | `ci/ripr-and-mutation-routing` | `ci: tighten ripr routing + confirm mutation off normal PRs` | ci-routing |
| C-AC | (file new) | `refactor/rust-1.95-ast-ast-cleanups` | `refactor(rust-1.95): targeted API cleanup in AST / LSP paths` | rust-1.95-api-cleanup (behavior-preserving only) |
| C-LL | (file new) | `ci/learned-lem-estimates` | `ci: use CI actuals to calibrate PR Plan LEM` | lem-calibration |
| RP-1 | [#8576](https://github.com/EffortlessMetrics/perl-lsp/issues/8576) | `release/0.14.0-prep` | `release: prepare 0.14.0` | release-prep |
| RP-2 | [#8579](https://github.com/EffortlessMetrics/perl-lsp/issues/8579) | `release/0.14.0-dry-run` | `release: validate publish readiness` | release-prep |

**Sequencing recommendation** (smallest-first; surface the easy wins):

```text
M-1 → T-1 (or one of C-1/C-2/C-3) → PG-6 → R-1 → C-CI → N-1 → DF-1
   → (then mid-weight rows) PG-1/PG-2/PG-3 → N-2 → DF-2
   → (heavier rows)         PG-4/PG-5 → DF-3 → DF-4 → N-3 → F-1
   → C-RM → C-AC → C-LL → RP-1 → RP-2
```

Adjust as issues are triaged. The point is **no row blocks the next**
unless its acceptance contract explicitly says so.

---

## 3. Per-rail acceptance contracts

Each contract has the same six fields. Where a contract is identical
to a sibling-rail doc, the contract cell references that doc.

### M-1: reconcile policy/clippy-lints.toml msrv

| Slot | Value |
|---|---|
| Objective | Fix `cargo xtask check-lint-policy` failing on `master` (`workspace.package.rust-version (1.95) must match policy/clippy-lints.toml msrv (1.93)`). |
| Files expected | `policy/clippy-lints.toml` (one-line bump). |
| Commands | `cargo xtask check-lint-policy`; `git diff --check`. |
| Forbidden scope | Cargo.toml, rust-toolchain.toml, clippy.toml, workflows, Rust source. |
| Merge rule | Green CI; one-line diff; no review needed beyond title check. |
| Follow-up | None. After this lands, the lint policy gate proves itself on every subsequent PR. |

### C-1, C-2, C-3: clippy allow-set burn-down

| Slot | Value |
|---|---|
| Objective | Remove one workspace clippy allow entry per PR by fixing the call sites it suppresses. |
| Files expected | One or more `crates/perl-*/src/**/*.rs` files; `Cargo.toml` allow-set entry removal. |
| Commands | `cargo clippy --workspace --lib --no-deps -- -D warnings`; `cargo test` for touched crates; `git diff --check`. |
| Forbidden scope | Multiple lints in one PR; unrelated cleanup; new `#[allow(clippy::*)]` (use `#[expect(..., reason = "policy:...")]` if absolutely needed). |
| Merge rule | Green CI; one lint per PR; no allow-entry resurrection. |
| Follow-up | Next C-* row. |

### T-1: remove allow-unwrap-in-tests carveout

| Slot | Value |
|---|---|
| Objective | Remove `allow-unwrap-in-tests = true` from `clippy.toml`. Verify tests already use `must_some` / fallible helpers (any remaining `.unwrap()` in tests is a finding to fix in this PR). |
| Files expected | `clippy.toml`; possibly tests where `.unwrap()` needs replacement. |
| Commands | `cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs`; `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib`. |
| Forbidden scope | New test carveouts; broad refactor in tests beyond `.unwrap()` → fallible replacement. |
| Merge rule | Green CI; `clippy.toml` shows the carveout removed. |
| Follow-up | Strong-clippy rail can now include test-targeted lints. |

### R-1: tighten workspace rustc lint floor

| Slot | Value |
|---|---|
| Objective | Promote `unexpected_cfgs` to `warn`; add `unused_must_use = "deny"` if not already; survey other 1.95 stabilizations worth denying. |
| Files expected | `Cargo.toml` `[workspace.lints]` block. |
| Commands | `cargo clippy --workspace --lib --no-deps -- -D warnings`; full CI. |
| Forbidden scope | Clippy lint activations beyond 1.95 deweighting (those belong to the strong-clippy rail). |
| Merge rule | Green CI. |
| Follow-up | Sibling strong-clippy rail rows (#8601–#8611). |

### SCL-*: strong-clippy lint activations

Defer to [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md) — that doc has per-row acceptance contracts.

### DF-1: define protected-field seams

| Slot | Value |
|---|---|
| Objective | Document the protected field classes that `clippy::disallowed_fields` will eventually enforce (redaction internals, bundle paths, trust receipts, source opaque IDs, cache internals, policy ledger metadata) with invariant / boundary / today's surface / failure-mode per class. Doc only; no lint activation. |
| Files expected | `docs/CLIPPY_PROTECTED_FIELDS.md` (new); `docs/CLIPPY_POLICY.md` (See-also cross-link); `policy/clippy-lints.toml` `[[planned]] disallowed_fields` `reason` field update. |
| Commands | `cargo xtask check-lint-policy`; `cargo xtask check-policy-schemas` (if available); `git diff --check`. |
| Forbidden scope | `clippy.toml` change; `[lints]` workspace block change; Rust source change. |
| Merge rule | Green CI; doc reviewable as a design anchor. |
| Follow-up | DF-2 scaffolds the policy ledger; DF-3 refactors the first seam; DF-4 activates the lint for that one class. |

### DF-2: scaffold policy/clippy-protected-fields.toml

| Slot | Value |
|---|---|
| Objective | Add the policy ledger that lists protected fields per class, without yet wiring into `clippy.toml`. `cargo xtask check-policy-schemas` (or equivalent) accepts the new ledger header. |
| Files expected | `policy/clippy-protected-fields.toml` (new); brief cross-link from `docs/CLIPPY_PROTECTED_FIELDS.md`. |
| Commands | `cargo check -p xtask --locked`; `cargo xtask check-lint-policy`. |
| Forbidden scope | `clippy.toml disallowed-fields` wiring; Rust accessor refactor. |
| Merge rule | Green CI; ledger header validated. |
| Follow-up | DF-3 picks the smallest class (cache internals) and adds accessors. |

### DF-3, DF-4: first-class refactor + activation

| Slot | Value |
|---|---|
| Objective | DF-3 introduces accessors for the smallest protected class (likely cache internals); DF-4 activates `clippy::disallowed_fields` for that one class via `clippy.toml`. |
| Files expected | DF-3: `crates/shiplog-cache/...` or the local equivalent (any private-to-pub-accessor swap), focused tests asserting the accessor invariant. DF-4: `clippy.toml` disallowed-fields list; possibly `policy/clippy-exceptions.toml` if any expected sites need policy-ID'd suppressions. |
| Commands | DF-3: `cargo clippy --workspace`; `cargo test` for touched crate; `git diff --check`. DF-4: full Clippy gate + lint policy. |
| Forbidden scope | Activating multiple classes in one PR; bypassing the accessor refactor. |
| Merge rule | Green CI; first run of `disallowed_fields` on this class must be `no findings` on `master`. |
| Follow-up | Repeat for next class (redaction internals, bundle paths, etc.). |

### N-1, N-2, N-3: no-panic ladder

| Slot | Value |
|---|---|
| Objective | N-1 defines the exact-identity scheme. N-2 introduces `cargo xtask check-no-panic-family` in advisory mode reading `policy/no-panic-baseline.toml` + `policy/no-panic-allowlist.toml`. N-3 generates the no-new-debt baseline once on clean `master`. |
| Files expected | N-1: docs only. N-2: `xtask/src/tasks/no_panic*.rs`, ledger TOMLs, tests. N-3: `policy/no-panic-baseline.toml` (generated) marked `linguist-generated` in `.gitattributes`. |
| Commands | N-1: `cargo xtask check-lint-policy`. N-2: `cargo test -p xtask`; `cargo xtask check-no-panic-family --mode advisory`. N-3: baseline-generation command in advisory; commit baseline; rerun `--mode blocking` returns clean. |
| Forbidden scope | Baseline reset outside N-3. Lint activation that surfaces new debt without baseline. |
| Merge rule | N-1 + N-2 advisory must ship before N-3. |
| Follow-up | Per-crate panic burn-downs are individual PRs after N-3. |

### F-1: non-rust-allowlist tightening

| Slot | Value |
|---|---|
| Objective | Remove stale entries from `policy/ci-non-rust-allowlist.toml`; narrow broad globs; add `review_after` where supported; surface shader/FFI/WASM explicitly. |
| Files expected | `policy/ci-non-rust-allowlist.toml`. |
| Commands | `cargo xtask check-lint-policy`; whatever non-rust-allowlist checker exists (advisory first). |
| Forbidden scope | New companion ledgers (those are PG-4 territory); Rust source. |
| Merge rule | Green CI; live file inventory still satisfied. |
| Follow-up | PG-4 net-new workflow-allowlist ledger; PG-6 broad-glob tightening. |

### PG-1 .. PG-6: proactive integrity guards

Defer to [`RUST_1_95_PROACTIVE_GUARDS.md`](RUST_1_95_PROACTIVE_GUARDS.md) — that doc has per-row acceptance contracts.

### C-CI: wire policy checkers into a Policy gates job

| Slot | Value |
|---|---|
| Objective | Consolidate every `cargo xtask check-*` call into a single `Policy gates` job in `ci.yml` (or whichever workflow is canonical) so a contributor sees one pass/fail for policy compliance. Mirrors the sibling-repo pattern. |
| Files expected | `.github/workflows/<canonical>.yml` adding a new job. |
| Commands | `cargo xtask check-lint-policy`; any other live `check-*` commands at the time of this PR (the set grows as guard rows land). |
| Forbidden scope | New checkers (those belong to PG-* rows); routing changes for other jobs. |
| Merge rule | Green CI; new job reports `no findings`. |
| Follow-up | Each subsequent guard PR adds a step to this job. |

### C-RM: ripr routing + mutation lane confirmation

| Slot | Value |
|---|---|
| Objective | Confirm `ripr.yml` triggers route docs-only / fixture-only PRs around the lane; confirm mutation testing remains gated off normal PRs (label / nightly / release only). Adjust `policy/ci-lanes.toml` and workflow `if:` blocks as needed. |
| Files expected | `policy/ci-lanes.toml`, `policy/ci-risk-packs.toml`, `.github/workflows/ripr.yml`, `.github/workflows/mutation-testing*.yml`. |
| Commands | `cargo xtask check-lint-policy`; PG-* guards if landed; lane-map review. |
| Forbidden scope | Making `ripr` blocking; putting mutation on default PR. |
| Merge rule | Green CI; ripr still advisory; mutation still off-default. |
| Follow-up | PG-3 (lane mappings) + PG-5 (actuals coverage) keep this honest. |

### C-AC: Rust 1.95 API cleanup in AST / LSP paths

| Slot | Value |
|---|---|
| Objective | Apply behavior-preserving Rust 1.95 idioms (`if let` guards, `push_mut`, `cold_path`, etc.) in AST / parser / provider / LSP paths where they materially reduce review load. **Behavior-preserving only.** |
| Files expected | `crates/perl-ast*/...`, `crates/perl-parser*/...`, `crates/perl-lsp-rs*/...` (focused; no broad sweep). |
| Commands | `cargo clippy --workspace --lib --no-deps -- -D warnings`; targeted tests for touched crates; UX scenarios; `git diff --check`. |
| Forbidden scope | Feature-bearing changes; broad refactor; combined with any policy / lint activation PR. |
| Merge rule | Green CI; explicit "behavior preserved" claim in PR body. |
| Follow-up | None mandatory; further cleanups are individual PRs. |

### C-LL: learned LEM estimates

| Slot | Value |
|---|---|
| Objective | Have the PR plan emit `estimate_source = "learned"` (or `"static"`) and use actuals-backed numbers when available; static fallback remains. |
| Files expected | `xtask/src/tasks/pr_plan*.rs` (or equivalent), `policy/ci-budget.toml` if new fields. |
| Commands | `cargo test -p xtask`; sample PR plan JSON shows the new field; CI plan dry-run if available. |
| Forbidden scope | Hard-enforce LEM below the existing 125 LEM ceiling before calibration. |
| Merge rule | Green CI; static fallback still works on a fresh checkout with no actuals. |
| Follow-up | None mandatory; per-lane calibration in subsequent PRs as actuals accumulate. |

### RP-1, RP-2: 0.14.0 release-prep

| Slot | Value |
|---|---|
| Objective | RP-1 prepares 0.14.0: version bump, CHANGELOG finalization, release readiness doc. RP-2 proves `cargo package` + `cargo publish --dry-run` on all publishable crates in dependency order. |
| Files expected | RP-1: `Cargo.toml` version bump (workspace + members as needed); `CHANGELOG.md`; `docs/release/0.14.0-readiness.md`. RP-2: dry-run receipts; release evidence doc updates. |
| Commands | RP-1: package-version audit (whatever the local equivalent is); CHANGELOG matches version. RP-2: `cargo package --locked` per crate; `cargo publish -p <crate> --dry-run` for the foundation crate; subsequent crates need their internal deps to be on crates.io before their dry-run resolves (interleaved publish, not pre-tag gate — see `docs/release/RUNBOOK.md` and sibling-repo experience for the same constraint). |
| Forbidden scope | Tagging; pushing crates; CI routing changes. |
| Merge rule | RP-1: green CI + readiness doc complete. RP-2: green CI + dry-run receipts committed. |
| Follow-up | Owner-gated: `git tag v0.14.0`; `release.yml` builds artifacts; install-smoke; crates.io publish in dependency order. |

---

## 4. Claude / Codex operating contract

Every PR in this rollout obeys the following rules. Deviation is a
review-blocker, not a debate.

### One PR per objective

- Each ladder row above is one PR. Do not bundle a lint activation
  with a baseline reset, a release bump with an API cleanup, or a
  policy schema change with a workflow routing change. If a row's
  acceptance contract names two surfaces (e.g. DF-1 touches the doc
  and the `[[planned]]` `reason` field), that's the *narrow* allowed
  scope — anything else is a different row.

### Draft first; self-review before "ready for review"

- Every rollout PR opens as a **draft**.
- Post the self-review template (below) as a PR comment **before**
  marking ready.
- Marking ready before CI is green is fine if the PR is awaiting bot
  review; marking ready while a real check fails is not.

### Forbidden cross-pollination

- **No provider-cutover, native-tooling, Codecov, or rail-doc PRs
  mixed in.** If your branch off `origin/master` was clean and your
  diff stayed within the row's `Files expected` cell, this is
  automatic.
- **No Dependabot mixing.** Dependabot PRs land separately; they are
  not auto-rebased into this rollout.
- **No bare `#[allow(clippy::*)]`.** Use
  `#[expect(..., reason = "policy:<id>")]` per
  [`docs/CLIPPY_POLICY.md`](../CLIPPY_POLICY.md). Scoped temporary
  debt goes in `policy/clippy-debt.toml` with `expires`.
- **No test carveouts.** The `allow-unwrap-in-tests = true` carveout
  is being removed in T-1; do not add new ones.
- **No no-panic baseline reset** except in the dedicated N-3 PR.
  Burn-down PRs may *only* drop entries that disappeared from source;
  they do not regenerate the baseline.
- **`ripr` advisory only.** No row in this rollout makes `ripr`
  branch-protection blocking. Real mutation evidence lives in
  targeted / nightly / release lanes per the doctrine quoted at the
  top of this doc.

### Acceptance gate (every PR)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --lib --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --test-threads=1
cargo xtask fmt
cargo xtask check-lint-policy
git diff --check
```

Per-row additions (e.g. `cargo xtask check-no-panic-family --mode advisory`
for the N-* rows, or `cargo xtask <new-check> --mode advisory` for any
PG-* row that adds a checker) follow the existing `xtask --help`
surface.

### Self-review template (post as a comment before marking ready)

```markdown
## Self-review

- Scope matches PR title:
- Files touched are expected (per the row's acceptance contract):
- No unrelated cleanup:
- Policy changes are intentional:
- No `clippy::*` test carveouts added:
- No bare `#[allow(clippy::...)]` added:
- No-panic baseline handling is scoped (only N-3 may reset it):
- File-policy changes are narrow:
- CI lanes are risk-pack appropriate:
- `ripr` vs mutation boundary preserved:
- Local validation:
- CI status:
- Bot comments addressed:
- Follow-ups:
```

### Bot / review loop

For each PR, inspect the current check and review state before
claiming green:

```bash
gh pr view <PR> --json statusCheckRollup,reviewDecision,mergeStateStatus
gh pr checks <PR> --watch
```

If CI fails, identify the first real failing command, reproduce
locally if possible, fix only that failure, rerun the matching local
gate, push, and check bot comments again. Bot comments:

- Fix real defects.
- Answer false positives with evidence.
- Fix cheap, in-scope style comments.
- Defer out-of-scope requests with a follow-up.
- Mark stale comments only after verifying current HEAD.

### "Done" definition for the whole rollout

The 0.14.0 release is the vehicle for the *completed quality posture*,
not merely the MSRV bump that already shipped in #8509. The rollout is
done when:

- Workspace clippy allow-set is at 0 entries (C-1/C-2/C-3 + sibling
  strong-clippy rail).
- `allow-unwrap-in-tests` is removed (T-1).
- `cargo xtask check-no-panic-family` runs in `no-new-debt` mode on
  every PR (N-1 + N-2 + N-3).
- `policy/ci-non-rust-allowlist.toml` is tightened with explicit
  receipts and reviewed entries (F-1).
- Proactive integrity guards PG-1..PG-6 are live in the `Policy gates`
  job (`RUST_1_95_PROACTIVE_GUARDS.md`).
- Strong-clippy rail Tier 1–3 lints are active (`STRONG_CLIPPY_LINTS_ROLLOUT.md`).
- `disallowed_fields` is active for at least the first protected class
  (DF-1..DF-4).
- RP-1 + RP-2 complete; owner tags `v0.14.0` and runs
  `release.yml` + install-smoke + crates.io publish per
  [`docs/release/RUNBOOK.md`](../release/RUNBOOK.md).

---

## References

- [`../ci/perl-lsp-rust-1.95-rollout.md`](../ci/perl-lsp-rust-1.95-rollout.md) — historical Rust-1.93-framed plan; now a pointer to this doc.
- [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md) — strong-clippy lint activation rail (sibling, umbrella #8590).
- [`RUST_1_95_PROACTIVE_GUARDS.md`](RUST_1_95_PROACTIVE_GUARDS.md) — proactive CI integrity guards rail (sibling, umbrella #8662).
- [`../CLIPPY_POLICY.md`](../CLIPPY_POLICY.md) — Clippy doctrine, suppression style, `check-lint-policy` flow.
- [`../NO_PANIC_POLICY.md`](../NO_PANIC_POLICY.md) — no-panic policy and baseline shape.
- [`../FILE_POLICY.md`](../FILE_POLICY.md) — non-Rust file policy and allowlist semantics.
- [`../POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md) — common header schema across policy ledgers.
- [`../ci/ripr.md`](../ci/ripr.md) — `ripr` lane doctrine and Rust 1.95 rollout note.
- [`../ci/lem-budgeting.md`](../ci/lem-budgeting.md) — LEM cost model and runner multipliers.
- [`../ci/test-evidence-lanes.md`](../ci/test-evidence-lanes.md) — evidence-lane shapes (PR fast / broad / nightly / label-gated).
- [`../release/RUNBOOK.md`](../release/RUNBOOK.md) — release execution runbook.
- `policy/clippy-lints.toml`, `policy/clippy-debt.toml` — live policy ledgers.
