# Rust 1.95 / 0.14.0 rollout map

This document is the control map for moving `perl-lsp` from the Rust 1.93
line to Rust 1.95.0 and preparing the next minor release line, `0.14.0`.
It is intentionally documentation-only: no MSRV, toolchain, lint, workflow,
release-version, baseline, or API changes are part of this PR.

## Current and target state

Truth sources for the current state are `Cargo.toml`, `rust-toolchain.toml`,
`clippy.toml`, and `policy/clippy-lints.toml`.

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

## Rust 1.95 value for `perl-lsp`

| Rust 1.95 item | Repo-specific value |
|---|---|
| `if let` guards | Parser/AST walkers, semantic extraction, import visibility, refactor preconditions, LSP provider routing. |
| `Vec::push_mut` / `insert_mut` | Diagnostic/report builders, semantic facts, scorecards, CI receipts, LSP response builders. |
| Atomic `update` / `try_update` | Session/cache counters, memory-pressure counters, cancellation or stream-state metrics. |
| `cfg_select!` | Windows/Unix path behavior, subprocess handling, VS Code install surfaces, native/virtual URI handling. |
| `cold_path` | Parser recovery, malformed URI/path handling, failed subprocess/perlcritic paths, policy failure reporting. |
| Clippy 1.95 | `manual_checked_ops`, `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, and future `disallowed_fields`. |

`if let` guards matter materially for AST-heavy semantic walkers because they
keep the branch shape and semantic precondition together. That should reduce
review load in parser, semantic-analyzer, and provider paths once the MSRV has
actually moved.

## Operating rule

The first implementation PR after this documentation PR is a Rust 1.95
compatibility spike. No MSRV bump, lint activation, no-panic baseline reset,
release bump, or API cleanup happens in the same PR.

## Queue control

The Rust 1.95 rollout must not be folded into unrelated product or test PRs.
Open draft work such as UX race fixes or native critic rules remains separate.
Each rollout PR starts from clean `master`/`origin/master` when available and
contains one objective.

## Policy status to carry forward

### Clippy

- Active root hard bans already cover `unwrap_used`, `expect_used`, `panic`,
  `todo`, `unimplemented`, and `dbg_macro`.
- `clippy::collapsible_if` remains a broad allow and is tracked as debt until
  a dedicated cleanup or exception ledger PR removes the global carveout.
- `policy/clippy-lints.toml` tracks Rust 1.93 rustc lints and Rust 1.94/1.95
  Clippy lints, but activation is intentionally deferred until the relevant
  rollout PRs.
- `clippy.toml` still has `allow-unwrap-in-tests = true` while the policy
  target is strict tests with no test carveouts. Removing that mismatch is a
  dedicated PR after the MSRV bump and ratchet measurement.

### No-panic

The target posture is exact, counted identity for retained panic-family debt:
`path + family + selector_kind + selector_callee + snippet + count`. Allowlist
counts are consumed first, then baseline counts are consumed unless the mode is
blocking, and anything left is reported as new debt. Baseline reset is reserved
for the dedicated baseline PR only.

### Non-Rust and file policy

Rust and `xtask` remain the default implementation surfaces. Non-Rust files are
allowed by receipt rather than anonymously. Legitimate surfaces include Perl
fixtures/corpus data, tree-sitter C/native parser bindings, VS Code extension
code, GitHub workflows, CI scripts, docs/status artifacts, and release metadata.
The target is a blocking allowlist plus companion ledgers for generated files,
executables, dependency surfaces, workflow behavior, process execution, and
network access.

### ripr

`ripr` remains advisory. It is a fast PR-time exposure filter, not a mutation
replacement and not branch-protection blocking. The rollout should tune routing
so ordinary PRs get `ripr` plus normal gates, high-risk changes can trigger
targeted mutation, nightly/release lanes can carry fuller mutation proof, and
docs-only/test-fixture-only changes can be skipped unless labels force it.

### CI economics

LEM budget policy, CI lane policy, risk packs, PR smoke, merge-gate shards, UX
tests, memory smoke, Windows guardrails, CI actuals, and advisory `ripr` already
exist. The rollout should tune this control plane using receipts and actuals;
it should not hard-enforce learned LEM estimates below the 125 LEM ceiling
before enough actuals are calibrated.

## PR ladder and acceptance gates

| PR | Branch | Objective | Acceptance |
|---:|---|---|---|
| 1 | `docs/rust-1.95-rollout` | Document this rollout map only. | `cargo check -p xtask --locked`; `cargo xtask check-lint-policy`; `git diff --check` |
| 2 | `probe/rust-1.95-compat` | Run the current repo under Rust 1.95.0 before changing declarations. | Rust 1.95 fmt/check/clippy/test, lint policy, pr-fast receipt, diff check |
| 3 | `chore/msrv-rust-1.95` | Raise MSRV/toolchain/config/workflow pins to Rust 1.95.0 without release bump. | Rust 1.95 fmt/check, lint policy, pr-fast receipt, diff check |
| 4 | `policy/rust-1.95-lints` | Activate the compiler lint floor. | workspace check, lint policy, pr-fast receipt |
| 5 | `policy/clippy-rust-1.95-ratchets` | Measure and activate clean/cheap Rust 1.94/1.95 Clippy ratchets. | lint policy, workspace clippy |
| 6 | `policy/no-test-clippy-carveouts` | Remove the test unwrap carveout and add fallible helper path. | targeted tests/checks from touched helper crate and lint policy |
| 7 | `policy/no-panic-exact-identity` | Harden no-panic matching to exact counted identity. | `cargo test -p xtask no_panic --locked`; `cargo xtask check-no-panic-family` |
| 8 | `policy/no-panic-baseline` | Add no-new-debt baseline after exact identity exists. | baseline reset, no-panic family check, no-panic tests, diff check |
| 9 | `policy/no-panic-diagnostics` | Improve actionable no-panic reports. | no-panic tests/checks and report existence |
| 10 | `policy/non-rust-file-ledger` | Add non-Rust allowlist/debt ledger without enforcement. | TOML parse, `cargo check -p xtask --locked` |
| 11 | `policy/check-file-policy` | Add inventory, propose, and check-file-policy commands. | xtask check/tests and advisory file-policy commands |
| 12 | `policy/file-companion-ledgers` | Add generated/executable/dependency/workflow/process/network ledgers. | companion checks in advisory mode and policy report |
| 13 | `ci/policy-gate-wiring` | Wire policy checks into gate receipts. | file-policy gate receipt and receipt validation |
| 14 | `ci/ripr-and-mutation-routing` | Tune advisory `ripr` exposure and mutation routing. | CI plan dry run if available, diff check |
| 15 | `refactor/rust-1.95-ast-cleanups` | Use Rust 1.95 APIs in AST/semantic/provider paths where they reduce risk. | targeted parser/semantic/LSP tests, targeted clippy, no-panic family, diff check |
| 16 | `policy/clippy-default-surface-cleanup` | Clean strict lint debt in parser/semantic/LSP default surfaces. | lint policy, clippy exceptions, workspace clippy |
| 17 | `policy/no-panic-first-burndown` | Burn down one narrow no-panic owner lane. | touched-crate tests, no-panic family, baseline refresh that only drops entries |
| 18 | `ci/learned-lem-estimates` | Use CI actuals to calibrate PR Plan LEM estimates. | `ci-plan.json` shows static/learned source and fallback behavior |
| 19 | `release/0.14.0-prep-rust-1.95` | Prepare the `0.14.0` minor release. | workspace check, lint/file/no-panic policies, pr-fast receipt, diff check |
| 20 | `release/0.14.0-dry-run` | Prove package and publish readiness before tagging. | package dry run, pr-fast receipt, policies, policy report |

## Bot, CI, and self-review loop

For every PR, inspect PR status and checks, then investigate the first real
failing command from failed logs. Reproduce locally when possible, fix only the
failing scope, rerun the matching local gate, push, and re-check bot comments.
Treat bot findings as real defects unless current-HEAD evidence shows they are
false positives, stale, or out of scope.

Before marking a rollout PR ready, add a self-review covering:

- scope matches PR title;
- files touched are expected;
- no unrelated cleanup;
- policy changes are intentional;
- no Clippy test carveouts added;
- no bare `#[allow(clippy::...)]` added;
- no-panic baseline handling is scoped;
- non-Rust allowlist changes are narrow;
- CI/ripr/LEM behavior matches docs;
- local validation;
- CI status;
- bot comments addressed;
- follow-ups.

If any item is not true, the PR is not ready.
