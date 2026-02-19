# Bug Execution Checklist (BUG=8)

**Generated**: 2025-12-31
**Updated**: 2025-12-31
**Target**: Zero bugs remaining before GA release

**Current Status**: Wave A complete (test brittleness, not parser bugs); Wave B/C remain

---

## Wave A: Lexer Boundary Bugs (Fast, Low Risk) ✅ COMPLETE

These were test brittleness issues, not actual parser bugs.

### Bug 1: `test_word_boundary_qwerty_not_matched` ✅ FIXED

**File**: `crates/perl-parser/tests/declaration_micro_tests.rs`
**Resolution**: Test expectation was incorrect. The parser correctly includes sigil in span.
- Changed from hardcoded offset expectation to dynamic position computation
- `#[ignore]` removed, test passes as part of regular suite

---

### Bug 2: `test_comment_with_qw_in_it` ✅ FIXED

**File**: `crates/perl-parser/tests/declaration_micro_tests.rs`
**Resolution**: Test used hardcoded byte offset `36` (a space!), not the actual variable position.
- Changed to dynamic position computation using `code.rfind("$var")`
- `#[ignore]` removed, test passes as part of regular suite

---

## Wave B: Substitution Operator Bugs (High ROI)

These mutations improve real-world correctness. Two pairs of related bugs.

### Bug 3-4: `test_substitution_empty_replacement_balanced_delimiters` (MUT_002)

**Files**:
- `crates/perl-parser/tests/substitution_operator_tests.rs:198`
- `crates/perl-parser/tests/substitution_ac_tests.rs:91`

**Issue**: Empty replacement with balanced delimiters fails: `s{pattern}{}`
**Likely Cause**: Replacement parser rejects empty string between delimiters

```bash
# Run the tests
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_empty_replacement_balanced_delimiters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_empty_replacement_balanced_delimiters --nocapture

# Fix location: quote_parser.rs around line 80
# Fix: Allow zero-length replacement; ensure closing delimiter detection works for empty content
```

**Definition of Done**: Remove `#[ignore]` from both, tests pass, gate passes

---

### Bug 5-6: `test_substitution_invalid_modifier_characters` (MUT_005)

**Files**:
- `crates/perl-parser/tests/substitution_operator_tests.rs:360`
- `crates/perl-parser/tests/substitution_ac_tests.rs:66`

**Issue**: Parser accepts invalid modifiers (e.g., `s/foo/bar/z`)
**Likely Cause**: Modifier scanning accepts arbitrary letters instead of allowlist

```bash
# Run the tests
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_invalid_modifier_characters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_invalid_flag_combinations --nocapture

# Fix location: parser_backup.rs around line 4231 (or wherever modifiers are parsed)
# Fix: Validate modifiers against allowlist: g, i, m, s, x, o, e, r
```

**Definition of Done**: Remove `#[ignore]` from both, tests pass, gate passes

---

## Wave C: Harder Semantics (Do After A & B)

### Bug 7: `test_ac5_negative_malformed`

**File**: `crates/perl-parser/tests/substitution_ac_tests.rs:270`
**Issue**: Malformed substitution operators don't reliably error

```bash
# Run the test
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --nocapture

# Fix: Decide what "malformed" means:
#   - parse must error, OR
#   - parse succeeds but has_error() must be true
# Then fix parser error propagation or test's acceptance condition
```

**Definition of Done**: Remove `#[ignore]`, test passes, gate passes

---

### Bug 8: `insertion_safe_is_consistent`

**File**: `crates/perl-parser/tests/prop_whitespace_idempotence.rs:38`
**Issue**: `insertion_safe` algorithm produces non-deterministic results

```bash
# Run the test
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture

# Fix: Make function total + deterministic:
#   - Define ordering for iteration
#   - Avoid HashMap iteration without sorting
#   - Add normalization step before compare
```

**Definition of Done**: Remove `#[ignore]`, test passes, gate passes

---

### Bug 9: `test_complex_precedence_combinations`

**File**: `crates/perl-parser/tests/comprehensive_operator_precedence_test.rs:122`
**Issue**: Non-critical precedence edge case
**Likely Cause**: One operator precedence level wrong, or specific combination short-circuits

```bash
# Run the test
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture

# Fix: Surgical fix to precedence table or associativity
# Look for: word operators (and/or/not) vs assignment precedence
```

**Definition of Done**: Remove `#[ignore]`, test passes, gate passes

---

### Bug 10: `print_filehandle_then_variable_is_indirect`

**File**: `crates/perl-parser/tests/parser_regressions.rs:85`
**Issue**: `print $fh $x;` not treated as indirect object form
**Requires**: Policy decision on ambiguous Perl syntax

```bash
# Run the test
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture

# Before fixing, establish truth table:
#   - print $fh $x;      → indirect object OR print($fh, $x)?
#   - new Class $arg;    → indirect object OR function call?
# Then implement disambiguation rule
```

**Definition of Done**: Remove `#[ignore]`, test passes, gate passes

---

## Quick Reference: All Bug Commands

```bash
# Wave A
cargo test -p perl-parser --test declaration_micro_tests -- test_word_boundary_qwerty_not_matched --nocapture
cargo test -p perl-parser --test declaration_micro_tests -- test_comment_with_qw_in_it --nocapture

# Wave B (MUT_002 pair)
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_empty_replacement_balanced_delimiters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_empty_replacement_balanced_delimiters --nocapture

# Wave B (MUT_005 pair)
cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_invalid_modifier_characters --nocapture
cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_invalid_flag_combinations --nocapture

# Wave C
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --nocapture
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture
```

---

## Post-Bug Cleanup

After all bugs are fixed:

```bash
# Update baseline (removes BUG category from baseline)
scripts/ignored-test-count.sh --update

# Run gate
scripts/gate-local.sh

# Confidence check
cargo test -p perl-lsp
cargo test -p perl-parser
```

---

## Progress Tracker

| # | Bug | Wave | Status |
|---|-----|------|--------|
| 1 | test_word_boundary_qwerty_not_matched | A | ✅ FIXED |
| 2 | test_comment_with_qw_in_it | A | ✅ FIXED |
| 3 | test_substitution_empty_replacement_balanced_delimiters | B | ⬜ TODO |
| 4 | test_ac2_empty_replacement_balanced_delimiters | B | ⬜ TODO |
| 5 | test_substitution_invalid_modifier_characters | B | ⬜ TODO |
| 6 | test_ac2_invalid_flag_combinations | B | ⬜ TODO |
| 7 | test_ac5_negative_malformed | C | ⬜ TODO |
| 8 | insertion_safe_is_consistent | C | ⬜ TODO |
| 9 | test_complex_precedence_combinations | C | ⬜ TODO |
| 10 | print_filehandle_then_variable_is_indirect | C | ⬜ TODO |
