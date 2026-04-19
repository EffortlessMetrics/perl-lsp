---
description: Red TDD builder step 1 — read the issue, spec-planner checklist, and existing test patterns
user-invocable: false
---

# Red TDD: Read

Read the issue, the spec-planner's checklist, and the target crate's
existing test patterns.

## Steps

1. Read the issue and comments:
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Check out the implementation branch (created by spec-planner):
   ```bash
   git fetch origin
   git checkout impl/<issue#>-<specslug>
   ```

3. Read the spec files:
   ```bash
   cat .spec/<issue#>-<specslug>/checklist.md
   cat .spec/<issue#>-<specslug>/acceptance.md
   ```

4. Read existing tests in the target crate to understand patterns:
   - What test framework? (inline `#[cfg(test)]` or `tests/` directory?)
   - What helpers are used? (`LspHarness`, `MockSubprocessRuntime`, `tempfile`, etc.)
   - What import patterns? (`use perl_tdd_support::must;`, `use insta::assert_snapshot;`, etc.)
   - How are test functions named?

5. For issues involving crate absorption or module refactoring (any issue where a crate's symbols move into a new module), read each absorbed crate's actual public API **before** writing any test:

   a. Check whether the source crate still exists on this branch:
      ```bash
      ls crates/<absorbed-crate>/src/lib.rs
      ```
   b. If it exists, read it:
      ```bash
      cat crates/<absorbed-crate>/src/lib.rs
      ```
      Then follow any `pub use` chains into sub-modules to locate the actual struct/fn/trait declarations.
   c. If the source crate has already been absorbed (file not found — prior wave merged it), read the destination module instead:
      ```bash
      cat crates/<dest-crate>/src/providers/<module>.rs
      ```
      Inspect its `pub struct`, `pub fn`, `pub trait`, and `pub use` items for exact signatures.
   d. Record the exact signatures you will test against. Do not infer `Default`, no-arg `new()`, field types, or trait bounds — use only what you read.
   e. If a signature cannot be located after checking both source and destination, write the test stub with a prominent comment:
      ```rust
      // TODO: signature unclear — API shape TBD. Builder: verify before making this green.
      ```

6. Identify from acceptance.md:
   - Each criterion that needs a test
   - Edge cases mentioned in oppositional/plan-review comments
   - The exact assertions that define "done"
