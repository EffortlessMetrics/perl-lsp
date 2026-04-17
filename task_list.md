# Task List — work-8eac7725

## PR 1: perl-parser-core (~87 occurrences)

- [ ] Count actual match-arm panics in perl-parser-core (pre-flight verification)
- [ ] Create `ci/panic_test_baseline.txt` with pre-replacement count
- [ ] Refactor `src/engine/parser/qualified_variable_subscript_tests.rs` (~39 occurrences)
- [ ] Refactor `src/engine/parser/chained_deref_method_tests.rs` (~25 occurrences)
- [ ] Refactor `src/engine/parser/coderef_invocation_tests.rs` (~11 occurrences)
- [ ] Refactor `src/engine/parser/tests.rs` (~2 occurrences)
- [ ] Refactor `src/engine/parser/from_tokens_tests.rs` (~1 occurrence)
- [ ] Refactor `src/engine/parser/declaration_in_args_tests.rs` (~1 occurrence)
- [ ] Add `use std::assert_matches::assert_matches;` where needed
- [ ] Run `cargo fmt --all && cargo clippy -p perl-parser-core --tests`
- [ ] Run `cargo test -p perl-parser-core --tests` and verify pass
- [ ] PR with title `test(quality): replace panic! with assert_matches in perl-parser-core (#3258)`

## PR 2: perl-dap + perl-lsp (~43 occurrences)

- [ ] Count actual match-arm panics in perl-dap (pre-flight verification)
- [ ] Refactor `tests/dap_protocol_message_tests.rs` (~14 occurrences)
- [ ] Refactor `tests/dap_non_regression_tests.rs` (~14 occurrences)
- [ ] Refactor `tests/dap_protocol_compliance_tests.rs` (~10 occurrences)
- [ ] Refactor `tests/dap_step_through_tests.rs` (~1 occurrence)
- [ ] Refactor `tests/dap_adapter_tests.rs` (~1 occurrence)
- [ ] perl-lsp `tests/editor_intelligence_scorecard.rs` — evaluate each `Err(e) => panic!` individually
  - [ ] Preserve setup failures as-is (panic is correct when fixtures fail to load)
- [ ] Add `use std::assert_matches::assert_matches;` where needed
- [ ] Run `cargo fmt --all && cargo clippy -p perl-dap --tests`
- [ ] Run `cargo test -p perl-dap --tests` and verify pass
- [ ] PR with title `test(quality): replace panic! with assert_matches in perl-dap and perl-lsp (#3258)`

## Verification Tasks (both PRs)

- [ ] Verify `ci/panic_test_baseline.txt` exists with pre-replacement count
- [ ] Run `cargo fmt --all && cargo clippy --workspace --tests` — no new warnings
- [ ] Run `cargo test --workspace --lib` — all tests pass
- [ ] Remove `#[allow(clippy::panic)]` directives only where all panics in scope replaced