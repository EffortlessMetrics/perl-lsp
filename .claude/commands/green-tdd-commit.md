---
description: Green TDD hardener step 4 — commit, push, comment on issue
user-invocable: false
---

# Green TDD: Commit

Commit your edge case tests, push, and comment on the issue.

## Steps

1. Stage test files only:
   ```bash
   git add <test-file-paths>
   ```

2. Commit:
   ```bash
   git commit -m "$(cat <<'EOF'
   test(<crate>): add edge case and regression tests for #<issue> (green TDD)

   <list of test functions added and what they cover>
   EOF
   )"
   ```

3. Push:
   ```bash
   git push origin impl/<issue#>-<specslug>
   ```

4. Comment on issue:
   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Green TDD: Edge Case Tests Added

   **Branch:** `impl/<issue#>-<specslug>`
   **Tests added:** <count>

   | Test | Edge case covered |
   |------|-------------------|
   | `test_<name>` | <what it tests> |
   | ... | ... |

   **All tests passing:** [yes/no]
   **Bugs found:** [none / list of failing tests with error messages]

   ---
   *Green TDD hardener — edge cases, boundaries, and regression guards.*
   EOF
   )"
   ```

5. **Set the sign-off label:**
   ```bash
   gh issue edit <number> --add-label "green-tdd-reviewed"
   ```

6. If bugs were found (new tests fail), ALSO add the routing label:
   ```bash
   gh issue edit <number> --add-label "needs-builder-fix"
   ```
