# Review Agent Prompt Template

Use this when spawning a subagent to review a single PR.

## Template

```
Invoke /coding-standards.

Goal: Review PR #<PR_NUMBER> — <PR_TITLE>.

## Context
- Branch: <BRANCH_NAME>
- Crate: <PRIMARY_CRATE>
- Builder receipt: <RECEIPT_SUMMARY>

## Review Checklist
1. CHECKOUT the branch and BUILD it locally — do not just read diffs:
   `git fetch origin && git checkout <BRANCH_NAME>`
   `cargo build -p <PRIMARY_CRATE>`
2. Run the full verification pipeline:
   `cargo fmt --all -- --check && cargo clippy -p <PRIMARY_CRATE> --tests -- -D warnings && cargo test -p <PRIMARY_CRATE>`
3. Check coding standards compliance:
   - No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()`
   - Proper error handling with `?` and `Result`/`Option`
   - `.first()` not `.get(0)`, `or_default()` not `or_insert_with(Vec::new)`
4. Verify new functions are actually called from an entry point — not just defined
5. Check tests exist for new functionality
6. Check PR description is accurate and complete
7. Check scope — no unrelated changes mixed in
8. Check commit message follows conventional format: `type(scope): description`

## Decision
- **Approve**: If all checks pass, report approval to ops coordinator
- **Request changes**: If issues found, list specific fixes needed and report to builder coordinator
- Do NOT make code changes yourself — route fixes back to builder

## Report Format
- PR number and title
- Build result: compiled / failed
- Test result: passed / failed (with count)
- Pass/fail for each checklist item
- Specific issues found (with file:line references)
- Verdict: approve / request-changes
```
