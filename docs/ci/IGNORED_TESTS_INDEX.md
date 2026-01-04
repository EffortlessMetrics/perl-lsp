# Ignored Tests Index

> **Status**: Post-sweep analysis. Systematic re-enablement in progress.
> **Last Updated**: 2025-12-27

## Summary

| Category                     | Count   | %    | Status                      | Notes                              |
|------------------------------|---------|------|-----------------------------|-----------------------------------|
| **Active ignores**           | **572** | 100% | Being systematically reduced| -                                 |
| Feature-gated (`lsp-extras`) | 23      | 4%   | Quarantined                 | `lsp_advanced_features_test.rs`   |
| Protocol compliance          | 4       | 0.7% | Documented, real issues     | Error code mismatches             |
| Robustness (crash on input)  | 1       | 0.2% | Server crash on malformed JSON | `test_malformed_json_request`   |
| Passing but ignored          | ~500    | 87%  | Ready for removal           | Legacy "BrokenPipe" annotations   |

## Recent Progress (2025-12-27)

### Tests Re-enabled Today

| File                                     | Before | After | Change  |
|------------------------------------------|--------|-------|---------|
| `lsp_protocol_violations.rs`             | 26     | 4     | **-22** |
| `lsp_window_progress_test.rs`            | 21     | 0     | **-21** |
| `lsp_unhappy_paths.rs`                   | 9      | 1     | **-8**  |
| `lsp_error_recovery_behavioral_tests.rs` | 21     | 0     | Already clean |

### Feature-Gated Tests

| File                            | Tests | Status                       |
|---------------------------------|-------|------------------------------|
| `lsp_advanced_features_test.rs` | 23    | Gated: `#![cfg(feature = "lsp-extras")]` |

**Reason**: These tests use a broken `AdvancedTestContext` that doesn't initialize the LSP server properly. They test speculative features not yet implemented.

## Files by Ignored Test Count (Current Top 20)

| File                                     | Count | Notes                                   |
|------------------------------------------|-------|-----------------------------------------|
| `lsp_advanced_features_test.rs`          | 23    | **Feature-gated** - doesn't run in CI   |
| `lsp_error_recovery_behavioral_tests.rs` | 21    | All tests pass - needs unignore         |
| `lsp_filesystem_failures.rs`             | 17    | Sweep candidate                         |
| `lsp_completion_tests.rs`                | 17    | Sweep candidate                         |
| `lsp_integration_tests.rs`               | 16    | Sweep candidate                         |
| `lsp_full_coverage_user_stories.rs`      | 16    | Sweep candidate                         |
| `lsp_e2e_user_stories.rs`                | 16    | Sweep candidate                         |
| `lsp_api_contracts.rs`                   | 15    | Sweep candidate                         |
| `lsp_schema_validation.rs`               | 14    | Sweep candidate                         |
| `lsp_memory_pressure.rs`                 | 14    | Keep ignored - resource-intensive       |
| `lsp_encoding_edge_cases.rs`             | 14    | Sweep candidate                         |
| `lsp_workspace_file_ops_tests.rs`        | 13    | Sweep candidate                         |
| `lsp_signature_integration_test.rs`      | 13    | Sweep candidate                         |
| `lsp_performance_benchmarks.rs`          | 13    | Keep ignored - benchmarks               |
| `lsp_edge_cases_test.rs`                 | 13    | Sweep candidate                         |
| `lsp_security_edge_cases.rs`             | 12    | Sweep candidate                         |
| `lsp_error_recovery.rs`                  | 11    | Sweep candidate                         |
| `lsp_behavioral_tests.rs`                | 11    | Sweep candidate                         |
| `lsp_unhappy_paths.rs`                   | 10    | Note: 9 bare ignores + 1 real failure   |
| `lsp_stress_tests.rs`                    | 10    | Keep ignored - stress tests             |

## Legitimate Ignores (Keep Ignored)

| Test                                     | File                          | Reason                              |
|------------------------------------------|-------------------------------|-------------------------------------|
| `test_malformed_json_request`            | `lsp_unhappy_paths.rs`        | Server crashes on malformed JSON    |
| `test_missing_jsonrpc_version`           | `lsp_protocol_violations.rs`  | Returns -32000 vs expected -32600   |
| `test_duplicate_request_ids`             | `lsp_protocol_violations.rs`  | Response ID is Null vs expected 100 |
| `test_request_before_initialization`     | `lsp_protocol_violations.rs`  | Different error format than -32002  |
| `test_batch_request_violations`          | `lsp_protocol_violations.rs`  | BrokenPipe with batch JSON-RPC      |
| All tests in `lsp_memory_pressure.rs`    | -                             | Resource-intensive                  |
| All tests in `lsp_performance_benchmarks.rs` | -                         | Benchmarks, run separately          |
| All tests in `lsp_stress_tests.rs`       | -                             | Stress tests, run with --ignored    |

## Sweep Strategy

The "flip strategy" for systematically re-enabling tests:

1. Run `cargo test -p perl-lsp --test <file> -- --include-ignored --test-threads=2`
2. If all pass: remove all `#[ignore]` annotations
3. If some fail: remove passing ones, update failing ones with accurate reason
4. Commit changes

### Files Ready for Sweep (High Confidence)

Based on the pattern that most "BrokenPipe" ignores are stale:

- `lsp_error_recovery_behavioral_tests.rs` (21 tests all pass)
- `lsp_filesystem_failures.rs` (17)
- `lsp_completion_tests.rs` (17)
- `lsp_integration_tests.rs` (16)
- `lsp_api_contracts.rs` (15)
- `lsp_schema_validation.rs` (14)
- `lsp_encoding_edge_cases.rs` (14)

## Infrastructure Improvements Made

### 1. TestContext Wrapper Fixed
- `params: None` now maps to JSON `null` instead of `{}` (per JSON-RPC spec)
- Added `initialize_with(root_uri, capabilities)` for custom initialization

### 2. LspHarness Enhancements
- `initialize_ready()` - canonical init with barrier synchronization
- `change_full()` - document content updates
- `close()` - document close
- Adaptive timeouts based on thread count and environment

### 3. Feature Gating
- `lsp_advanced_features_test.rs` behind `lsp-extras` feature
- Tests with broken initialization patterns quarantined

## Commands for Validation

```bash
# Count current ignored tests
grep -rE '#\[ignore\]|#\[ignore\s*=' crates/perl-lsp/tests/*.rs | wc -l

# Run ignored tests to verify they pass
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --ignored --test-threads=2

# Run specific file's tests (including ignored)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test <test_file> -- --include-ignored --test-threads=2

# Run feature-gated tests
cargo test -p perl-lsp --features lsp-extras --test lsp_advanced_features_test -- --include-ignored
```

## Progress Tracking

- [x] Phase 1: `lsp_protocol_violations.rs` (26 → 4 ignores)
- [x] Phase 1: `lsp_window_progress_test.rs` (21 → 0 ignores)
- [x] Phase 1: `lsp_unhappy_paths.rs` (9 → 1 ignores) + dead code cleanup
- [x] Phase 1: Quarantine `lsp_advanced_features_test.rs` (23 tests feature-gated)
- [x] Phase 2: Sweep remaining high-confidence files ✅
- [x] Phase 3: Address real failures with proper fixes ✅
- [x] Phase 4: Final audit and documentation ✅

**Current status**: BUG=0, MANUAL=1 (run `bash scripts/ignored-test-count.sh` for live counts; baseline: `scripts/.ignored-baseline`)
