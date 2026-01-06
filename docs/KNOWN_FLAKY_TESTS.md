# Known Flaky Tests

This document catalogs tests that exhibit non-deterministic behavior (flakiness) in the perl-lsp and perl-parser test suites. Each entry includes root cause analysis, current mitigations, and guidance for reliable local execution.

## Quick Reference

| Test File/Group | Failure Type | Requires | Tracking Issue |
|-----------------|--------------|----------|----------------|
| `lsp_document_symbols_test` | BrokenPipe | `RUST_TEST_THREADS=2` | - |
| `lsp_document_links_test` | BrokenPipe | `RUST_TEST_THREADS=2` | - |
| `lsp_encoding_edge_cases` | BrokenPipe, Timeout | `RUST_TEST_THREADS=2` | Issue #200 |
| `lsp_cancellation_infrastructure_tests` | Timeout, Race | `RUST_TEST_THREADS=1` | Issue #48 |
| `lsp_cancellation_parser_integration_tests` | Timeout, Race | `RUST_TEST_THREADS=1` | Issue #48 |

---

## Flaky Test Details

### 1. lsp_document_symbols_test

**File**: `crates/perl-lsp/tests/lsp_document_symbols_test.rs`

**Symptoms**:
- `BrokenPipe` errors during LSP server communication
- Intermittent timeouts during response reading
- Server shutdown race conditions

**Root Cause**:
The LSP server process may terminate or become unresponsive during concurrent test execution. In constrained CI environments, multiple tests spawning LSP server processes compete for system resources, leading to I/O failures.

**Current Mitigations**:
- Graceful error handling in `common/mod.rs` with `map_send_error()`
- Global mutex `LSP_SERVER_MUTEX` to serialize server creation
- Adaptive timeout scaling based on `RUST_TEST_THREADS`

**Reliable Local Execution**:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_document_symbols_test -- --test-threads=2
```

**Tracking Issue**: None (mitigated)

---

### 2. lsp_document_links_test

**File**: `crates/perl-lsp/tests/lsp_document_links_test.rs`

**Symptoms**:
- `BrokenPipe` errors when sending notifications
- Server process exits unexpectedly

**Root Cause**:
Same resource contention issues as document symbols tests. URL handling and file path operations may trigger edge cases during concurrent execution.

**Current Mitigations**:
- Error tolerant notification sending (ignores `BrokenPipe` during teardown)
- Shared LSP server creation mutex

**Reliable Local Execution**:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_document_links_test -- --test-threads=2
```

**Tracking Issue**: None (mitigated)

---

### 3. lsp_encoding_edge_cases

**File**: `crates/perl-lsp/tests/lsp_encoding_edge_cases.rs`

**Symptoms**:
- Timeouts during Unicode content processing
- `BrokenPipe` errors with complex grapheme clusters
- Performance regression on constrained hardware

**Root Cause**:
Unicode processing (especially emojis, surrogate pairs, and grapheme clusters) requires significantly more processing time. In thread-constrained environments, the combination of LSP server initialization overhead and Unicode parsing can exceed default timeouts.

**Current Mitigations**:
- Adaptive timeout computation via `compute_adaptive_timeout()`:
  ```rust
  if rust_test_threads <= 2 {
      Duration::from_secs(60)  // High contention
  } else if rust_test_threads <= 4 {
      Duration::from_secs(45)  // Medium contention
  } else {
      Duration::from_secs(30)  // Low/no contention
  }
  ```
- Simplified Unicode test cases focused on critical symbols
- Graceful fallback when document symbols request times out

**Reliable Local Execution**:
```bash
# Standard execution with thread constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_encoding_edge_cases -- --test-threads=2

# For specific problematic tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_encoding_edge_cases -- test_emoji_and_special_unicode --nocapture
```

**Tracking Issue**: Issue #200

---

### 4. lsp_cancellation_infrastructure_tests (4 tests)

**File**: `crates/perl-lsp/tests/lsp_cancellation_infrastructure_tests.rs`

**Affected Tests**:
- `test_infrastructure_cleanup_and_resource_management_ac9`
- `test_deadlock_detection_and_prevention_ac10`
- `test_lsp_infrastructure_integration_ac11`
- `test_lsp_regression_prevention_ac11`

**Symptoms**:
- Race conditions during cancellation token operations
- Deadlock detection false positives
- LSP initialization failures in CI environments

**Root Cause**:
These tests validate thread-safety and cancellation infrastructure. Multiple threads interacting with shared cancellation state can cause non-deterministic ordering issues. CI environments with limited resources exacerbate timing-sensitive operations.

**Current Mitigations**:
- Explicit `RUST_TEST_THREADS=1` requirement check at test start
- CI environment detection with graceful skip:
  ```rust
  if std::env::var("CI").is_ok()
      || std::env::var("GITHUB_ACTIONS").is_ok()
      || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
  {
      eprintln!("Skipping in CI environment for stability");
      return;
  }
  ```
- Enhanced retry logic for LSP initialization (up to 2 retries)

**Reliable Local Execution**:
```bash
# Required: Single-threaded execution
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_cancellation_infrastructure_tests -- --test-threads=1

# Individual test execution
RUST_TEST_THREADS=1 cargo test test_infrastructure_cleanup_and_resource_management_ac9 -- --nocapture
```

**Tracking Issue**: Issue #48 (LSP Cancellation Enhancement)

---

### 5. lsp_cancellation_parser_integration_tests (5 tests)

**File**: `crates/perl-lsp/tests/lsp_cancellation_parser_integration_tests.rs`

**Affected Tests**:
- `test_incremental_parsing_checkpoint_cancellation_ac6`
- `test_workspace_indexing_cancellation_integrity_ac7`
- `test_dual_pattern_indexing_cancellation_ac7`
- `test_cross_file_reference_cancellation_ac8`
- `test_multi_tier_resolver_cancellation_ac8`

**Symptoms**:
- Parser fixture initialization timeouts
- Workspace indexing race conditions
- Cross-file reference resolution failures

**Root Cause**:
Parser integration tests create substantial test workspaces (including 2000+ line files) and require full LSP initialization. The combination of large file parsing and cancellation testing creates timing-sensitive scenarios.

**Current Mitigations**:
- Feature-gated stress tests (`#[cfg_attr(not(feature = "stress-tests"), ignore)]`)
- Adaptive fixture initialization timeout:
  ```rust
  let adaptive_timeout = match max_concurrent_threads() {
      0..=2 => Duration::from_secs(30),  // Heavily constrained
      3..=4 => Duration::from_secs(20),  // Moderately constrained
      5..=8 => Duration::from_secs(15),  // Lightly constrained
      _ => Duration::from_secs(10),      // Unconstrained
  };
  ```
- CI environment skip for stability

**Reliable Local Execution**:
```bash
# Required: Single-threaded execution
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_cancellation_parser_integration_tests -- --test-threads=1

# Run stress tests (normally ignored)
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_cancellation_parser_integration_tests --features stress-tests -- --test-threads=1
```

**Tracking Issue**: Issue #48 (LSP Cancellation Enhancement)

---

## General Guidance

### How to Identify a Flaky Test

A test is considered flaky if it exhibits any of these behaviors:

1. **Non-deterministic failures**: Passes sometimes, fails others with identical code
2. **Environment sensitivity**: Fails only in CI or only locally
3. **Thread sensitivity**: Behavior changes with different `RUST_TEST_THREADS` values
4. **Timeout-related failures**: Different timeout thresholds change pass/fail rate
5. **Resource contention symptoms**: `BrokenPipe`, connection refused, or deadlock errors

**Debugging Commands**:
```bash
# Run with maximum verbosity
RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture 2>&1 | tee test_output.log

# Enable LSP debug output
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture

# Enable reader thread debugging
LSP_TEST_DEBUG_READER=1 RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture

# Run with extended timeout
LSP_TEST_TIMEOUT_MS=30000 cargo test <test_name>
```

### How to Report a New Flaky Test

When you encounter a new flaky test, please create a GitHub issue with:

1. **Test Name**: Full test path (e.g., `lsp_cancellation_infrastructure_tests::test_infrastructure_cleanup_and_resource_management_ac9`)

2. **Environment Details**:
   - OS and version
   - Rust version (`rustc --version`)
   - `RUST_TEST_THREADS` value (if set)
   - CI vs local execution

3. **Failure Mode**:
   - Error message (full stack trace if available)
   - Frequency (e.g., "fails 1 in 5 runs")
   - Any patterns (e.g., "only fails when run with other tests")

4. **Reproduction Steps**:
   ```bash
   # Command that reproduces the failure
   cargo test <test_name> -- --nocapture
   ```

5. **Label the Issue**: Use `flaky-test` and `ci-reliability` labels

### Process for Fixing Flaky Tests

1. **Immediate Mitigation**: Add to this document with known workarounds

2. **Short-term Fix Options**:
   - Add thread constraint requirements (`RUST_TEST_THREADS=1`)
   - Implement adaptive timeouts
   - Add retry logic for transient failures
   - Feature-gate stress tests

3. **Long-term Resolution**:
   - Identify and fix root cause (often race conditions)
   - Add proper synchronization primitives
   - Refactor test to be deterministic
   - Consider mocking external dependencies

4. **Validation**:
   - Run test 100+ times locally to verify fix
   - Monitor CI for 1-2 weeks after fix
   - Remove from this document once stable

---

## Environment Variables Reference

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_TEST_THREADS` | Limits concurrent test execution | System core count |
| `LSP_TEST_TIMEOUT_MS` | Default per-request timeout (ms) | 5000 |
| `LSP_TEST_SHORT_MS` | Short timeout for optional responses (ms) | 500 |
| `LSP_TEST_ECHO_STDERR` | Echo perl-lsp stderr in tests | Disabled |
| `LSP_TEST_DEBUG_READER` | Debug LSP reader thread | Disabled |
| `PERL_LSP_BIN` | Explicit path to perl-lsp binary | Auto-detected |

---

## CI Configuration

The CI pipeline uses the following configuration for test reliability:

```yaml
env:
  RUST_TEST_THREADS: "2"  # Balance parallelism and reliability

# For cancellation tests specifically
- name: Run cancellation tests
  env:
    RUST_TEST_THREADS: "1"
  run: cargo test -p perl-lsp --test lsp_cancellation_* -- --test-threads=1
```

---

## Related Documentation

- [Threading Configuration Guide](THREADING_CONFIGURATION_GUIDE.md) - Adaptive threading and concurrency management
- [Cancellation Architecture Guide](CANCELLATION_ARCHITECTURE_GUIDE.md) - LSP cancellation system design
- [LSP Development Guide](LSP_DEVELOPMENT_GUIDE.md) - General LSP testing patterns

---

*Last updated: 2025-12-31*
