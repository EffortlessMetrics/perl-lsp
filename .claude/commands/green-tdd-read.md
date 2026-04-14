---
description: Green TDD hardener step 1 — read the diff, spec files, and oppositional comments for edge cases
user-invocable: false
---

# Green TDD: Read

Understand what the builder implemented and identify untested edge cases.

## Steps

1. Check out the implementation branch:
   ```bash
   git fetch origin
   git checkout impl/<issue#>-<specslug>
   ```

2. Read the builder's diff:
   ```bash
   git diff origin/master..HEAD --stat
   git diff origin/master..HEAD
   ```

3. Read the spec files:
   ```bash
   cat .spec/<issue#>-<specslug>/acceptance.md
   cat .spec/<issue#>-<specslug>/context.md
   ```

4. Read the issue comments for edge cases:
   ```bash
   gh issue view <number> --json comments --jq '.comments[].body'
   ```
   Look for:
   - Oppositional planner objections (especially "what if..." questions)
   - Plan-reviewer edge cases
   - Research-verifier corrections

5. Read existing tests to understand what's already covered:
   - Red-TDD tests (committed earlier on this branch)
   - Builder's tests (if they added any)

6. Identify gaps:
   - Edge cases mentioned in comments but not tested
   - Error paths in the implementation that have no test
   - Boundary conditions (empty, max, None, Unicode, concurrent)
