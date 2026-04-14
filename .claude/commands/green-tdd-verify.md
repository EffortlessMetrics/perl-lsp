---
description: Green TDD hardener step 3 — run all tests, confirm green
user-invocable: false
---

# Green TDD: Verify

Run all tests — both the red-TDD originals and your new edge case tests.
Everything must pass.

## Steps

1. Run the full test suite for the affected crate:
   ```bash
   cargo test -p <crate>
   ```

2. Run clippy on tests:
   ```bash
   cargo clippy -p <crate> --tests
   ```

3. Run formatter:
   ```bash
   cargo xtask fmt
   ```

4. If any NEW test fails:
   - This reveals a bug in the builder's implementation
   - Note the failure with the test name and error message
   - Keep the failing test — the reviewer needs to see it
   - Comment on the issue flagging the specific failure

5. If any EXISTING test fails:
   - This is a regression from the builder's changes
   - Flag it immediately — this blocks the PR
   - Comment on the issue with the failing test and error
