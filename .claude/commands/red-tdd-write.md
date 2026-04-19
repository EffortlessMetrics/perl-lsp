---
description: Red TDD builder step 2 — write the failing tests
user-invocable: false
---

# Red TDD: Write Tests

Write failing tests that define "done" for this issue. Match the crate's
existing test patterns exactly.

## Steps

> **Absorption issues — API-shape guard:** Before writing any test that references a symbol from an absorbed crate, confirm you read that crate's actual `pub struct` / `pub fn` / `pub trait` / `pub use` declarations in `/red-tdd-read` Step 5. Do not infer `Default`, no-arg `new()`, or field shapes. If you did not capture the exact signature during the read step, go back and read it now. If a signature cannot be located, use `// TODO: signature unclear — API shape TBD` and continue — do not block.

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
   - If testing an absorbed type: use only signatures confirmed during the read step — never add a `Default::default()` or `::new()` call based on Rust convention alone

5. Verify compilation:
   ```bash
   cargo test -p <crate> --no-run
   ```
