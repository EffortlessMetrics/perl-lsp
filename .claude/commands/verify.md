---
description: Run standard verification pipeline (fmt, clippy, test) for a crate
argument-hint: "<crate-name> [--skip-fmt] [--skip-clippy] [--skip-test]"
---

# Verify Crate

Run the standard three-step verification pipeline for a single crate. This replaces inline `Verify:` steps in agent prompts. Context: **$ARGUMENTS**

## Steps

### 1. Parse arguments

Extract the crate name from `$ARGUMENTS`. The crate name is the first positional argument (e.g., `perl-parser`, `perl-lsp`, `perl-lexer`).

Optional flags:
- `--skip-fmt`: Skip formatting check
- `--skip-clippy`: Skip clippy lint
- `--skip-test`: Skip test run

### 2. Verify crate exists
```bash
cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | grep -qx "<crate>"
```

If the crate is not found, report an error and list similar crate names.

### 3. Run formatting check
```bash
cargo fmt -p <crate> -- --check
```

If formatting fails, fix it:
```bash
cargo fmt -p <crate>
```

Record: PASS / FAIL (auto-fixed)

### 4. Run clippy
```bash
cargo clippy -p <crate> --tests -- -D warnings
```

Record: PASS / FAIL

If clippy fails, output the specific warnings/errors for the agent to fix.

### 5. Run tests
```bash
cargo test -p <crate>
```

For `perl-lsp` specifically, use threading constraints:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

Record: PASS / FAIL

### 6. Report

```
### Verification: <crate>
| Step    | Result | Duration |
|---------|--------|----------|
| fmt     | PASS/FAIL | Xs    |
| clippy  | PASS/FAIL | Xs    |
| test    | PASS/FAIL | Xs    |

**Overall**: PASS / FAIL
**Failures**: <details if any>
```

### 7. Exit status

If all steps pass, report overall PASS.
If any step fails, report overall FAIL with details of the first failure.

Agents should use this result to decide whether to commit and create a PR.
