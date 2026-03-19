# Test Coverage Builder Prompt Template

Use this when spawning a worktree worker to add tests to a crate.

## Template

```
Invoke /coding-standards.

Goal: Add test coverage for <CRATE_NAME> — <FOCUS_AREA>.

## Context
- Issue: <ISSUE_NUMBER_OR_LINK>
- Current coverage gap: <WHAT_IS_UNTESTED>
- Target files: <FILE_SURFACE>

## Steps
1. Read the source files to understand the API surface
2. Identify untested code paths, edge cases, and error conditions
3. Add tests following the naming convention: `test_<what>_<scenario>_<expected>`
4. Use `Result<()>` returns or `perl_tdd_support::must` / `must_some`
5. Run verification: `cargo fmt --all && cargo clippy -p <CRATE> --tests -- -D warnings && cargo test -p <CRATE>`
6. Run `just status-update && just status-check` (required when adding tests)
7. Commit: `test(<CRATE>): <description>`

## Test Standards
- No `unwrap()` or `expect()` in tests — use `Result<()>` or `must`/`must_some`
- Descriptive names: `test_<what>_<scenario>_<expected>`
- For perl-lsp: `RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2`

## Out of Scope
- Do not fix bugs found during testing (file an issue instead)
- Do not refactor production code

## Verification
cargo fmt --all && cargo clippy -p <CRATE> --tests -- -D warnings && cargo test -p <CRATE>
```
