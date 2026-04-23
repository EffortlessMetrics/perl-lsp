# Acceptance Criteria: #5499

- [ ] Test function `test_completion_scope_distance_ranking()` added to `crates/perl-lsp-rs/tests/lsp_completion_tests.rs`
- [ ] Test creates Perl code with nested scopes and shadowed variable name ($config)
- [ ] Test sends `didOpen` notification to open file with test code
- [ ] Test sends `textDocument/completion` request at position where $c is typed
- [ ] Test extracts completion items from response
- [ ] Test filters items for those matching "config"
- [ ] Test asserts at least one $config completion item exists
- [ ] Test verifies sort_text ordering: immediate scope item should have lower sort_text than parent scope item
- [ ] Test compares sort_text lexicographically: `first_sort < second_sort`
- [ ] Test executes without panic
- [ ] All LSP tests pass: `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2`
- [ ] No clippy warnings on new code: `cargo clippy -p perl-lsp-rs --tests`
- [ ] Code formatted correctly: `cargo xtask fmt`
- [ ] Test positioned after line 1055 (end of test_completion_ranking)
