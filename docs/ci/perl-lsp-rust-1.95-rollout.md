# Rust 1.95 / 0.14.0 rollout map

This document is the control map for moving `perl-lsp` from Rust 1.93 / toolchain
1.93.1 to Rust 1.95.0 and preparing the next minor release line, 0.14.0. It is
intentionally documentation-only: no MSRV, workflow, lint, release, or code changes
belong in this PR.

## Current and target state

| Layer | Current | Target | Status |
|---|---:|---:|---|
| Edition | 2024 | 2024 | done |
| MSRV | 1.93 | 1.95.0 | planned |
| Toolchain | 1.93.1 | 1.95.0 | planned |
| Release line | 0.13.4 | 0.14.0 | planned |
| Clippy panic-family | partly active | strict + no test carveouts | partial |
| `collapsible_if` | broad allow | debt ledger / cleanup | todo |
| Rust 1.93 rustc lints | planned/tracked | active | todo |
| Rust 1.95 Clippy lints | planned | active or ratcheted | todo |
| No-panic allowlist | missing/incomplete | exact counted no-new-debt | todo |
| Non-Rust allowlist | missing/incomplete | blocking file policy | todo |
| ripr | advisory, installed in workflow | advisory + better routing | partial |
| CI economics | LEM/risk packs exist | tuned + actuals-backed | partial |

Truth sources for the current state are `Cargo.toml`, `rust-toolchain.toml`,
`clippy.toml`, and `policy/clippy-lints.toml`. Do not infer release, MSRV, or lint
state from this document after implementation begins; update this map as the ladder
lands.

## Why Rust 1.95 matters here

| Rust 1.95 item | Repo-specific value |
| --- | --- |
| `if let` guards | Parser/AST walkers, semantic extraction, import visibility, refactor preconditions, LSP provider routing. |
| `Vec::push_mut` / `insert_mut` | Diagnostic/report builders, semantic facts, scorecards, CI receipts, LSP response builders. |
| Atomic `update` / `try_update` | Session/cache counters, memory-pressure counters, cancellation or stream-state metrics. |
| `cfg_select!` | Windows/Unix path behavior, subprocess handling, VS Code install surfaces, native/virtual URI handling. |
| `cold_path` | Parser recovery, malformed URI/path handling, failed subprocess/perlcritic paths, policy failure reporting. |
| Clippy 1.95 | `manual_checked_ops`, `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, and future `disallowed_fields`. |

The AST-heavy parts of this repo benefit especially from `if let` guards because the
branch shape and semantic precondition stay together. That matters in parser recovery,
semantic walkers, import visibility extraction, and LSP provider routing, where a split
condition can hide which fact justified a branch.

## Operating rule

The first implementation PR after this documentation PR is a Rust 1.95 compatibility
spike. No MSRV bump, lint activation, no-panic baseline reset, release bump, or API
cleanup happens in the same PR.

## Queue control

The Rust 1.95 rollout must stay separate from product/test PRs. If unrelated draft PRs
are open, either let them finish first or start each rollout PR from a clean `master`
and expect to rebase after they merge. Do not fold rollout changes into unrelated UX,
critic, parser, or provider branches.

## Policy status to carry into the ladder

- **Clippy ledger:** `policy/clippy-lints.toml` already tracks active strict lints,
  debt such as the broad `collapsible_if` allow, and planned Rust 1.94/1.95 flips.
- **Test unwrap carveout:** `clippy.toml` still contains `allow-unwrap-in-tests = true`
  while the target posture is no test carveouts. Removing it is a dedicated policy PR,
  not part of the MSRV bump.
- **No-panic policy:** exact counted identity must land before any baseline reset.
  Allowlist and baseline matching should consume `path + family + selector_kind +
  selector_callee + snippet` counts before reporting new debt.
- **File policy:** Rust and `xtask` remain the default implementation surface. Non-Rust
  surfaces are allowed by receipt through a ledger before CI enforces the policy.
- **ripr:** keep ripr advisory. Use it as a fast PR-time exposure filter, not as a
  branch-protection replacement for mutation testing.
- **CI economics:** LEM budget policy, lane intent, risk packs, gate receipts, CI
  actuals, PR smoke, merge-gate shards, UX tests, memory smoke, and Windows guardrails
  already exist. The rollout should tune those controls rather than add a parallel
  process.

## PR ladder and acceptance gates

| Step | Branch | Purpose | Acceptance |
|---:|---|---|---|
| 1 | `docs/rust-1.95-rollout` | Map the rollout. | `cargo check -p xtask --locked`; `cargo xtask check-lint-policy`; `git diff --check` |
| 2 | `probe/rust-1.95-compat` | Run the current repo under Rust 1.95.0 before changing declared MSRV. | Rust 1.95 fmt, check, all-features check, clippy, tests, lint policy, PR-fast receipt, diff check |
| 3 | `chore/msrv-rust-1.95` | Raise MSRV/toolchain metadata to Rust 1.95.0 without release bump. | Rust 1.95 fmt, workspace check, lint policy, PR-fast receipt, diff check |
| 4 | `policy/rust-1.95-lints` | Activate Rust compiler lint floor. | Workspace check, lint policy, PR-fast receipt |
| 5 | `policy/clippy-rust-1.95-ratchets` | Measure and activate clean or cheap Clippy 1.94/1.95 ratchets. | Lint policy and workspace clippy without global `-D warnings` for warn-stage debt |
| 6 | `policy/no-test-clippy-carveouts` | Remove the test unwrap carveout and add fallible helper path. | Policy config consistent; no full test-suite migration required |
| 7 | `policy/no-panic-exact-identity` | Make no-panic matching exact and counted before baseline. | `cargo test -p xtask no_panic --locked`; `cargo xtask check-no-panic-family` |
| 8 | `policy/no-panic-baseline` | Add generated no-panic baseline and no-new-debt mode. | Baseline reset, no-panic check, no-panic tests, diff check |
| 9 | `policy/no-panic-diagnostics` | Improve no-panic failure reports. | No-panic tests, no-panic check, markdown/json report existence |
| 10 | `policy/non-rust-file-ledger` | Add non-Rust file allowlist ledgers without enforcement. | TOML parse check and `cargo check -p xtask --locked` |
| 11 | `policy/check-file-policy` | Add inventory, proposal, and advisory/blocking file-policy checker modes. | xtask check, file-policy tests, inventory, propose, advisory check |
| 12 | `policy/file-companion-ledgers` | Add generated, executable, dependency, workflow, process, and network ledgers. | Advisory companion checks and policy report |
| 13 | `ci/policy-gate-wiring` | Wire file and lint policy into gate receipts. | File-policy gate receipt and receipt validation |
| 14 | `ci/ripr-and-mutation-routing` | Tune advisory ripr exposure routing and keep mutation off normal PRs. | CI plan dry-run if available and diff check |
| 15 | `refactor/rust-1.95-ast-cleanups` | Use Rust 1.95 APIs where they reduce AST/LSP review load. | Targeted parser, semantic, core, LSP tests; targeted clippy; no-panic check; diff check |
| 16 | `policy/clippy-default-surface-cleanup` | Burn down strict lint debt in default parser/semantic/LSP surface. | Lint policy, clippy exceptions, workspace clippy |
| 17 | `policy/no-panic-first-burndown` | Burn down one narrow no-panic owner lane. | Touched-crate tests, no-panic check, baseline refresh that only drops disappeared entries |
| 18 | `ci/learned-lem-estimates` | Calibrate PR Plan estimates from CI actuals. | Plan output shows static vs learned source with fallback and no branch-protection change |
| 19 | `release/0.14.0-prep-rust-1.95` | Prepare the 0.14.0 minor release. | Workspace check, lint policy, file policy, no-panic, PR-fast receipt, diff check |
| 20 | `release/0.14.0-dry-run` | Prove package and release readiness before tagging. | Workspace package dry-run and policy/gate reports |

## Bot, CI, and self-review rules

For every PR, inspect status checks and review state, then watch checks. If CI fails,
identify the first real failing command, reproduce it locally when practical, fix only
that failure, rerun the matching local gate, push, and re-check bot comments. Treat bot
comments as defects when real, stale when current HEAD proves them obsolete, and follow-up
work when they are correct but out of scope.

Before marking a rollout PR ready, record a self-review covering scope/title match,
expected files, absence of unrelated cleanup, intentional policy changes, no added Clippy
test carveouts, no bare `#[allow(clippy::...)]`, scoped no-panic baseline handling,
narrow non-Rust allowlist changes, CI/ripr/LEM behavior, local validation, CI status,
bot comments, and follow-ups.
