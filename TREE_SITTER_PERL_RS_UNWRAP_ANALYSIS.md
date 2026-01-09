# tree-sitter-perl-rs Unwrap Analysis

## Executive Summary

**Key Finding**: tree-sitter-perl-rs unwraps (761 total) are **NOT in production code**.

- **tree-sitter-perl-rs** is **excluded from the workspace** (Cargo.toml line 14)
- **perl-lsp** and **perl-parser** do NOT depend on tree-sitter-perl-rs
- All 761 unwraps in tree-sitter-perl-rs are test/benchmark infrastructure only

## Impact

By excluding tree-sitter-perl-rs from production unwrap counting:
- **Before**: 1279 production unwraps (incorrect, included tree-sitter-perl-rs)
- **After**: 518 production unwraps (correct, workspace crates only)
- **Reduction**: 59% (761 unwraps correctly classified as test-only)

## Top Unwrap Offenders (tree-sitter-perl-rs)

All of these are test/benchmark infrastructure (NOT production):

| File | Unwraps | Status |
|------|---------|--------|
| test_slash.rs | 284 | Already gated with `#[cfg(test)]` in lib.rs |
| pure_rust_parser.rs | 74 | Behind `#[cfg(feature = "pure-rust")]` |
| sexp_formatter.rs | 67 | Behind `#[cfg(feature = "pure-rust")]` |
| integration_highlight.rs | 38 | Orphaned (not declared in lib.rs) |
| integration_corpus.rs | 34 | Orphaned (not declared in lib.rs) |

## Orphaned Test Files (Not Compiled)

These files have `#[cfg(test)]` internally but are not declared in lib.rs:

1. **integration_corpus.rs** - 34 unwraps
2. **integration_highlight.rs** - 38 unwraps
3. **test_harness.rs** - utilities for orphaned tests
4. **test_multiple_heredocs_debug.rs** - debug utilities

These files are not compiled into any build and do not affect production.

## Architecture Verification

### Workspace Structure (Cargo.toml)
```toml
[workspace]
members = [
    "crates/perl-lexer",
    "crates/perl-parser",
    "crates/perl-corpus",
    "crates/perl-lsp",
    "crates/perl-dap",
    "xtask",
]
exclude = [
    "tree-sitter-perl",
    "tree-sitter-perl-c",
    "crates/tree-sitter-perl-rs",  # ← NOT IN PRODUCTION
    "fuzz",
    "archive",
]
```

### Dependency Check
- **perl-lsp/Cargo.toml**: No tree-sitter-perl-rs dependency
- **perl-parser/Cargo.toml**: No tree-sitter-perl-rs dependency

## Fix Applied

Updated `ci/check_unwraps_prod.sh` to exclude tree-sitter-perl-rs:

```bash
# Skip tree-sitter-perl-rs (excluded from workspace, not in production)
if [[ "$dir" == "crates/tree-sitter-perl-rs/src" ]]; then
  continue
fi
```

## Updated Baseline

- **Old baseline**: 1285 (incorrect)
- **New baseline**: 518 (correct)
- **File**: ci/unwrap_prod_baseline.txt

## Recommendations

### No Action Needed for tree-sitter-perl-rs
- All unwraps are in test/benchmark infrastructure
- Already properly gated behind `#[cfg(test)]` or `#[cfg(feature = "pure-rust")]`
- Not compiled into production perl-lsp binary

### Focus on Production Crates

Top actual production unwrap offenders:

| File | Unwraps | Action Priority |
|------|---------|-----------------|
| perl-parser/src/semantic.rs | 42 | High - core parser |
| perl-parser/src/execute_command.rs | 40 | High - LSP commands |
| perl-parser/src/incremental_v2.rs | 32 | Medium - incremental parsing |
| perl-parser/src/debug_adapter.rs | 29 | Low - debugging utilities |
| perl-parser/src/parser.rs | 25 | High - core parser |

### Pattern Identification

Common production unwrap patterns to address:

1. **Lock poisoning**: `.expect("lock poisoned")` (RwLock/Mutex)
2. **Hardcoded regexes**: `.expect("hardcoded regex should compile")`
3. **Non-empty assertions**: `.expect("name is not empty")`

## Verification

```bash
# Verify production unwrap count (excluding tree-sitter-perl-rs)
bash ci/check_unwraps_prod.sh

# Expected output:
# Production unwrap/expect count: 518 (baseline: 518)
# ✅ PASS: unwrap count maintained at baseline
```

## Conclusion

The investigation confirms that tree-sitter-perl-rs unwraps are test infrastructure only and should NOT be counted in production metrics. The unwrap check script has been corrected to reflect the actual workspace architecture, reducing false positives by 59%.
