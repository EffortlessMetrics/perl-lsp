# Rust 1.95 / 0.14.0 rollout map

This document is the control map for moving `perl-lsp` from the current Rust
1.93 / toolchain 1.93.1 line to Rust 1.95.0 and preparing the next minor release,
`0.14.0`. It is intentionally documentation-only: no MSRV, workflow, policy,
code, or release-version changes happen in this PR.

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

## Why Rust 1.95 matters here

Rust 1.95 is not being adopted only because it is newer. The useful changes map
directly to `perl-lsp`'s parser, semantic, LSP, DAP, policy, and CI-control
surfaces.

| Rust 1.95 item | Repo-specific value |
|---|---|
| `if let` guards | Parser/AST walkers, semantic extraction, import visibility, refactor preconditions, LSP provider routing. |
| `Vec::push_mut` / `insert_mut` | Diagnostic/report builders, semantic facts, scorecards, CI receipts, LSP response builders. |
| Atomic `update` / `try_update` | Session/cache counters, memory-pressure counters, cancellation or stream-state metrics. |
| `cfg_select!` | Windows/Unix path behavior, subprocess handling, VS Code install surfaces, native/virtual URI handling. |
| `cold_path` | Parser recovery, malformed URI/path handling, failed subprocess/perlcritic paths, policy failure reporting. |
| Clippy 1.95 | `manual_checked_ops`, `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, and future `disallowed_fields`. |

The biggest implementation value is `if let` guards in AST-heavy semantic walkers:
they keep branch shape and semantic preconditions together, which reduces review
load and lowers the odds of panic-prone follow-up indexing or unwraps.


## Queue control

Two open draft PRs are not part of the Rust 1.95 rollout and must stay separate:

- `#8306 fix(ux-tests): eliminate post-fix race in scenario_19 diagnostics lifecycle`
- `#8301 feat(critic): add parameter_shadows_global native critic rule`

Do not fold Rust 1.95 work into either product/test PR. Start rollout work from a
clean `master` baseline and expect to rebase after unrelated queue items merge.

## Explicit operating rule

The first implementation PR after this documentation PR is a Rust 1.95
compatibility spike. No MSRV bump, lint activation, no-panic baseline reset,
release bump, or API cleanup happens in the same PR.

## Current policy notes

### Clippy

- Root `Cargo.toml` already hard-bans the core panic-family Clippy lints:
  `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and `dbg_macro`.
- Root `Cargo.toml` still carries `collapsible_if = "allow"`; this remains debt
  until a cleanup or exception-ledger PR addresses it deliberately.
- `policy/clippy-lints.toml` records the broader active/tracked/planned lint
  posture and still uses `msrv = "1.93"`.
- `clippy.toml` still has `allow-unwrap-in-tests = true`, while the target
  posture is no test carveouts. The rollout keeps this mismatch visible and
  resolves it in its own policy PR with helper functions, not as part of the
  MSRV bump.

### No-panic

The target no-panic posture is exact, counted, and no-new-debt:

1. Findings are keyed by path, family, selector kind, selector callee, and snippet.
2. Allowlist entries consume exact counted slots.
3. Baseline entries consume remaining counted slots only outside blocking mode.
4. New unmatched findings fail the gate.

The baseline reset is deliberately deferred until exact matching exists.

### Non-Rust and file policy

Rust and `xtask` remain the default implementation surface. Non-Rust files are
allowed when the reason, owner, surface, classification, and coverage are recorded
in policy. Legitimate non-Rust surfaces include Perl fixtures/corpus,
tree-sitter/native C bindings, VS Code extension assets, GitHub workflows, CI
scripts, generated status artifacts, and release metadata.

The file-policy rollout is ledger-first, checker-second, then CI wiring. That
prevents a broad allowlist from silently becoming the policy.

### ripr

`ripr` is advisory static oracle-gap detection. It complements normal gates and
mutation testing but does not replace either. During this rollout it remains
advisory, skips low-risk docs/test-fixture-only changes unless forced by label,
and routes higher-risk results toward targeted follow-up or mutation lanes.

### CI economics

The existing control plane already has LEM budgeting, risk packs, lane policy, PR
smoke, merge-gate shards, UX tests, memory smoke, Windows guardrails, CI actuals,
and advisory `ripr`. This rollout tunes those rails instead of inventing a new
process. Learned LEM estimates should use recent actuals only after a calibration
window, and no sub-125-LEM hard enforcement should be added before that work lands.

## PR ladder

| Step | Branch | Title | Scope boundary | Acceptance gate |
|---:|---|---|---|---|
| 1 | `docs/rust-1.95-rollout` | `docs(policy): map Rust 1.95 and 0.14.0 quality rollout` | Documentation-only rollout map. | `cargo check -p xtask --locked`; `cargo xtask check-lint-policy`; `git diff --check` |
| 2 | `probe/rust-1.95-compat` | `chore(msrv): probe Rust 1.95 compatibility` | Run current repo under Rust 1.95 before declaring it. Prefer audit note only. | fmt, workspace checks, clippy, tests, lint policy, pr-fast receipt, diff check |
| 3 | `chore/msrv-rust-1.95` | `chore(msrv): raise workspace toolchain to Rust 1.95` | MSRV/toolchain/config/workflow references only; no release bump. | Rust 1.95 override, fmt, workspace check, lint policy, pr-fast receipt, diff check |
| 4 | `policy/rust-1.95-lints` | `policy(rust): enable Rust 1.95 compiler lint floor` | Activate rustc lint floor and update ledger. | workspace check, lint policy, pr-fast receipt |
| 5 | `policy/clippy-rust-1.95-ratchets` | `policy(clippy): activate Rust 1.95 lint ratchets` | Measure first; activate clean or cheaply fixable Clippy 1.94/1.95 lints. | lint policy and workspace clippy without global `-D warnings` until debt is receipted |
| 6 | `policy/no-test-clippy-carveouts` | `policy(clippy): remove test unwrap carveout` | Remove test unwrap carveout and add fallible helper path; do not migrate the whole suite. | targeted helper tests and lint-policy check |
| 7 | `policy/no-panic-exact-identity` | `policy(panic): harden no-panic allowlist identity` | Exact counted no-panic matching before any baseline reset. | `cargo test -p xtask no_panic --locked`; `cargo xtask check-no-panic-family` |
| 8 | `policy/no-panic-baseline` | `policy(panic): add no-panic baseline and no-new-debt gate` | Generate baseline once from current master after exact identity exists. | baseline reset, no-panic family check, no-panic tests, diff check |
| 9 | `policy/no-panic-diagnostics` | `policy(panic): improve no-panic report diagnostics` | More actionable missing/stale/delta/blocking diagnostics. | no-panic tests, no-panic family check, report existence check |
| 10 | `policy/non-rust-file-ledger` | `policy(files): add non-Rust file allowlist` | Ledger only; no enforcement. | TOML parse and `cargo check -p xtask --locked` |
| 11 | `policy/check-file-policy` | `policy(files): enforce non-Rust allowlist` | Inventory/proposal/checker commands and advisory/blocking modes. | xtask check, file-policy tests, inventory, propose, advisory check |
| 12 | `policy/file-companion-ledgers` | `policy(files): add companion allowlists for risky surfaces` | Generated, executable, dependency, workflow, process, and network ledgers. | companion checks in advisory mode and policy report |
| 13 | `ci/policy-gate-wiring` | `ci(policy): wire file and lint policies into gate receipts` | Add policy gate receipt to existing conveyor. | file-policy gate receipt and receipt validation |
| 14 | `ci/ripr-and-mutation-routing` | `ci(ripr): tune advisory exposure routing` | Tune `ripr` routing; keep mutation off normal PRs. | CI plan dry-run if available and diff check |
| 15 | `refactor/rust-1.95-ast-cleanups` | `refactor: use Rust 1.95 APIs in AST and provider paths` | Targeted API cleanup where Rust 1.95 reduces complexity. | targeted parser/semantic/LSP tests, clippy, no-panic check, diff check |
| 16 | `policy/clippy-default-surface-cleanup` | `policy(clippy): clean strict lint debt in default surface` | Fix or receipt strict-lint debt in parser, semantic, and LSP defaults. | lint policy, clippy exceptions, workspace clippy |
| 17 | `policy/no-panic-first-burndown` | `policy(panic): burn down first no-panic owner lane` | One narrow owner lane only. | touched-crate tests, no-panic family check, baseline refresh that only drops disappeared entries |
| 18 | `ci/learned-lem-estimates` | `ci: use actuals to calibrate PR Plan LEM estimates` | Actuals-backed estimates with static fallback; no branch-protection change. | plan JSON includes estimate source and summary distinguishes static vs learned |
| 19 | `release/0.14.0-prep-rust-1.95` | `release: prepare 0.14.0 for Rust 1.95` | Version and release-facing prep for the minor release. | workspace check, lint/file/no-panic checks, pr-fast receipt, diff check |
| 20 | `release/0.14.0-dry-run` | `release: validate 0.14.0 publish readiness` | Package/publish readiness proof and release docs. | package dry-run, pr-fast receipt, policy checks, policy report |

## Bot, CI, and self-review rules

For every rollout PR:

1. Start from a clean `master` baseline when possible; do not stack unless a step
   explicitly depends on the previous branch.
2. Keep one PR per objective and do not fold unrelated cleanup into a failing CI fix.
3. If CI fails, identify the first real failing command, reproduce locally when
   possible, fix only that failure, rerun the matching local gate, then re-check CI.
4. Treat bot comments as defects when real, stale when superseded by current HEAD,
   and follow-up material when correct but out of scope.
5. Report skipped lanes as skipped by policy, not passed.

Before marking a PR ready, the author must self-review:

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

If any item is not true, the PR should not merge.
