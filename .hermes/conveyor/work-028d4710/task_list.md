# Task List — work-028d4710

## Implementation Tasks

- [ ] Add `PerlLanguage` struct, `impl` block, `language()` function, and `LANGUAGE` constant to `crates/tree-sitter-perl-rs/src/lib.rs`
- [ ] Add three BDD tests to `crates/tree-sitter-perl-rs/tests/behavior_spec_tests.rs`:
  - `when_language_is_called_then_descriptor_has_nonzero_kind_count`
  - `when_language_reports_kind_names_then_program_is_present`
  - `when_checking_named_kind_then_program_is_named_and_unknown_is_not`
- [ ] Run `cargo test -p tree-sitter-perl-rs -- when_language --exact` to verify new tests pass
- [ ] Run `cargo xtask fmt` to format code
- [ ] Run `cargo clippy -p tree-sitter-perl-rs --tests` to verify clean
- [ ] Run `cargo test -p tree-sitter-perl-rs` to confirm all existing tests pass
- [ ] Verify total test count meets acceptance criteria (35 existing + 3 new = 38)