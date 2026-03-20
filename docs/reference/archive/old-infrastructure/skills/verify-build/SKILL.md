---
name: verify-build
description: Run the standard per-crate verification pipeline and report a receipt. Use when a worker needs fmt, clippy, and test results for a specific crate before handoff, review, or PR creation.
argument-hint: "<crate-name> [--skip-fmt] [--skip-clippy] [--skip-test]"
---

# Verify Build

Run the standard verification pipeline for one crate. Context: **$ARGUMENTS**

This skill verifies a deliverable and produces a receipt. It does not replace
`/pr-create`, commit flow, or reviewer/ops handoff.

## Steps

1. Parse the crate name from the first positional argument
2. Confirm the crate exists with `cargo metadata`
3. Run formatting unless `--skip-fmt` is present
4. Run clippy unless `--skip-clippy` is present
5. Run tests unless `--skip-test` is present
6. If the change added, removed, or materially changed tests, run
   `python3 scripts/update-current-status.py` then `just status-check`
   before handoff so computed docs stay fresh
7. Verify new functions are called from an entry point (not just defined)
8. Report a receipt with pass/fail status for each step

## Commands

Check the crate exists:

```bash
cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | grep -qx "<crate>"
```

Formatting:

```bash
cargo fmt -p <crate> -- --check
```

If formatting fails, fix it with:

```bash
cargo fmt -p <crate>
```

Clippy:

```bash
cargo clippy -p <crate> --tests -- -D warnings
```

Tests:

```bash
cargo test -p <crate>
```

For `perl-lsp`, use:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

Status refresh (REQUIRED when tests were added, removed, or changed):

```bash
python3 scripts/update-current-status.py
just status-check
```

This prevents the `policy_checks` CI gate from failing due to stale derived
metrics in `docs/project/CURRENT_STATUS.md`. Always run after modifying tests.

Wiring check (REQUIRED for new features and enhancements):

Verify that new public functions or methods are actually called from an entry
point. Grep for the function name — if no call site exists outside its
definition and tests, the feature is built but not wired and will be dead code
in production.

## Receipt Format

Report:

- crate name
- fmt result
- clippy result
- test result
- whether status refresh was required and run
- wiring check result (for new features)
- overall pass/fail
- first failure details if anything failed

Use that receipt in the handoff, reviewer briefing, or PR body.
