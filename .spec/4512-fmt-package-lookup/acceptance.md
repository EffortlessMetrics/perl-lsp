# Acceptance Criteria: #4512

- [ ] `cargo xtask resolve-package-name crates/perl-lsp` outputs `perl-lsp-rs` (not `perl-lsp`)
- [ ] Pre-push hook on a modification to `crates/perl-lsp/src/foo.rs` invokes `cargo fmt -p perl-lsp-rs` (not `-p perl-lsp`)
- [ ] Hook succeeds when formatting is clean (no --no-verify needed for routine perl-lsp-rs edits)
- [ ] Regression test `resolve_uses_cargo_toml_name_not_dir_basename` passes: synthetic workspace where dir="my-dir", package="my-package" returns "my-package"
- [ ] Regression test `resolve_when_dir_and_name_match` passes: normal case where dir and package name match
- [ ] Regression test `resolve_returns_error_for_unknown_dir` passes: unknown dir returns Err
- [ ] All xtask tests pass: `cargo test -p xtask`
- [ ] No clippy warnings: `cargo clippy -p xtask -- -D warnings`
- [ ] Formatted: `cargo xtask fmt`
- [ ] `hooks/pre-push` (canonical) and `.git/hooks/pre-push` remain in sync (identical after change)
- [ ] `xtask/src/tasks/fmt.rs` is NOT changed (it was already correct)
