---
description: Refactor planner step 1 — read the diff, existing code patterns, and review comments
user-invocable: false
---

# Refactor Planner: Read

Understand the builder's implementation and the crate's existing patterns.

## Steps

1. Check out the PR:
   ```bash
   gh pr checkout <number>
   ```

2. Read the diff:
   ```bash
   git diff origin/master..HEAD
   ```

3. Read existing code in the same module for patterns:
   - How are helpers structured?
   - What error handling patterns are used?
   - What naming conventions?

4. Run clippy to find mechanical issues:
   ```bash
   cargo clippy -p <crate> --tests 2>&1
   ```

5. Read review comments for simplification hints:
   ```bash
   gh pr view <number> --json comments --jq '.comments[].body'
   ```
