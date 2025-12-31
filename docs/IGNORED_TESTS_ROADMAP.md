# Ignored Tests Roadmap

> **Issue**: #144 - Systematic Resolution of Ignored Tests
> **Target**: Reduce ignored tests from 41 (baseline) to 25 or fewer (49% reduction)
> **Last Updated**: 2025-12-31

## Executive Summary

| Metric | Value |
|--------|-------|
| **Baseline** | Tracked via `scripts/.ignored-baseline` |
| **Target** | <=25 ignored tests |
| **Current Status** | **BUG=0**, MANUAL=1, total hard ignores=1 ✅ |
| **Progress** | Waves A, B, C complete (see below) |

### Current Breakdown

| Category | Count | Status |
|----------|-------|--------|
| **Hard `#[ignore]`** | 1 | Only MANUAL utility test |
| **Feature-gated (`cfg_attr`)** | 21 | By design, run with features |
| **MANUAL** | 1 | `lsp_capabilities_snapshot.rs` - keep |
| **BUG** | 0 | All fixed in PR #261 ✅ |

### Hard `#[ignore]` Tests (1 total)

| File | Test | Category |
|------|------|----------|
| `lsp_capabilities_snapshot.rs:62` | `regenerate_snapshots` | MANUAL |

**Track current count:**
```bash
bash scripts/ignored-test-count.sh
```

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

## Wave C: Parser Limitations - COMPLETE ✅

**Status**: 4/4 tests FIXED (PR #261)
**Type**: Parser robustness improvements addressing edge cases
**Completed**: 2025-12-31

### C.1: Return After `or` Precedence - FIXED ✅

**File**: `crates/perl-parser/tests/comprehensive_operator_precedence_test.rs:122`
**Test Name**: `test_complex_precedence_combinations`
**Resolution**: Fixed in PR #261 - improved operator precedence handling

**Validation**:
```bash
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture
```

---

### C.2: Indirect Object Detection - FIXED ✅

**File**: `crates/perl-parser/tests/parser_regressions.rs:85`
**Test Name**: `print_filehandle_then_variable_is_indirect`
**Resolution**: Fixed in PR #261 - improved indirect object heuristics

**Validation**:
```bash
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture
```

---

### C.3: Insertion Safe Algorithm - FIXED ✅

**File**: `crates/perl-parser/tests/prop_whitespace_idempotence.rs:38`
**Test Name**: `insertion_safe_is_consistent`
**Resolution**: Fixed in PR #261 - made algorithm deterministic

**Validation**:
```bash
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture
```

---

### C.4: Malformed Substitution Operators - FIXED ✅

**File**: `crates/perl-parser/tests/substitution_ac_tests.rs:268`
**Test Name**: `test_ac5_negative_malformed`
**Resolution**: Fixed in PR #261 - improved substitution validation

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
| A: Brittleness | 2 | 2 | 0 | COMPLETE ✅ |
| B: Substitution | 4 | 4 | 0 | COMPLETE ✅ |
| C: Limitations | 4 | 4 | 0 | COMPLETE ✅ |
| D: Feature-Gated | 21 | - | - | BY DESIGN |
| **Total Hard Ignores** | **1** | - | **1** | MANUAL utility only |

### Current Hard Ignores

| # | Test | Wave | Status |
|---|------|------|--------|
| 1 | `regenerate_snapshots` | MANUAL | Keep ignored (utility) |

### Commands

```bash
# Count current ignored tests
bash scripts/ignored-test-count.sh

# Verbose with categories
VERBOSE=1 bash scripts/ignored-test-count.sh

# Update baseline after fixing tests
bash scripts/ignored-test-count.sh --update

# CI gate (fails if increased)
bash scripts/ignored-test-count.sh --check
```

---

## Definition of Done

For each test fix:

1. Remove `#[ignore]` annotation
2. Verify test passes: `cargo test -p <crate> --test <file> -- <test_name> --nocapture`
3. Run full test suite: `cargo test`
4. Update baseline: `scripts/ignored-test-count.sh --update`
5. Verify gate passes: `scripts/gate-local.sh`

### Target Achievement ✅

- [x] Wave A complete (2/2 tests fixed)
- [x] Wave B complete (4/4 tests fixed)
- [x] Wave C complete (4/4 tests fixed) - PR #261
- [x] Final count <=25 ✅ (1 hard ignore MANUAL + 21 feature-gated)

---

## Related Documentation

- [BUG_EXECUTION_CHECKLIST.md](BUG_EXECUTION_CHECKLIST.md) - Detailed bug fix procedures
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) - Parser limitations by version
- [TEST_FEATURES.md](TEST_FEATURES.md) - Feature gate documentation
- [ci/IGNORED_TESTS_INDEX.md](ci/IGNORED_TESTS_INDEX.md) - Full index of all ignored tests
- [IGNORED_TESTS_IMPLEMENTATION_ROADMAP.md](IGNORED_TESTS_IMPLEMENTATION_ROADMAP.md) - Implementation priorities
