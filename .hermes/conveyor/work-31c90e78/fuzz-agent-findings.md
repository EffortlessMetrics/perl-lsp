# Fuzz Testing Report — work-31c90e78

## Change Summary
This PR adds include path scanning for module completion in the Perl LSP. When completing `use` or `require` statements, the LSP now suggests modules from configured include paths (`.perl-lsp.toml` `includePaths`), `PERL5LIB`, and system `@INC` in addition to modules found in the workspace index.

Key files:
- `crates/perl-lsp-completion/src/completion/workspace.rs` - Main implementation with `scan_modules_in_directory` and `path_to_module_name`
- `crates/perl-lsp-completion/src/completion.rs` - Added `include_paths` and `system_inc_paths` fields to `CompletionProvider`

## Fuzz Testing Applicability

**Yes, fuzzing is applicable here.** The change has meaningful input surfaces:

- **Input surface**: File paths from include directories - directories with `.pm` files to scan
- **Parsing boundaries**: `path_to_module_name` converts file paths to Perl module names
- **Algorithmic surface**: Directory traversal with `WalkDir`, filtering, deduplication, timeout/cancellation

### What was fuzzed:
1. `path_to_module_name` - pathological paths, unusual separators, dot components
2. `scan_modules_in_directory` - timeout, cancellation, depth limits, entry limits
3. Full completion pipeline - deduplication across many paths, empty/non-existent paths

## Fuzz Targets Written

All fuzz targets are in `crates/perl-lsp-completion/tests/inc_path_fuzz.rs`:

| Target | What it tests | Iterations | Crashes Found |
|--------|--------------|------------|---------------|
| `fuzz_path_traversal_attempts_do_not_cause_panics` | Path traversal (`..`), dot components | 1 | 0 |
| `fuzz_mixed_separators_and_dot_components` | Double/triple slashes, leading dots, wrong extensions | 1 | 0 |
| `fuzz_cancellation_returns_partial_results_without_panic` | Always-cancelled callback | 1 | 0 |
| `fuzz_frequent_cancellation_checks_no_panic` | Alternating cancel callback | 1 | 0 |
| `fuzz_depth_limit_with_generated_nested_paths` | WalkDir depth enforcement (depth 1-8) | 1 | 0 |
| `fuzz_special_characters_in_module_names` | Underscores, numbers, Unicode, spaces | 1 | 0 |
| `fuzz_empty_include_paths_do_not_cause_panic` | Empty include path list | 1 | 0 |
| `fuzz_nonexistent_include_path_does_not_cause_panic` | Non-existent directory | 1 | 0 |
| `fuzz_deduplication_with_many_modules_and_paths` | 4 paths × 10 modules = 40 entries, deduplicated | 1 | 0 |
| `fuzz_very_long_path_components_do_not_cause_panic` | 10,000 char path component | 1 | 0 |
| `fuzz_module_with_special_names` | Just `.pm`, trailing dots, slashes | 1 | 0 |
| `fuzz_empty_prefix_with_many_modules_no_timeout` | 500 modules within 200ms | 1 | 0 |
| `fuzz_unicode_module_names` | German, Japanese, Greek, Cyrillic, emoji | 1 | 0 |

**Total: 13 fuzz targets, 0 crashes found**

## Files Changed

| File | What changed | Fuzz surface |
|------|-------------|--------------|
| `crates/perl-lsp-completion/src/completion/workspace.rs` | Added `scan_modules_in_directory` and `path_to_module_name` | **Yes** - path parsing, directory traversal |
| `crates/perl-lsp-completion/src/completion.rs` | Added `include_paths`/`system_inc_paths` fields | **No** - mechanical fields |
| `crates/perl-lsp-completion/tests/inc_path_fuzz.rs` | **New fuzz test file** | **Yes** - tests above surfaces |

## Crashes Found

**No crashes found.** All 13 fuzz targets exercised without panics or unexpected errors.

### Pre-existing Test Bug Discovered

The fuzz testing revealed that the existing `property_prefix_filtering_exact` test has an incorrect assertion (Bug 1 in the specs):

- **Location**: `crates/perl-lsp-completion/tests/inc_path_property_tests.rs:241-245`
- **Issue**: Test asserts `Alphabet::Module` should NOT appear for prefix "Alpha"
- **Reality**: `Alphabet.starts_with("Alpha")` is `true`, so `Alphabet::Module` correctly appears
- **Fix needed**: Change the test to use `Beta::Module` as the negative case (since `Beta` does not start with `Alpha`)

My fuzz tests correctly verified the implementation works as expected - the prefix filtering is correct, the test assertion is wrong.

## Summary

- Fuzz targets written: 13
- Crashes found: 0
- Regression tests added: 0 (existing tests cover regressions; fuzz file is new)
- Coverage assessment:
  - `path_to_module_name`: Covered pathological paths, Unicode, special characters
  - `scan_modules_in_directory`: Covered depth limits, cancellation, timeouts, empty paths
  - Deduplication: Covered multi-path deduplication

## Recommendations

1. **Fix Bug 1 in `property_prefix_filtering_exact`**: The test assertion is wrong. Change to assert `Beta::Module` does NOT appear for prefix "Alpha".

2. **Consider adding randomized fuzzing**: The current tests are deterministic. A true fuzzing harness (libfuzzer, AFL) could generate random path patterns more efficiently.

3. **Symlink traversal**: The code uses `follow_links(false)` for security. Consider adding a test with symlinks to verify they are not followed (edge case for actual filesystem setup).
