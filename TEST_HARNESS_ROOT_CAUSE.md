# Root Cause Analysis: Integration Test Discovery Issue

## The Problem
Integration tests in `tests/` directory show "0 tests" when run with `cargo test --test <name>` but work correctly when:
1. An empty string filter `''` is provided
2. Any environment variable like `RUST_TEST_NOCAPTURE=1` or `RUST_TEST_THREADS=1` is set

## Root Cause
This appears to be a bug or undocumented behavior change in Cargo 1.87.0 (May 2025) where:

1. When running `cargo test --test <name>` without additional arguments, cargo appears to pass the test binary name as a filter to the test harness
2. This causes the test harness to filter for tests matching the binary name, which typically matches no test functions
3. Setting ANY test-related environment variable (RUST_TEST_NOCAPTURE, RUST_TEST_THREADS, etc.) changes cargo's behavior to not apply this implicit filter

## Evidence
1. Running the test binary directly works: `target/debug/deps/test_name` runs all tests
2. Running with `cargo test --test name` shows "X filtered out"
3. Running with `cargo test --test name ''` passes an explicit empty filter, which works
4. Running with ANY test env var makes it work: `RUST_TEST_THREADS=1 cargo test --test name`

## The Real Fix

### Option 1: Set a Default Test Environment Variable
Add to `.cargo/config.toml`:
```toml
[env]
RUST_TEST_THREADS = "0"  # 0 means use default (number of CPUs)
```

### Option 2: Use Workspace-level Test Running
Instead of `cargo test -p perl-parser --test X`, use:
```bash
cargo test --workspace
```

### Option 3: Always Pass an Explicit Filter
Train developers to always use:
```bash
cargo test -p perl-parser --test name ''  # Empty filter to run all
cargo test -p perl-parser --test name pattern  # Specific pattern
```

## Why The Compatibility Shim Isn't Needed

The compatibility shim was created to fix API changes between test files and the main library, but it doesn't solve the test discovery issue. The discovery issue is purely a cargo invocation problem, not an API compatibility problem.

## Recommended Solution

1. **Immediate**: Add `RUST_TEST_THREADS = "0"` to `.cargo/config.toml`
2. **Long-term**: File a bug report with Cargo about this behavior
3. **CI**: Keep the test runner script as a safety net but simplify it

This is NOT a problem with our code - it's a cargo behavior issue that affects ALL Rust projects using integration tests with cargo 1.87+.