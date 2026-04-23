---
description: Red TDD builder step 4 — commit red tests, push, add label, comment on issue
user-invocable: false
---

# Red TDD: Commit

Commit failing tests, push, set the pipeline label, and comment on the issue.

## Steps

1. Stage test files only:
   ```bash
   git add <test-file-paths>
   ```

2. Commit:
   ```bash
   git commit -m "$(cat <<'EOF'
   test(<crate>): add failing tests for #<issue> (red TDD)

   <list of test functions and what each asserts>
   EOF
   )"
   ```

3. Push:
   ```bash
   git push origin impl/<issue#>-<specslug>
   ```

4. **Set the pipeline label** — this signals the builder that red tests are ready:
   ```bash
   gh issue edit <number> --add-label "red-tdd-reviewed"
   ```

5. Comment on the issue:
   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Red TDD: Failing Tests Committed

   **Branch:** `impl/<issue#>-<specslug>`
   **Tests added:** <count>

   | Test | What it asserts (expected to FAIL) |
   |------|-----------------------------------|
   | `test_<name>` | <assertion> |
   | ... | ... |

   **Compilation:** passes (`cargo test --no-run`)
   **Failures:** all <count> tests fail as expected

   Builder: check out this branch and make the tests green.

   ---
   *Red TDD — tests define "done", builder makes them pass.*
   EOF
   )"
   ```
