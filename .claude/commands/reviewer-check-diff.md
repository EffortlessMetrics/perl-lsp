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

5. Check for **correctness**:
   - Does the logic match the issue's recommended approach?
   - Are error paths handled?
   - Any obvious bugs?

6. **Fix forward** — for anything you find:
   - Banned pattern? Fix it and commit.
   - Missing test? Write it and commit.
   - Naming could be better? Rename it and commit.
   - Push improvements directly to the PR branch rather than listing them as comments.

## Output

Record in your task:
```
Improvements pushed: <list of changes you made>
Remaining blockers: <list or NONE>
Scope: CLEAN / CREEP (list extra files)
```
