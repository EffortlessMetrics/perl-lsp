# Rust 1.95 / Next-Minor Rollout

> **Companion to** [`docs/ci/perl-lsp-rust-1.95-rollout.md`](../ci/perl-lsp-rust-1.95-rollout.md).
> That doc is the **initial rollout plan** (historical, planned-from-1.93
> framing). This doc is the **post-landing improvement spec** — what's
> remaining now that MSRV / toolchain / `clippy.toml`-msrv have shipped
> (#8509). Each row in the ladder below is an agent-actionable spec; a
> haiku scout can file it as a GitHub issue and a sonnet builder can
> implement it without re-discovering the call sites.

Continuation of the Rust 1.93 → 1.95 quality control plane. This doc tracks
both the ratchet (what already landed) and the remaining ladder (what's
next), so subsequent PRs do **one thing**.

> Doctrine, repeated from the cross-repo CI economics plan:
> **`ripr` is the PR-time static oracle-exposure filter; it is not a
> replacement for mutation testing. Mutation remains runtime evidence
> for targeted PRs, nightly lanes, and release readiness.**

## Current vs target

| Layer                        |                         Current |                          Target | Status                 |
| ---------------------------- | ------------------------------: | ------------------------------: | ---------------------- |
| Edition                      |                       Rust 2024 |                       Rust 2024 | done                   |
| Workspace MSRV               |                          1.95.0 |                          1.95.0 | done (#8509)           |
| Toolchain                    |                          1.95.0 |                          1.95.0 | done (#8509)           |
| `clippy.toml` `msrv`         |                            1.95 |                            1.95 | done (#8509)           |
| Workspace clippy allow set   |                       5 entries |                       0 entries | in flight (#8538 +)    |
| `clippy.toml` test carveouts | `allow-unwrap-in-tests = true`  |                         removed | todo (PR T-1 below)    |
| Workspace lints — rustc      | `unsafe_op_in_unsafe_fn = deny` |       broader 1.95 lint floor   | partial (PR R-1 below) |
| No-panic policy infra        |                          absent |     exact-identity baseline     | todo (PR N-series)     |
| Non-Rust file policy         |    `policy/non-rust-allowlist.toml` present, broad |        tightened + reviewed     | partial                |
| CI lane whitelist / LEM      |                         present |             Rust 1.95 budgeted  | partial                |
| Release line                 |                          0.13.4 |             next minor (TBD)    | planned (PR R-prep)    |

## What already landed

The `@INC` rail wrapped up while the Rust 1.95 bump was being staged. The
ratchet so far:

| PR    | Effect |
| ----- | ------ |
| #8509 | Toolchain → 1.95.0; MSRV → 1.95; `clippy.toml` msrv → 1.95; nine 1.94/1.95 lints added to workspace allowlist with `priority = 1`. |
| #8511 | Cleaned 10 `unnecessary_sort_by` sites in `perl-{parser,refactoring}`. |
| #8520 | Removed `unnecessary_sort_by` workspace allow; fixed remaining site in `perl-lsp-rs/cli.rs`. |
| #8521 | Removed `useless_conversion` workspace allow. |
| #8522 | Removed `manual_checked_ops` workspace allow; fix in `runtime/workspace_progress.rs` (`checked_div`). |
| #8523 | Fixed `unnecessary_sort_by` in xtask `parser_stats.rs` (caught after #8511's lib-only survey). |
| #8538 | Removes `while_let_loop` workspace allow; refactored `selection_range.rs` `loop` → `while let`. |

After #8538 lands, the workspace allow set is **5 entries**: `collapsible_match`,
`manual_range_contains`, `useless_vec`, `vec_init_then_push`, `assertions_on_constants`.

## Remaining PR ladder

Each row is one PR. Branch from clean `origin/master`. Do **not** combine.

| #     | Tracking                          | Branch                                          | Title                                                            | Notes                                                 |
| ----- | --------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------- |
| C-1   | [#8561](https://github.com/EffortlessMetrics/perl-lsp/issues/8561) | `chore/clippy-collapsible-match`                | `chore(clippy): clean collapsible_match` + remove workspace allow | 3 lib sites (`perl-regex`, `perl-pragma`, `perl-parser-pest`) |
| C-2   | [#8562](https://github.com/EffortlessMetrics/perl-lsp/issues/8562) + [#8559](https://github.com/EffortlessMetrics/perl-lsp/issues/8559) | `chore/clippy-test-only-allows`                 | `chore(clippy): remove useless_vec / vec_init_then_push / assertions_on_constants allows` | Test-code cleanup; replaces `assert!(false, …)` with `panic!(…)` in test files that already allow `clippy::panic`. #8559 covers the `assertions_on_constants` subset; #8562 covers `useless_vec` + `vec_init_then_push`. |
| C-3   | [#8563](https://github.com/EffortlessMetrics/perl-lsp/issues/8563) | `chore/clippy-manual-range-contains`            | `chore(clippy): clean manual_range_contains in perl-ci-hygiene`  | The single remaining surface; may also need `expect_used` cleanup in the same crate |
| T-1   | [#8564](https://github.com/EffortlessMetrics/perl-lsp/issues/8564) | `policy/clippy-test-carveout`                   | `policy(clippy): remove allow-unwrap-in-tests carveout`          | `clippy.toml` removes `allow-unwrap-in-tests = true`; verify tests already use `must_some` / fallible helpers |
| R-1   | [#8565](https://github.com/EffortlessMetrics/perl-lsp/issues/8565) | `policy/rust-1.95-rustc-floor`                  | `policy(rust): tighten workspace rustc lint floor`               | Promote `unexpected_cfgs` to `warn`, add `unused_must_use = "deny"` (verify it's not already), survey other 1.95 stabilizations worth denying. Clippy lint activations beyond 1.95 deweighting are split into the **strong-clippy-lints** rail — see [`STRONG_CLIPPY_LINTS_ROLLOUT.md`](STRONG_CLIPPY_LINTS_ROLLOUT.md) (umbrella #8590, rows #8601-#8611). |
| N-1   | [#8567](https://github.com/EffortlessMetrics/perl-lsp/issues/8567) | `policy/no-panic-design`                        | `docs/policy(panic): design no-panic exact-identity baseline`    | Docs-only — defines `path + family + selector_kind + selector_callee + snippet + count` identity; describes baseline file format + matching rules. No code yet. |
| N-2   | [#8569](https://github.com/EffortlessMetrics/perl-lsp/issues/8569) | `feat/no-panic-xtask`                           | `feat(xtask): no-panic baseline + check command`                 | `cargo xtask check-no-panic-family` reads `policy/no-panic-baseline.toml` + `policy/no-panic-allowlist.toml`; advisory mode first |
| N-3   | [#8571](https://github.com/EffortlessMetrics/perl-lsp/issues/8571) | `policy/no-panic-baseline-init`                 | `policy(panic): generate no-new-debt baseline`                   | Run once from clean `master`; `mode = "no-new-debt"`; `.gitattributes` marks baseline `linguist-generated`; **only baseline PR** |
| F-1   | [#8574](https://github.com/EffortlessMetrics/perl-lsp/issues/8574) | `policy/file-allowlist-tightening`              | `policy(files): narrow non-rust-allowlist coverage`              | Remove stale entries; narrow broad globs; add `review_after` where supported; surface shader/FFI/WASM explicitly |
| RP-1  | [#8576](https://github.com/EffortlessMetrics/perl-lsp/issues/8576) | `release/next-minor-prep`                       | `release: prepare next minor for Rust 1.95`                      | Version decision (see below); CHANGELOG; release evidence doc |
| RP-2  | [#8579](https://github.com/EffortlessMetrics/perl-lsp/issues/8579) | `release/next-minor-dry-run`                    | `release: validate publish readiness`                            | `cargo package --locked`, `cargo publish --dry-run`, evidence receipt under `docs/release/` |

## Version decision (for PR RP-1)

Current package version is `0.13.4`. The full project Cargo.toml lineage
suggests the next user-facing release after the MSRV bump should be a
**minor** bump (semver: raising MSRV is user-visible).

Two cases:

| Case                                                | Version move          |
| --------------------------------------------------- | --------------------- |
| `0.14.x` has not actually shipped                   | `0.13.4 → 0.14.0`     |
| `0.14.0` already shipped or is reserved             | `0.13.4 → 0.15.0`     |

Reconcile against `CHANGELOG.md` and any git tag named `v0.14.*` before
choosing. Document the decision in the PR body.

## What Rust 1.95 buys us in this repo

| Rust 1.95 item                  | perl-lsp use                                                                                                |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `if let` guards                 | Provider dispatch, parser route selection, `EffectiveIncContext` lane selection, completion fixture routing |
| `Vec::push_mut` / `insert_mut`  | Receipt builders, diagnostic vec builders, status/scorecard report assembly                                 |
| Atomic `update` / `try_update`  | Runtime metrics, once-warn counters, cancellation flags                                                     |
| `cfg_select!`                   | Platform-specific paths in `perl-uri`, `perl-subprocess-runtime`, Windows-vs-POSIX in `fetch_perl_inc`      |
| `cold_path`                     | Diagnostic-emit paths, `unreachable!` substitutes, parser error-recovery branches                           |
| Clippy 1.95                     | `manual_checked_ops` (done), `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, others surveyed in #8508 |

Use these where they reduce review load or close a small lint debt. **Behavior-preserving only** in this rollout — feature-bearing uses get their own PR per usual.

## Acceptance gates (every PR)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --lib --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --test-threads=1
cargo xtask fmt
git diff --check
```

Per-PR additions (e.g. policy reports) follow the existing xtask command
names; see `xtask --help` for the current command surface.

## Do not (per cross-repo rollout doctrine)

- Combine MSRV bump, lint activation, no-panic baseline, release bump, or
  code cleanup into one PR.
- Weaken schemas or policy to satisfy CI.
- Add `#[allow(clippy::...)]` suppressions without a debt-policy entry.
- Reset a no-panic baseline outside its dedicated PR.
- Make `ripr` branch-protection blocking.
- Replace mutation testing with `ripr`.
- Put full mutation on ordinary PRs.
- Hide skipped lanes as passed.

## Self-review template (every PR)

```markdown
## Self-review

- Scope matches PR title:
- Files touched are expected:
- No unrelated cleanup:
- Policy changes are intentional:
- No `clippy::*` test carveouts added:
- No bare `#[allow(clippy::...)]` added:
- No-panic baseline handling is scoped:
- File-policy changes are narrow:
- CI lanes are risk-pack appropriate:
- `ripr` vs mutation boundary preserved:
- Local validation:
- CI status:
- Bot comments addressed:
- Follow-ups:
```

## References

- Cross-repo doctrine: this rollout follows the same pattern as the
  Rust 1.95 quality wave in adjacent repos (BitNet-rs, etc.).
- `docs/ci/codecov-rollout.md` — the parallel CI/Codecov cleanup ladder.
- `docs/project/status/module_resolution.md` — the `@INC` rail summary
  that this rollout sits on top of.
- `policy/clippy-lints.toml`, `policy/clippy-debt.toml` — the live
  policy ledgers.
