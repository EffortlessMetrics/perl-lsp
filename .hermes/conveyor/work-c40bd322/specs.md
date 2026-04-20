# Specification: Test Code Quality Baseline and Panic Burn-Down

## Feature/Behavior Description

Extend perl-lsp's production-code quality standards (CLAUDE.md §"Coding Standards") to test code via `perl-ci-hygiene` baseline infrastructure, and perform a mechanically verified burn-down of `panic!` in match-arm catches using `assert_matches!`.

### Baseline Establishment

- `ci/panic_test_baseline.txt` — records the current count of `panic!` calls in test code **before** any remediation (enables "done" = baseline reached)
- `ci/todo_test_baseline.txt` — records the current count of unlinked TODO/FIXME in test code
- These files are created by a **fresh scan** using `cargo run -p perl-ci-hygiene` with the same methodology as issue #3237

### Verified Panic Burn-Down

Replace `panic!` in match-arm catches with `std::assert_matches!` for all verified crates. Only match-arm catches outside `#[should_panic]` functions are replaced — `panic!` inside `should_panic` tests is the correct idiom.

### Verified Dependency Additions

Add `perl-tdd-support` as a dev-dependency to crates that:
1. Exist in the workspace (verified)
2. Have `panic!` findings in test code
3. Lack the dev-dependency

Each such crate must include `#![allow(clippy::panic)]` on its test module(s) to avoid compile failures from the workspace's `panic = "deny"` lint when calling `must()` helpers.

## Acceptance Criteria

1. **`ci/panic_test_baseline.txt` exists** — contains the exact count of `panic!` in test code as of this PR, established via a fresh `perl-ci-hygiene` scan
2. **`ci/todo_test_baseline.txt` exists** — contains 0 (no unlinked TODOs in test code; the only unlinked TODO found is in production code, outside scope)
3. **`perl-dead-code` crate** has `perl-tdd-support` as a dev-dependency and `#![allow(clippy::panic)]` on its test module
4. **`perl-lsp-feature-policy` crate** has `perl-tdd-support` as a dev-dependency and `#![allow(clippy::panic)]` on its test module
5. **`panic!` burn-down** — all verified `panic!` in match-arm catches (outside `should_panic` functions) are replaced with `assert_matches!` in the 4 highest-count crates: `perl-parser-core`, `perl-dap`, `perl-builtins`, `tree-sitter-perl-rs`
6. **`cargo build --all-targets`** compiles without errors
7. **`cargo test`** passes for all affected crates
8. **Production baselines unchanged** — `ci/panic_prod_baseline.txt` and `ci/unwrap_prod_baseline.txt` remain at their original values (no regression)

## Non-Goals

- Fixing `unreachable!()` findings (paired with `must(Err)` for type-checker exhaustivity; requires separate refactor)
- Triage or remediation of `println!`/`eprintln!` calls (spin-off #3263)
- Triage or remediation of no-assertion test functions (spin-off #3259)
- Triage or remediation of hardcoded absolute paths (spin-off #3260)
- Fixing `unreachable!()` in `perl-parser/src/heredoc_anti_patterns.rs` (production code; separate work)
- Adding `perl-tdd-support` to crates that don't exist in the workspace

## Dependencies

- `perl-tdd-support` crate (already exists, provides `must`, `must_some`, `must_err`)
- `std::assert_matches!` (Rust 1.73+, workspace MSRV is 1.92)
- Existing `perl-ci-hygiene` tooling for baseline scanning
- No new external dependencies
