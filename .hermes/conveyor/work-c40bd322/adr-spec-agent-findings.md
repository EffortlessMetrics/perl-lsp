# ADR/Spec Findings — work-c40bd322

## What This ADR Decides
This ADR establishes the architectural approach for extending perl-lsp's production-code quality standards (banned `panic!`, `unwrap`, `todo!`) to test code via baseline infrastructure, and scopes the mechanically actionable panic burn-down work. The core decision is **baseline-first ordering** — establish the baseline with current counts BEFORE any fixes, so "done" is measurable.

## Key Decision
1. **Baseline-first**: `ci/panic_test_baseline.txt` is created with the current (pre-remediation) count, making "done" = baseline reached
2. **Scope limited to verified items**: Only crates that actually exist in the workspace and have verified `panic!` counts are included
3. **`#[allow(clippy::panic)]` propagation**: Every crate adding `perl-tdd-support` dev-dependency must also opt out of `clippy::panic` in its test modules — this is documented as a first-class constraint, not a footnote
4. **Excluded categories**: `unreachable!()`, prints, no-assertion tests, hardcoded paths require scout triage before builder

## Alternatives Considered
- Post-remediation baseline (rejected — "done" is ambiguous)
- Batch all 7 categories (rejected — unreviewable, judgment-heavy)
- Don't document `#[allow(clippy::panic)]` (rejected — compile failures during implementation)
- Include `unreachable!()` (rejected — paired with `must(Err)` for type-checker exhaustivity)

## Consequences
- **Benefits**: Measurable "done", zero new external deps, incremental gate enforcement, better test diagnostics via `assert_matches!`
- **Tradeoffs**: `panic!` count is ~132 (not 180), only 2 verified crates need `perl-tdd-support` dev-dep (not 5)
- **Risks**: Stale crate list, `#[allow(clippy::panic)]` propagation gap, production gate already red

## Acceptance Criteria
1. `ci/panic_test_baseline.txt` exists with current count
2. `ci/todo_test_baseline.txt` exists (0 unlinked TODOs in test code)
3. `perl-dead-code` has `perl-tdd-support` dev-dep + `#![allow(clippy::panic)]`
4. `perl-lsp-feature-policy` has `perl-tdd-support` dev-dep + `#![allow(clippy::panic)]`
5. `panic!` burn-down in top 4 crates: `perl-parser-core`, `perl-dap`, `perl-builtins`, `tree-sitter-perl-rs`
6. `cargo build --all-targets` compiles
7. `cargo test` passes for affected crates
8. Production baselines unchanged
