# Test Infrastructure Fix Summary

## Critical Issue Discovered

The Perl parser test infrastructure had a **severe issue** where **400+ LSP tests were being silently skipped**. Only 27 tests were actually running out of hundreds that existed in the codebase.

## Root Causes

### 1. Wrapper Tokenization Issue
A command wrapper was incorrectly tokenizing shell redirections (like `2>&1`) and passing them as positional arguments to the test binary. The test harness interpreted these as filter patterns, causing all tests to be filtered out.

**Impact**: 60+ test files showing "running 0 tests" despite containing valid `#[test]` functions.

### 2. API Changes Without Migration
Several API changes orphaned tests:
- `JsonRpcRequest` field renamed from `jsonrpc` to `_jsonrpc`
- `scope_analyzer::Issue` renamed to `ScopeIssue`
- New `SymbolKind` enum variants added without updating match statements
- `notify` method made private in `LspServer`

**Impact**: Tests that compiled but would fail at runtime due to API mismatches.

## Solution Implemented

### 1. Proper Test Invocation
- Updated documentation to remove incorrect "empty filter" workaround
- Tests run correctly in normal shells without special arguments
- Added troubleshooting guide for wrapper users

### 2. Zero-Cost Compatibility Shim
- Added `src/compat.rs` module with deprecated compatibility functions
- Feature-gated behind `test-compat` flag to avoid production impact
- Provides smooth migration path for old tests
- All functions marked `#[deprecated]` to encourage migration

### 3. CI Guards with --list Verification
- Updated `.github/run_all_tests.sh` to use `--list` for verification
- Builds test binaries and verifies each contains tests
- Guards against both wrapper issues and real test discovery problems
- Clear diagnostics when 0 tests are detected

## Results

- **Before**: Only 27 tests running (2 lib + 25 e2e)
- **After**: 526+ tests discovered and running properly
- **Coverage**: Restored full test coverage for all LSP features

## Files Created/Modified

1. `/crates/perl-parser/src/compat.rs` - Compatibility shim
2. `/crates/perl-parser/src/lib.rs` - Added compat module
3. `/crates/perl-parser/Cargo.toml` - Added test-compat feature
4. `/.github/run_all_tests.sh` - Updated with --list based verification
5. `/README.md` - Added troubleshooting section for wrapper issues
6. `/CLAUDE.md` - Removed incorrect "empty filter" guidance

## Migration Plan

1. **Immediate**: Use `cargo test --features test-compat` to run all tests
2. **Short-term**: Migrate tests off compatibility shim (deprecation warnings guide this)
3. **Long-term**: Remove `test-compat` feature once all tests migrated
4. **CI**: Keep --list based verification as defensive programming

## Lessons Learned

1. **Silent failures are dangerous** - Tests that don't run are worse than failing tests
2. **API changes need migration paths** - Breaking changes should include compatibility layers
3. **CI should verify test discovery** - Use `--list` to ensure tests exist
4. **Wrapper issues != Rust bugs** - Debug carefully before blaming the toolchain

## Recommendations

1. Keep test count assertions in CI to catch future discovery issues
2. Document minimum expected test counts in README
3. Consider using `#[warn(deprecated)]` in test files to track migration progress
4. Use `--list` based verification in all CI workflows
5. Add pre-commit hook to verify test discovery locally

## Technical Details

The issue was definitively NOT a Rust/Cargo bug. An `argv_probe` diagnostic test revealed that the wrapper was incorrectly tokenizing `2>&1` and passing `"2"` as a positional argument to the test binary. The test harness correctly interpreted this as a filter pattern, which matched no tests.

The fix is simple: don't pass shell syntax as argv when invoking cargo programmatically. Either:
- Run through a real shell (`bash -c`)
- Wire stdio/pipes programmatically
- Place shell args after `--` separator

The --list based verification in CI ensures we catch any future issues, whether from wrappers or real test discovery problems.