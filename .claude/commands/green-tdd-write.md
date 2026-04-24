---
description: Green TDD hardener step 2 — write edge case, boundary, and regression tests
user-invocable: false
---

# Green TDD: Write Tests

Write additional tests that cover edge cases, boundaries, and regressions
the builder's implementation should handle.

## Steps

1. For each untested edge case from your read step, write one test function.

2. For each error path in the builder's implementation, write one test.

3. For boundary conditions:
   - Empty input / empty collection
   - Single element
   - Maximum reasonable size (e.g., 500 @INC paths, 10K-line file)
   - Unicode / multibyte characters
   - None/Some transitions
   - Off-by-one on byte ranges

4. Match the crate's test patterns exactly:
   - Same imports, helpers, naming convention
   - `Result<()>` with `?`, `perl_tdd_support::must`/`must_some`
   - Name tests descriptively: `test_<feature>_<edge_case>_<expected>`

5. ALL tests must pass immediately. These are green tests, not red.
   If a test fails, that reveals a bug in the builder's implementation:
   - Note the failure
   - Comment on the issue flagging the bug
   - Still commit the failing test — the reviewer needs to see it
