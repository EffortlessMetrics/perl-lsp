# Task List — work-c40bd322

## Phase 1: Verification (fresh scan)
- [ ] Run `cargo run -p perl-ci-hygiene -- check-panics` to get current test panic count
- [ ] Run `cargo run -p perl-ci-hygiene -- check-todos` to confirm 0 unlinked TODOs in test code
- [ ] Create `ci/panic_test_baseline.txt` with the count from above
- [ ] Create `ci/todo_test_baseline.txt` with `0`

## Phase 2: Verified Crate Dependency Additions
- [ ] Add `perl-tdd-support` as dev-dependency to `perl-dead-code`
- [ ] Add `#![allow(clippy::panic)]` to `perl-dead-code` test module(s)
- [ ] Run `cargo check -p perl-dead-code --tests` to verify no lint conflicts
- [ ] Add `perl-tdd-support` as dev-dependency to `perl-lsp-feature-policy`
- [ ] Add `#![allow(clippy::panic)]` to `perl-lsp-feature-policy` test module(s)
- [ ] Run `cargo check -p perl-lsp-feature-policy --tests` to verify no lint conflicts

## Phase 3: Panic Burn-Down (top 4 crates)
- [ ] `perl-parser-core` — replace `panic!` in match-arm catches with `assert_matches!`
- [ ] `perl-dap` — replace `panic!` in match-arm catches with `assert_matches!`
- [ ] `perl-builtins` — replace `panic!` in match-arm catches with `assert_matches!`
- [ ] `tree-sitter-perl-rs` — replace `panic!` in match-arm catches with `assert_matches!`
- [ ] Run `cargo test` for each modified crate
- [ ] Run `cargo clippy --all-targets --all-features` to verify no new lint failures

## Phase 4: Verification
- [ ] `cargo build --all-targets` — all targets compile
- [ ] `cargo test` across all affected crates — all tests pass
- [ ] `perl-ci-hygiene check-panics-prod` — no regression in production baseline
- [ ] `perl-ci-hygiene check-unwraps-prod` — no regression in production baseline

## Excluded (require scout triage first)
- `unreachable!()` findings — paired with `must(Err)` for type-checker exhaustivity
- `println!`/`eprintln!` triage (~2,000 calls)
- No-assertion test functions (~620)
- Hardcoded absolute paths (~133)
