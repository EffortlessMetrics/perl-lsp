---
description: Red TDD builder step 3 — confirm tests compile but fail
user-invocable: false
---

# Red TDD: Verify

Confirm your tests compile but fail — that's what "red" means.

## Steps

1. Verify tests compile:
   ```bash
   cargo test -p <crate> --no-run
   ```
   If this fails, you have a syntax/import error — fix it before proceeding.

2. Run the tests and confirm they FAIL:
   ```bash
   cargo test -p <crate> -- <test_name_pattern> 2>&1
   ```
   Every new test should show a failure. If a test passes, either:
   - The feature already exists (check if issue is already-fixed)
   - Your assertion isn't testing the right thing — tighten it

3. Run clippy on tests:
   ```bash
   cargo clippy -p <crate> --tests
   ```

4. Run formatter:
   ```bash
   cargo xtask fmt
   ```
