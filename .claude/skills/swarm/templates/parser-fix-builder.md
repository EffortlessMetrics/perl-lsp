# Parser Fix Builder Prompt Template

Use this when spawning a worktree worker to fix a parser bug.

## Template

```
Invoke /coding-standards.

Goal: Fix parser bug — <BUG_DESCRIPTION>.

## Context
- Issue: <ISSUE_NUMBER_OR_LINK>
- Root cause: <ROOT_CAUSE_SUMMARY_FROM_SCOUT>
- Target files: <FILE_SURFACE>
- Construct: <PERL_CONSTRUCT_THAT_FAILS>

## Steps
1. Read the target files and understand the current parsing path
2. Add a failing test in the appropriate `*_tests.rs` file
3. Implement the smallest fix that makes the new test pass
4. Run verification: `cargo fmt --all && cargo clippy -p perl-parser-core --lib && cargo test -p perl-parser-core && cargo test -p perl-parser`
5. If tests were added, run `python3 scripts/update-current-status.py` then `just status-check`
6. Commit: `fix(perl-parser-core): <description>`

## Rules
- Do NOT rebase. Only fix code and verify locally.
- Do NOT fix unrelated parser issues found during investigation
- Do NOT refactor surrounding code
- If the bug is actually in the lexer, stop and report back

## Verification
cargo fmt --all && cargo clippy -p perl-parser-core --lib && cargo test -p perl-parser-core && cargo test -p perl-parser
```
