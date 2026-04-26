# Task List — work-94d78475

## Implementation Tasks

1. [ ] **Modify builtins.rs** — Add `utf8::encode`, `utf8::decode`, `utf8::downgrade` entries to `get_builtin_documentation()` in `crates/perl-semantic-analyzer/src/analysis/semantic/builtins.rs`
   - Each entry must have only `signature` and `description` fields (NO `version_required`)
   - Follow existing patterns for other builtin entries

2. [ ] **Add unit tests** — Add tests in `crates/perl-semantic-analyzer/tests/builtin_context_docs_tests.rs` for the new entries
   - `test_utf8_encode_builtin_doc()`
   - `test_utf8_decode_builtin_doc()`
   - `test_utf8_downgrade_builtin_doc()`

3. [ ] **Run tests** — Verify `cargo test -p perl-semantic-analyzer` passes

4. [ ] **Run clippy** — Verify `cargo clippy -p perl-semantic-analyzer` passes with no warnings

## Verification Tasks

5. [ ] **Hover verification** — Verify that hovering on `utf8::encode($var)` in a Perl file shows the documentation

## Dependencies

- Implementation depends on: nothing (pure documentation addition)
- Tests depend on: builtins.rs modification complete