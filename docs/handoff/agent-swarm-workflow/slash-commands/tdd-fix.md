---
description: TDD fix workflow (failing test, fix, verify)
argument-hint: "<bug description> e.g. 'login fails when email contains +'"
---

# TDD Fix

Fix a bug using test-driven development. Bug: **$ARGUMENTS**

## Launch an agent in a worktree to do this work:

Use the Agent tool with `isolation: "worktree"` and `mode: "auto"` to:

### 1. Find the root cause
- Search the codebase for the module responsible for the described behavior
- Read the relevant source code to understand WHY the bug occurs
- Check for existing tests that should have caught this

### 2. Write failing tests FIRST
- Add tests in the appropriate test file
- Each test should exercise the buggy behavior and assert the correct outcome
- The test MUST fail before your fix (this proves it tests the right thing)

```
# Run tests to confirm they fail:
$TEST_CMD
```

### 3. Implement minimal fix
- Change as little code as possible
- Follow the project's coding standards:
  - No panicking constructs in production code (unwrap, expect, panic, todo, etc.)
  - Use proper error handling (Result, Option, error types)
  - Match existing patterns in surrounding code

### 4. Verify
```bash
$FMT_CMD
$LINT_CMD
$TEST_CMD
```
All must pass cleanly.

### 5. Create PR
- Branch, commit, push, `gh pr create`
- Title: `fix(<scope>): <description>`
- Return PR URL

## Coding standards reminder
- No fatal constructs in production code
- Use `?`, `.ok_or_else()`, pattern matching for errors
- In tests: use `Result<()>` return types or equivalent assertion helpers
- Formatter and linter must be clean
