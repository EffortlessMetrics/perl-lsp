---
description: Red TDD builder step 2 — write the failing tests
user-invocable: false
---

# Red TDD: Write Tests

Write failing tests that define "done" for this issue. Match the crate's
existing test patterns exactly.

## Steps

1. For each acceptance criterion in `.spec/<issue#>-<specslug>/acceptance.md`, write one test function.

2. For each edge case from oppositional/plan-review comments, write one test function.

3. Match the crate's patterns:
   - Same imports, same helper usage, same naming convention
   - `Result<()>` return type with `?` operator
   - `perl_tdd_support::must` / `must_some` instead of `unwrap()`
   - `insta::assert_snapshot!()` for output/S-expression tests

4. Tests must COMPILE but FAIL:
   - If testing a function that doesn't exist yet, test against the existing API and assert the *absence* of desired behavior
   - If testing a new type, add a minimal stub (empty struct) that compiles but has no implementation
   - Never use `todo!()` or `unimplemented!()` in test code

5. Verify compilation:
   ```bash
   cargo test -p <crate> --no-run
   ```
