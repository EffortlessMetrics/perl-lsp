# Ignored Tests Roadmap

> **Issue**: #144 - Systematic Resolution of Ignored Tests
> **Target**: Reduce ignored tests from 41 (baseline) to 25 or fewer (49% reduction)
> **Last Updated**: 2025-12-31

## Executive Summary

| Metric | Value |
|--------|-------|
| **Baseline** | 41 ignored tests (stored in `ci/ignored_baseline.txt`) |
| **Target** | <=25 ignored tests |
| **Required Reduction** | 16+ tests (49% target achievement) |
| **Current Status** | 5 hard `#[ignore]` + 21 `cfg_attr` feature-gated = 26 total |
| **Progress** | Wave A complete, Wave B complete (see below) |

### Current Breakdown

| Category | Count | Status |
|----------|-------|--------|
| **Hard `#[ignore]`** | 5 | Active ignores needing resolution |
| **Feature-gated (`cfg_attr`)** | 21 | By design, run with features |
| **MANUAL** | 1 | `lsp_capabilities_snapshot.rs` - keep |
| **BUG** | 3 | Parser limitations documented |
| **Other** | 1 | Parsing strictness test |

### Hard `#[ignore]` Tests (5 total)

| File | Test | Category |
|------|------|----------|
| `lsp_capabilities_snapshot.rs:62` | `regenerate_snapshots` | MANUAL |
| `substitution_ac_tests.rs:268` | `test_ac5_negative_malformed` | BUG |
| `prop_whitespace_idempotence.rs:38` | `insertion_safe_is_consistent` | BUG |
| `comprehensive_operator_precedence_test.rs:122` | `test_complex_precedence_combinations` | BUG |
| `parser_regressions.rs:85` | `print_filehandle_then_variable_is_indirect` | BUG |

---

## Wave A: Test Brittleness Issues - COMPLETE

**Status**: 2 tests FIXED
**Type**: Test expectations were incorrect, not parser bugs

### A.1: `test_word_boundary_qwerty_not_matched` - FIXED

**File**: `crates/perl-parser/tests/declaration_micro_tests.rs`

**Problem**: Test used hardcoded byte offset that didn't account for sigil in span.

**Resolution**: Changed to dynamic position computation.

**Commit**: `#[ignore]` removed, test now runs as part of regular suite.

---

### A.2: `test_comment_with_qw_in_it` - FIXED

**File**: `crates/perl-parser/tests/declaration_micro_tests.rs`

**Problem**: Test used hardcoded byte offset `36` (pointed to a space, not the variable).

**Resolution**: Changed to dynamic position computation using `code.rfind("$var")`.

**Commit**: `#[ignore]` removed, test now runs as part of regular suite.

---

## Wave B: Substitution Operator Tests - COMPLETE

**Status**: 4/4 tests FIXED
**Type**: Parser bugs in substitution operator handling
**Completed**: These tests have been fixed and `#[ignore]` removed

### B.1: Empty Replacement with Balanced Delimiters (MUT_002) - FIXED

**Files**:
- `crates/perl-parser/tests/substitution_operator_tests.rs:193`
- `crates/perl-parser/tests/substitution_ac_tests.rs:90`

**Test Names**:
- `test_substitution_empty_replacement_balanced_delimiters`
- `test_ac2_empty_replacement_balanced_delimiters`

**Resolution**: Fixed in `quote_parser.rs` - balanced delimiters now use per-segment delimiter detection.

**Validation**:
```bash
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_empty_replacement_balanced_delimiters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_empty_replacement_balanced_delimiters --nocapture
```

---

### B.2: Invalid Modifier Validation (MUT_005) - FIXED

**Files**:
- `crates/perl-parser/tests/substitution_operator_tests.rs:351`
- `crates/perl-parser/tests/substitution_ac_tests.rs:67`

**Test Names**:
- `test_substitution_invalid_modifier_characters`
- `test_ac2_invalid_flag_combinations`

**Resolution**: Invalid modifier validation now properly rejects invalid modifiers. Only `g`, `i`, `m`, `s`, `x`, `o`, `e`, `r` are allowed.

**Validation**:
```bash
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_invalid_modifier_characters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_invalid_flag_combinations --nocapture
```

---

## Wave C: Parser Limitations - TODO

**Status**: 0/4 tests fixed
**Type**: Known parser limitations requiring deeper refactoring
**Priority**: MEDIUM - Edge cases
**Documentation**: See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)

### C.1: Return After `or` Precedence

**File**: `crates/perl-parser/tests/comprehensive_operator_precedence_test.rs:122`

**Test Name**: `test_complex_precedence_combinations`

**Ignore Reason**: `"BUG: 'return' after 'or' needs deeper parser refactoring - return as expression"`

**Issue**: `$a = 1 or return` doesn't parse correctly

**Root Cause**: `return` needs to be treated as an expression in this context, not a statement.

**Fix Strategy**:
1. Investigate operator precedence table for word operators
2. Consider treating `return` as expression when following `or`/`and`
3. May require parser architecture changes

**Complexity**: HIGH - Affects operator precedence fundamentals

**Validation**:
```bash
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture
```

---

### C.2: Indirect Object Detection

**File**: `crates/perl-parser/tests/parser_regressions.rs:85`

**Test Name**: `print_filehandle_then_variable_is_indirect`

**Ignore Reason**: `"BUG: Indirect object detection requires deeper parser refactoring"`

**Issue**: `print $fh $x;` not treated as indirect object form

**Root Cause**: Ambiguous syntax requires semantic analysis to disambiguate.

**Policy Decision Required**:
- `print $fh $x;` - indirect object OR `print($fh, $x)`?
- `new Class $arg;` - indirect object OR function call?

**Fix Strategy**:
1. Define disambiguation rules
2. Implement heuristics for common patterns (print/say/new/open)
3. Document edge cases in KNOWN_LIMITATIONS.md

**Complexity**: HIGH - Perl's ambiguous syntax

**Validation**:
```bash
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture
```

---

### C.3: Insertion Safe Algorithm

**File**: `crates/perl-parser/tests/prop_whitespace_idempotence.rs:38`

**Test Name**: `insertion_safe_is_consistent`

**Ignore Reason**: `"insertion_safe algorithm has known inconsistencies"`

**Issue**: `insertion_safe` function produces non-deterministic results.

**Root Cause**: Likely iteration order dependency (HashMap iteration without sorting).

**Fix Strategy**:
1. Make function deterministic (define ordering for iteration)
2. Avoid HashMap iteration without sorting
3. Add normalization step before comparison

**Complexity**: MEDIUM

**Validation**:
```bash
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture
```

---

### C.4: Malformed Substitution Operators

**File**: `crates/perl-parser/tests/substitution_ac_tests.rs:268`

**Test Name**: `test_ac5_negative_malformed`

**Ignore Reason**: `"Exposes parsing strictness issues - will kill various mutants when parsing is hardened"`

**Issue**: Malformed substitution operators don't reliably error.

**Test Cases**:
```perl
s/pattern/           # Missing replacement and closing delimiter
s/pattern            # Missing replacement delimiter and replacement
s/pattern/replacement # Missing closing delimiter
s                    # Just the 's' keyword
s/                   # Just 's' and opening delimiter
s//                  # Missing replacement
s/pattern/replacement/invalid_flag  # Invalid flag
```

**Fix Strategy**:
1. Define expected behavior: parse must error, OR parse succeeds but `has_error()` returns true
2. Fix parser error propagation
3. Update test acceptance condition

**Complexity**: MEDIUM

**Validation**:
```bash
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --nocapture
```

---

## Wave D: Feature-Gated Tests - BY DESIGN

**Status**: Not applicable for Issue #144 target

These 21 tests use `#[cfg_attr(not(feature = "..."), ignore)]` and are **intentionally gated** behind feature flags. They run with specific features enabled and should NOT be counted toward the reduction target.

### Feature-Gated Tests (21 total)

| File | Feature | Count | Purpose |
|------|---------|-------|---------|
| `lsp_cancellation_parser_integration_tests.rs` | `stress-tests` | 4 | Long-running stress tests |
| `declaration_unit_tests.rs` | `package-qualified` | 2 | Package-qualified parsing |
| `error_classifier_tests.rs` | `error-classifier-v2` | 6 | Error classifier v2 features |
| `declaration_edge_cases_tests.rs` | `constant-advanced` | 6 | Advanced constant handling |
| `declaration_edge_cases_tests.rs` | `qw-variants` | 3 | qw() variant parsing |

**Run feature-gated tests**:
```bash
# Stress tests (long-running)
cargo test -p perl-lsp --features stress-tests

# Parser advanced features
cargo test -p perl-parser --features "package-qualified,error-classifier-v2,constant-advanced,qw-variants"

# All features at once
cargo test -p perl-parser --all-features
```

**Note**: Feature-gated tests are a best practice for:
- Stress tests that are too slow for regular CI
- Features under development
- Optional functionality that requires extra dependencies

---

## Manual Helper Tests - KEEP IGNORED

These are utilities, not validation tests:

### `regenerate_snapshots`

**File**: `crates/perl-lsp/tests/lsp_capabilities_snapshot.rs:62`

**Reason**: `"MANUAL: Regenerate with: cargo test -p perl-lsp --test lsp_capabilities_snapshot regenerate -- --ignored"`

**Status**: Keep ignored - snapshot regeneration utility

**Run when needed**:
```bash
cargo test -p perl-lsp --test lsp_capabilities_snapshot regenerate -- --ignored
```

---

## Progress Tracking

### Summary Table

| Wave | Tests | Fixed | Remaining | Priority |
|------|-------|-------|-----------|----------|
| A: Brittleness | 2 | 2 | 0 | COMPLETE |
| B: Substitution | 4 | 4 | 0 | COMPLETE |
| C: Limitations | 4 | 0 | 4 | MEDIUM |
| D: Audit | 0 | - | - | N/A |
| **Total Hard Ignores** | **5** | - | **5** | - |

### Current Hard Ignores

| # | Test | Wave | Status |
|---|------|------|--------|
| 1 | `regenerate_snapshots` | MANUAL | Keep ignored |
| 2 | `test_ac5_negative_malformed` | C | TODO |
| 3 | `insertion_safe_is_consistent` | C | TODO |
| 4 | `test_complex_precedence_combinations` | C | TODO |
| 5 | `print_filehandle_then_variable_is_indirect` | C | TODO |

### Commands

```bash
# Count current ignored tests
scripts/ignored-test-count.sh

# Verbose with categories
VERBOSE=1 scripts/ignored-test-count.sh

# Update baseline after fixing tests
scripts/ignored-test-count.sh --update

# CI gate (fails if increased)
scripts/ignored-test-count.sh --check

# Run all Wave C tests (ignored)
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- --ignored --nocapture
cargo test -p perl-parser --test parser_regressions -- --ignored --nocapture
cargo test -p perl-parser --test prop_whitespace_idempotence -- --ignored --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --ignored --nocapture
```

---

## Definition of Done

For each test fix:

1. Remove `#[ignore]` annotation
2. Verify test passes: `cargo test -p <crate> --test <file> -- <test_name> --nocapture`
3. Run full test suite: `cargo test`
4. Update baseline: `scripts/ignored-test-count.sh --update`
5. Verify gate passes: `scripts/gate-local.sh`

### Target Achievement

- [x] Wave A complete (2/2 tests fixed)
- [x] Wave B complete (4/4 tests fixed)
- [ ] Wave C complete (0/4 tests fixed)
- [ ] Final count <=25 (currently at 5 hard ignores, 21 feature-gated)

---

## Related Documentation

- [BUG_EXECUTION_CHECKLIST.md](BUG_EXECUTION_CHECKLIST.md) - Detailed bug fix procedures
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) - Parser limitations by version
- [TEST_FEATURES.md](TEST_FEATURES.md) - Feature gate documentation
- [ci/IGNORED_TESTS_INDEX.md](ci/IGNORED_TESTS_INDEX.md) - Full index of all ignored tests
- [IGNORED_TESTS_IMPLEMENTATION_ROADMAP.md](IGNORED_TESTS_IMPLEMENTATION_ROADMAP.md) - Implementation priorities
