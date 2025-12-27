# Ignored Tests Index

> **Status**: Band 2 analysis complete. Ready for systematic re-enablement.
> **Last Updated**: 2025-12-27

## Summary

| Category | Count | % | Root Cause | Fix Strategy |
|----------|-------|---|------------|--------------|
| BrokenPipe Initialization | 688 | 93% | LSP init timing race | Migrate to `LspHarness` |
| Double Initialization | 5 | 0.7% | Test infrastructure | Fix test setup |
| Cancellation Env | 8 | 1% | Environment config | Stabilize env vars |
| perl-parser (various) | 53 | 7% | Mixed reasons | Case-by-case |
| **Total** | **~793** | 100% | | |

## Root Cause Analysis

### Primary Issue: BrokenPipe Initialization (688 tests)

**Pattern observed:**
```
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
```

**Root Cause**: Tests use the old `TestContext` pattern which:
1. Creates `LspServer` directly without proper initialization sequence
2. Doesn't wait for `initialized` notification to be processed
3. Missing barrier synchronization before sending requests
4. No graceful shutdown handling

**Evidence**: The `LspHarness` (in `tests/support/lsp_harness.rs`) already fixes these issues:
- Adaptive timeout scaling (200-800ms based on thread count)
- Barrier synchronization after initialization
- Proper shutdown sequence
- CI environment detection

**Fix**: Migrate tests from `TestContext` to `LspHarness`.

### Secondary Issues

| Issue | Count | Files | Fix |
|-------|-------|-------|-----|
| Double initialization | 5 | `lsp_code_actions_comprehensive_tests_enhanced.rs` | Ensure single init per test |
| Cancellation env | 8 | `lsp_cancellation_*.rs` | Stabilize env vars before test |
| Stress tests | 10 | Various `*_stress_*.rs` | Tag as `#[cfg(feature = "slow")]` |

## Files by Ignored Test Count (Top 20)

| File | Count | Strategy |
|------|-------|----------|
| `lsp_comprehensive_3_17_test.rs` | 59 | **Priority 1**: Convert to LspHarness |
| `lsp_comprehensive_e2e_test.rs` | 33 | **Priority 1**: Uses TestContext directly |
| `lsp_protocol_violations.rs` | 26 | **Priority 2**: Protocol compliance tests |
| `lsp_execute_command_comprehensive_tests.rs` | 25 | **Priority 2**: Execute command tests |
| `lsp_advanced_features_test.rs` | 23 | **Priority 3**: Feature tests |
| `lsp_window_progress_test.rs` | 21 | **Priority 3**: Window progress |
| `lsp_error_recovery_behavioral_tests.rs` | 21 | **Priority 3**: Error recovery |
| `lsp_unhappy_paths.rs` | 19 | **Priority 4**: Edge cases |
| `lsp_filesystem_failures.rs` | 17 | **Priority 4**: FS failure handling |
| `lsp_completion_tests.rs` | 17 | **Priority 4**: Completion tests |
| `lsp_integration_tests.rs` | 16 | **Priority 4**: Integration |
| `lsp_full_coverage_user_stories.rs` | 16 | **Priority 4**: User stories |
| `lsp_e2e_user_stories.rs` | 16 | **Priority 4**: E2E user stories |
| `lsp_api_contracts.rs` | 15 | **Priority 5**: API contracts |
| `lsp_schema_validation.rs` | 14 | **Priority 5**: Schema validation |
| `lsp_memory_pressure.rs` | 14 | Tag as slow test |
| `lsp_encoding_edge_cases.rs` | 14 | **Priority 5**: Encoding tests |
| `lsp_workspace_file_ops_tests.rs` | 13 | **Priority 5**: Workspace ops |
| `lsp_signature_integration_test.rs` | 13 | **Priority 5**: Signature help |
| `lsp_performance_benchmarks.rs` | 13 | Tag as slow test |

## Re-enablement Plan

### Phase 1: Priority 1 Files (92 tests, ~12% of total)

Target: `lsp_comprehensive_3_17_test.rs` (59) + `lsp_comprehensive_e2e_test.rs` (33)

**Action**:
1. Replace `TestContext` with `LspHarness`
2. Add `barrier()` calls after initialization
3. Use adaptive timeouts
4. Run with `RUST_TEST_THREADS=2` to validate

### Phase 2: Priority 2-3 Files (~115 tests, ~15%)

Target: Protocol/command/feature tests

**Action**:
1. Apply same pattern as Phase 1
2. Group by functional area for efficient review

### Phase 3: Priority 4-5 Files (~200 tests, ~27%)

Target: Integration and edge case tests

**Action**:
1. Continue migration pattern
2. Document any tests that need different handling

### Phase 4: Remaining + Cleanup (~340 tests, ~46%)

Target: All remaining tests

**Action**:
1. Complete migration
2. Tag legitimately slow tests with `#[cfg(feature = "slow")]`
3. Document any permanently ignored tests with clear justification

## Legitimate Ignores (Keep Ignored)

| Test | File | Reason |
|------|------|--------|
| Performance benchmarks | `lsp_performance_benchmarks.rs` | Slow, run separately |
| Memory pressure tests | `lsp_memory_pressure.rs` | Resource-intensive |
| Stress tests | Various | Run with `--ignored` flag |

## Progress Tracking

- [ ] Phase 1: 0/92 tests re-enabled
- [ ] Phase 2: 0/115 tests re-enabled
- [ ] Phase 3: 0/200 tests re-enabled
- [ ] Phase 4: 0/340 tests re-enabled
- [ ] Final count: <100 ignored with documented reasons

## Commands for Validation

```bash
# Count current ignored tests
grep -r '#\[ignore\]' crates/perl-lsp/tests/*.rs | wc -l

# Run ignored tests to verify they pass
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --ignored --test-threads=2

# Run specific file's ignored tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_3_17_test -- --ignored --test-threads=2
```
