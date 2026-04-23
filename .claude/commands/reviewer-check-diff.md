---
description: Reviewer step 2 — check the diff for correctness, standards, and scope
user-invocable: false
---

# Reviewer Check Diff

Read the actual diff and check for issues.

## Steps

1. Read the diff:
   ```bash
   gh pr diff <number>
   ```

2. Check for **banned patterns** (instant blockers):
   - `unwrap()`, `expect()`, `panic!()` in non-test code
   - `todo!()`, `unimplemented!()`, `dbg!()`
   - `std::process::exit()` outside bin/ and lifecycle.rs
   - Hardcoded secrets, paths, or credentials

3. Check for **scope creep**:
   - Does every changed file relate to the issue?
   - Are there "bonus" refactors or improvements?
   - Does the diff touch files outside the spec?

4. Check for **missing tests**:
   - Does the PR add a test for the changed behavior?
   - Are edge cases covered?

5. Check for **vacuous assertions** — tests that assert properties of locally-constructed data:
   - Does the test still pass if you comment out the feature code? If yes, the test is not testing the feature.
   - Watch for: `assert!(vec.len() > 0)` on a `Vec` built from hardcoded items, `assert!(!s.is_empty())` on string literals, `assert_eq!(result.len(), N)` when result was built from N hardcoded items, `assert!(result.is_some())` when result is `Some(hardcoded_value)`.
   - The tell: the assertion proves a property of the test data, not the code under test.

6. Check for **correctness**:
   - Does the logic match the issue's recommended approach?
   - Are error paths handled?
   - Any obvious bugs?

7. **Fix forward** — for anything you find:
   - Banned pattern? Fix it and commit.
   - Missing test? Write it and commit.
   - Vacuous assertion? Rewrite to test actual behavior of the code under test.
   - Naming could be better? Rename it and commit.
   - Push improvements directly to the PR branch rather than listing them as comments.

## Output

Record in your task:
```
Improvements pushed: <list of changes you made>
Remaining blockers: <list or NONE>
Scope: CLEAN / CREEP (list extra files)
```
