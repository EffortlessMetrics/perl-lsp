# Specs — work-8eac7725

## Feature/Behavior Description

Replace `panic!()` calls in test match-arm catch handlers (`_ => panic!` and `other => panic!`) with `assert_matches!()` macro from `std::assert_matches`. This is a test code quality refactor that uses Rust 1.75+'s canonical idiom for enum variant assertions.

## Feature Behavior

### What Changes

1. **perl-parser-core** (~87 occurrences across 6 files):
   - `src/engine/parser/qualified_variable_subscript_tests.rs`: `_ => panic!` patterns
   - `src/engine/parser/chained_deref_method_tests.rs`: `_ => panic!` patterns
   - `src/engine/parser/coderef_invocation_tests.rs`: `_ => panic!` patterns
   - `src/engine/parser/tests.rs`: `other => panic!` patterns
   - `src/engine/parser/from_tokens_tests.rs`: `_ => panic!` patterns
   - `src/engine/parser/declaration_in_args_tests.rs`: `_ => panic!` patterns

2. **perl-dap** (~40 occurrences across 5 files):
   - `tests/dap_protocol_message_tests.rs`: `other => panic!` patterns
   - `tests/dap_non_regression_tests.rs`: `other => panic!` patterns
   - `tests/dap_protocol_compliance_tests.rs`: `other => panic!` patterns
   - `tests/dap_step_through_tests.rs`: `other => panic!` patterns
   - `tests/dap_adapter_tests.rs`: `_ => panic!` patterns

3. **perl-lsp** (~3 occurrences):
   - `tests/editor_intelligence_scorecard.rs`: `Err(e) => panic!` patterns (setup failures — preserved)

### What Doesn't Change

1. **perl-test-must helpers**: `must()`, `must_some()`, `must_err()` in `crates/perl-test-must/src/lib.rs` — intentionally designed to panic, excluded
2. **Setup failure panics**: `Err(e) => panic!("Failed to load fixtures...")` patterns — semantically not assertion failures
3. **Non-match-arm panics**: Standalone `panic!` calls in `if` conditions, for-loops, etc.
4. **`unreachable!()`**: Issue explicitly excludes these (267 occurrences)

## Acceptance Criteria

### Must Have

1. **Correct pattern replacement**: All `_ => panic!` and `other => panic!` patterns in perl-parser-core and perl-dap test files are replaced with `assert_matches!` macro

2. **Tests pass**: `cargo test -p perl-parser-core --tests && cargo test -p perl-dap --tests` succeeds with no regressions

3. **Formatting and linting clean**: `cargo fmt --all && cargo clippy --workspace --tests` passes with no new warnings introduced by the changes

4. **Scope correctness**:
   - perl-test-must helpers unchanged
   - Setup failure `Err/Ok => panic!` patterns preserved
   - `unreachable!()` not touched

5. **Baseline file created**: `ci/panic_test_baseline.txt` exists with pre-replacement count (e.g., `126`), not `0`

### Should Have

6. **`#[allow(clippy::panic)]` cleanup**: Only remove if all panics in that scope have been replaced (per-file analysis after refactoring)

7. **PR titles reference issue**: Each PR title includes `#3258` as required by `validate-title` check

## Non-Goals

- Not changing `unreachable!()` patterns (issue explicitly excludes)
- Not changing standalone `panic!` calls that are not in match arms
- Not refactoring perl-test-must helper functions (they are test infrastructure, not assertion patterns)
- Not preserving all diagnostic panic messages — structured `assert_matches!` failure is sufficient

## Dependencies

- Rust 1.75+ (for `assert_matches!` stability)
- No new dependencies introduced
- CI gates: `cargo check --workspace --tests`, `cargo test --workspace --lib`

## Verification Commands

```bash
# Pre-flight: count actual match-arm panics
grep -rE '^\s*(_|other)\s*=>\s*panic!' crates/perl-parser-core/ crates/perl-dap/ --include='*.rs' | wc -l

# Verify workspace builds
cargo check --workspace --tests

# Run tests for affected crates
cargo test -p perl-parser-core --tests
cargo test -p perl-dap --tests
cargo test -p perl-lsp --tests

# Format and clippy
cargo fmt --all && cargo clippy --workspace --tests
```