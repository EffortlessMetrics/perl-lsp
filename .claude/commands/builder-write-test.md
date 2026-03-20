---
description: Builder step 2 — write the failing test from the spec before implementing the fix
user-invocable: false
---

# Builder Write Test

Write the test BEFORE the fix. TDD: red → green → refactor.

## Steps

1. Read the test code from the spec (step 1 output)

2. Create or open the test file:
   - Parser tests: `crates/perl-parser-core/tests/<descriptive_name>.rs`
   - LSP tests: `crates/perl-lsp-<provider>/tests/<name>.rs`
   - Use snake_case file names matching the test subject

3. Write the test function exactly as specified in the spec.
   If the spec didn't provide exact code, write it based on the
   reproduction from the issue.

4. Run the test — it MUST fail:
   ```bash
   cargo test -p <crate> -- <test_name> --exact 2>&1
   ```

5. If the test passes (unexpected), the bug may already be fixed.
   Check if the issue should be closed instead.

## Standards

- No `unwrap()` or `expect()` in test code — use `Result<()>` returns
  or `perl_tdd_support::must` / `must_some`
- Test names: `test_<what_it_tests>` in snake_case
- One assertion per test when possible

## Output

Record in your task:
```
Test file: <path>
Test function: <name>
Status: FAILING (expected) / PASSING (unexpected — check if already fixed)
```
