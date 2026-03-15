# Example Agent Prompts

Copy-paste templates for common agent tasks. Replace placeholders with your
project's specifics.

Placeholders used throughout:
- `$PROJECT_ROOT` -- absolute path to your repository root
- `$TEST_CMD` -- your test runner (e.g., `cargo test`, `pytest`, `npm test`, `go test ./...`)
- `$LINT_CMD` -- your linter (e.g., `cargo clippy`, `ruff check .`, `eslint .`, `golangci-lint run`)
- `$FMT_CMD` -- your formatter (e.g., `cargo fmt`, `ruff format .`, `prettier --write .`, `gofmt -w .`)
- `$CHECK_CMD` -- fast type/build check (e.g., `cargo check`, `mypy .`, `tsc --noEmit`, `go vet ./...`)

---

## Fix a bug (TDD)

```
Fix the following bug using test-driven development:

BUG: <describe the bug, including reproduction steps if known>

Steps:
1. Find the root cause by searching the codebase.
2. Write a failing test FIRST that demonstrates the bug.
3. Run $TEST_CMD to confirm the test fails for the right reason.
4. Implement the minimal fix.
5. Run $TEST_CMD to confirm the test passes.
6. Run $FMT_CMD and $LINT_CMD to verify code quality.
7. Do NOT create a PR -- just leave the changes in the worktree.

Coding standards:
- No panicking constructs in production code (unwrap, expect, panic, etc.)
- Use proper error handling (Result, Option, error types)
- Keep the fix minimal -- change as little as possible
```

## Add test coverage for a module

```
Add comprehensive test coverage for the module at: <path/to/module>

Steps:
1. Read the module and understand its public API and edge cases.
2. Identify untested or under-tested code paths.
3. Write tests that cover:
   - Happy path for each public function/method
   - Edge cases (empty input, boundary values, nil/null/None)
   - Error cases (invalid input, missing dependencies)
4. Run $TEST_CMD to verify all tests pass.
5. Run $FMT_CMD and $LINT_CMD.
6. Do NOT create a PR -- just leave the changes in the worktree.

Guidelines:
- Use the project's existing test patterns and helpers
- Each test should test ONE thing and have a descriptive name
- Tests should be deterministic (no randomness, no network, no clock)
- Prefer table-driven / parameterized tests for similar cases
```

## Update documentation

```
Update the documentation for: <feature or component>

Reason: <why it needs updating, e.g., "API changed in PR #123">

Steps:
1. Find all documentation files related to this feature:
   - README sections
   - Doc comments in source code
   - Dedicated docs/ files
   - Example code
2. Read the current implementation to understand the actual behavior.
3. Update documentation to match reality:
   - Fix incorrect descriptions
   - Add missing parameters or options
   - Update examples to use current API
   - Remove references to deprecated features
4. Run $CHECK_CMD to verify any doc-embedded code examples still compile.
5. Do NOT create a PR -- just leave the changes in the worktree.

Guidelines:
- Documentation should describe WHAT and WHY, not just HOW
- Keep examples minimal but complete (they should work if copy-pasted)
- Use consistent terminology throughout
```

## Clean up dead code

```
Find and remove dead code in: <path or module scope>

Steps:
1. Use your project's dead code detection tooling if available:
   - Rust: `cargo machete` for unused deps, compiler warnings for unused code
   - Python: `vulture .` or `ruff check --select F841,F811`
   - TypeScript: `ts-prune` or compiler `noUnusedLocals`
   - Go: `staticcheck ./...`
2. Review each finding manually:
   - Is it truly unreachable, or called via reflection/macros/dynamic dispatch?
   - Is it part of a public API that external consumers might use?
   - Is it intentionally kept for future use (check for TODO/FIXME comments)?
3. Remove confirmed dead code.
4. Run $TEST_CMD to verify nothing breaks.
5. Run $LINT_CMD to confirm cleanliness.
6. Do NOT create a PR -- just leave the changes in the worktree.

Guidelines:
- When in doubt, keep the code -- false positives are worse than missed dead code
- Remove entire files when all their contents are dead
- Update any documentation that references removed code
```

## Convert a shell script to native code

```
Convert the shell script at <path/to/script.sh> to native code in
the project's primary language.

Steps:
1. Read the shell script thoroughly. Understand every command, pipe, and
   conditional.
2. Identify external tool dependencies (grep, sed, awk, jq, curl, etc.)
   and find native equivalents.
3. Write the native implementation:
   - Preserve the exact same behavior and output format
   - Handle all error cases that the shell script handles (and more)
   - Add proper argument parsing if the script takes arguments
   - Follow the project's coding standards
4. Write tests that verify the native code matches the shell script's output
   for representative inputs.
5. Run $TEST_CMD, $FMT_CMD, $LINT_CMD.
6. Do NOT create a PR -- just leave the changes in the worktree.

Guidelines:
- Do not delete the original shell script yet (the PR reviewer will decide)
- Document any behavioral differences in comments
- The native version should be faster and more robust, not just a transliteration
```

## PR a worktree's changes

```
Create a pull request from the changes in this worktree.

Steps:
1. Examine the changes: git diff --stat HEAD, git diff HEAD
2. Validate:
   - $FMT_CMD -- --check (or equivalent dry-run)
   - $LINT_CMD
   - $TEST_CMD (for affected modules at minimum)
   Fix any issues before proceeding.
3. Create a branch:
   - fix/... for bug fixes
   - feat/... for new features
   - test/... for test additions
   - docs/... for documentation
   - chore/... for cleanup
4. Stage and commit with a conventional commit message.
5. Push and create PR with gh pr create.
6. Return the PR URL.
```

## Refactor for readability

```
Refactor the code at <path/to/file> for improved readability.

Constraints:
- NO behavioral changes -- this is a pure refactoring
- All existing tests must continue to pass without modification
- Do not change public API signatures

Steps:
1. Read the code and identify readability issues:
   - Long functions that should be extracted
   - Unclear variable names
   - Missing or misleading comments
   - Complex conditionals that could be simplified
   - Duplicated logic that could be shared
2. Apply refactorings one at a time, running $TEST_CMD after each.
3. Run $FMT_CMD and $LINT_CMD.
4. Do NOT create a PR -- just leave the changes in the worktree.
```

## Add a new feature

```
Implement the following feature:

FEATURE: <description>
ACCEPTANCE CRITERIA:
- <criterion 1>
- <criterion 2>
- <criterion 3>

Steps:
1. Understand the existing architecture around where this feature fits.
2. Write failing tests FIRST that encode the acceptance criteria.
3. Implement the feature to make the tests pass.
4. Run the full test suite: $TEST_CMD
5. Run $FMT_CMD and $LINT_CMD.
6. Do NOT create a PR -- just leave the changes in the worktree.

Coding standards:
- Follow existing patterns in the codebase
- No panicking constructs in production code
- Add doc comments on public items
- Keep the implementation minimal -- do not gold-plate
```

## Bulk dispatch template

Use this as the orchestrator prompt to launch a wave:

```
Launch the following agents in parallel, each in its own worktree:

1. Agent: "fix-<name>"
   Prompt: "<specific bug fix prompt from above>"

2. Agent: "test-<name>"
   Prompt: "<specific test coverage prompt from above>"

3. Agent: "docs-<name>"
   Prompt: "<specific docs prompt from above>"

4. Agent: "cleanup-<name>"
   Prompt: "<specific cleanup prompt from above>"

Use isolation: "worktree" and run_in_background: true for all of them.

After all agents complete, run /bulk-pr to create PRs for all worktrees
with changes.
```
