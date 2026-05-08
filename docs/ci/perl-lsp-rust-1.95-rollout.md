# Rust 1.95 / 0.14.0 rollout map

This document is the control map for moving `perl-lsp` from the current Rust
1.93 line to Rust 1.95.0 and preparing the next minor release, 0.14.0. It is
intentionally documentation-only: no MSRV, toolchain, workflow, lint, baseline,
or version changes happen in this PR.

Truth sources for rollout facts are the repository files, not this document:

- Workspace edition, MSRV, package version, and active workspace lints:
  [`Cargo.toml`](../../Cargo.toml).
- Pinned toolchain: [`rust-toolchain.toml`](../../rust-toolchain.toml).
- Clippy runtime configuration: [`clippy.toml`](../../clippy.toml).
- Governed lint ledger and planned Rust-version flips:
  [`policy/clippy-lints.toml`](../../policy/clippy-lints.toml).
- CI lane, risk-pack, LEM, and actuals policy:
  [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml),
  [`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml), and
  [`policy/ci-budget.toml`](../../policy/ci-budget.toml).

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

## Why Rust 1.95 matters for `perl-lsp`

| Rust 1.95 item | Repo-specific value |
|---|---|
| `if let` guards | Parser/AST walkers, semantic extraction, import visibility, refactor preconditions, LSP provider routing. |
| `Vec::push_mut` / `insert_mut` | Diagnostic/report builders, semantic facts, scorecards, CI receipts, LSP response builders. |
| Atomic `update` / `try_update` | Session/cache counters, memory-pressure counters, cancellation or stream-state metrics. |
| `cfg_select!` | Windows/Unix path behavior, subprocess handling, VS Code install surfaces, native/virtual URI handling. |
| `cold_path` | Parser recovery, malformed URI/path handling, failed subprocess/perlcritic paths, policy failure reporting. |
| Clippy 1.95 | `manual_checked_ops`, `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, and future `disallowed_fields`. |

The AST-heavy parts of this repository are the main beneficiary: `if let` guards
let semantic walkers keep the branch shape and the semantic precondition in the
same expression, which reduces review load and helps keep parser/provider paths
from drifting into panic-prone precondition checks.

## Operating rule

The first implementation PR after this documentation PR is a Rust 1.95
compatibility spike. No MSRV bump, lint activation, no-panic baseline reset,
release bump, or API cleanup happens in the same PR.

## Queue control

The Rust 1.95 rollout is separate from unrelated product and test PRs. If draft
or in-flight work such as UX diagnostic lifecycle fixes or native critic rules is
open, keep it separate: start rollout PRs from a clean `master` snapshot and
rebase after those PRs merge rather than folding Rust 1.95 work into them.

## Policy surfaces to watch

### Clippy policy

- `Cargo.toml` currently carries the active hard bans for
  `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and
  `dbg_macro`.
- `clippy.toml` still has `allow-unwrap-in-tests = true` while
  `policy/clippy-lints.toml` says tests should be panic-free. Removing that
  carveout is a later policy PR, not part of the compatibility spike or MSRV
  bump.
- `collapsible_if = "allow"` is current debt. It should move through a debt
  ledger and behavior-preserving cleanup rather than being flipped while
  changing the toolchain.
- Rust 1.93 rustc lints and Rust 1.94/1.95 Clippy lints are already tracked in
  `policy/clippy-lints.toml`; they should be activated only in their dedicated
  lint-floor and ratchet PRs.

### No-panic policy

The target posture is exact, counted, no-new-debt policy:

1. Match panic-family findings by path, family, selector kind, selector callee,
   snippet, and count.
2. Consume exact allowlist counts first.
3. Consume baseline counts second, except in blocking mode.
4. Report anything left as new debt.

Do not reset or add a no-panic baseline until exact counted identity exists.

### Non-Rust and file policy

Rust and `xtask` remain the default implementation surfaces. Non-Rust files are
legitimate for Perl corpus fixtures, tree-sitter/native parser bindings, VS Code
extension code, workflows, CI scripts, generated status artifacts, and release
metadata, but the target is receipt-backed allowlisting rather than anonymous
expansion.

### ripr

`ripr` is advisory static oracle-gap detection. It should remain non-blocking for
normal PRs during this rollout. The target is better routing: skip docs-only and
test-fixture-only changes unless a label forces analysis, keep artifacts
consistent, and reserve mutation testing for targeted, nightly, or release lanes.

### CI economics

LEM budgets, CI risk packs, CI lane policy, PR smoke, merge-gate shards, UX tests,
memory smoke, Windows guardrails, CI actuals, and advisory `ripr` already exist.
This rollout should tune that control plane; it should not replace it or add a
new parallel process. Learned estimates should be actuals-backed and must not
hard-enforce below the existing 125 LEM ceiling before calibration.

## PR ladder and acceptance gates

| Step | Branch | Objective | Acceptance gate |
|---:|---|---|---|
| 1 | `docs/rust-1.95-rollout` | Map the rollout, docs only. | `cargo check -p xtask --locked`; `cargo xtask check-lint-policy`; `git diff --check` |
| 2 | `probe/rust-1.95-compat` | Run current repo under Rust 1.95.0 before declaring MSRV. | fmt, workspace checks, clippy, tests, lint policy, pr-fast receipt, diff check |
| 3 | `chore/msrv-rust-1.95` | Raise MSRV/toolchain/config/workflow pins to Rust 1.95.0; no release bump. | Rust 1.95 override, fmt, workspace check, lint policy, pr-fast receipt, diff check |
| 4 | `policy/rust-1.95-lints` | Activate the Rust compiler lint floor. | workspace check, lint policy, pr-fast receipt |
| 5 | `policy/clippy-rust-1.95-ratchets` | Measure and activate clean or cheap Rust 1.94/1.95 Clippy ratchets. | lint policy and workspace clippy without global `-D warnings` until debt is receipted |
| 6 | `policy/no-test-clippy-carveouts` | Remove Clippy test unwrap carveout and add fallible test helper path. | targeted checks for the touched helper crate plus lint policy |
| 7 | `policy/no-panic-exact-identity` | Harden no-panic matching by exact counted identity. | `cargo test -p xtask no_panic --locked`; `cargo xtask check-no-panic-family` |
| 8 | `policy/no-panic-baseline` | Add generated no-panic baseline and no-new-debt mode. | baseline reset, no-panic family check, xtask no-panic tests, diff check |
| 9 | `policy/no-panic-diagnostics` | Improve no-panic diagnostics. | xtask no-panic tests, no-panic family check, report existence |
| 10 | `policy/non-rust-file-ledger` | Add non-Rust file allowlist ledger; no enforcement yet. | TOML parse check and `cargo check -p xtask --locked` |
| 11 | `policy/check-file-policy` | Add inventory, proposal, and file-policy checker commands. | xtask check, file-policy tests, inventory, propose, advisory check |
| 12 | `policy/file-companion-ledgers` | Add generated, executable, dependency, workflow, process, and network ledgers. | advisory companion policy checks and `policy-report` |
| 13 | `ci/policy-gate-wiring` | Wire policy checks into gate receipts. | file-policy gate receipt and receipt validation |
| 14 | `ci/ripr-and-mutation-routing` | Tune ripr routing and keep mutation off normal PRs. | CI plan dry run where available and diff check |
| 15 | `refactor/rust-1.95-ast-cleanups` | Apply targeted Rust 1.95 API cleanup in AST/LSP paths. | targeted parser, semantic, LSP tests; targeted clippy; no-panic family; diff check |
| 16 | `policy/clippy-default-surface-cleanup` | Clean strict lint debt in parser, semantic, and LSP default surface. | lint policy, clippy exceptions, workspace clippy |
| 17 | `policy/no-panic-first-burndown` | Burn down one narrow no-panic owner lane. | touched-crate tests, no-panic family check, baseline refresh that only drops disappeared entries |
| 18 | `ci/learned-lem-estimates` | Use CI actuals to calibrate PR Plan LEM estimates. | plan JSON shows `estimate_source`, summaries compare static vs learned, static fallback remains |
| 19 | `release/0.14.0-prep-rust-1.95` | Prepare 0.14.0 release surfaces. | workspace check, lint/file/no-panic policy checks, pr-fast receipt, diff check |
| 20 | `release/0.14.0-dry-run` | Prove package/publish readiness before tagging. | package dry run, pr-fast receipt, policy checks, release readiness docs |

## Bot, CI, and self-review loop

For each PR, inspect the current check and review state before claiming green:

```bash
gh pr view <PR> --json statusCheckRollup,reviewDecision,mergeStateStatus
gh pr checks <PR> --watch
```

If CI fails:

```bash
gh run view <run-id> --log-failed
```

Then identify the first real failing command, reproduce locally if possible, fix
only that failure, rerun the matching local gate, push, and check bot comments
again. Bot comments should be treated as follows: fix real defects, answer false
positives with evidence, fix cheap in-scope style comments, defer out-of-scope
requests with a follow-up, and mark stale comments only after verifying current
HEAD.

Every rollout PR should self-review before it is marked ready:

```markdown
## Self-review

- Scope matches PR title:
- Files touched are expected:
- No unrelated cleanup:
- Policy changes are intentional:
- No Clippy test carveouts added:
- No bare `#[allow(clippy::...)]` added:
- No-panic baseline handling is scoped:
- Non-Rust allowlist changes are narrow:
- CI/ripr/LEM behavior matches docs:
- Local validation:
- CI status:
- Bot comments addressed:
- Follow-ups:
```

If any item is not true, do not merge.
