# Resolution Plan: P0 Ambiguous Slash Division vs Regex

## Issue Summary

The slash `/` character has dual meaning in Perl, creating a fundamental parsing ambiguity:

- **Division operator**: `$a / $b`
- **Regex delimiter**: `/pattern/`

This ambiguity requires context-aware tokenization to correctly interpret the slash character.

## Current State

### Implemented Protections

| Feature | Status | Location |
|---------|--------|----------|
| Mode-aware lexer | ✅ Implemented | [`crates/perl-lexer/src/mode.rs`](../../../crates/perl-lexer/src/mode.rs) |
| Slash disambiguation | ✅ Implemented | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |
| Budget guards | ✅ Implemented | `MAX_REGEX_BYTES = 64KB` |
| Test coverage | ✅ 21 test cases | [`crates/perl-lexer/tests/lexer_slash_timeout_tests.rs`](../../../crates/perl-lexer/tests/lexer_slash_timeout_tests.rs) |

### LexerMode Tracking System

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexerMode {
    ExpectTerm,     // Slash starts a regex
    ExpectOperator, // Slash is division
}
```

## Gap Analysis

### Identified Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| Edge case coverage | Low | Unusual Perl idioms may not parse correctly |
| Performance validation | Low | Need to verify no hang on pathological input |
| Fuzz/property testing | Low | Randomized slash-heavy inputs would broaden regression coverage |

### Test Coverage Status

- [x] Simple division: `$a / $b`
- [x] Simple regex: `/pattern/`
- [x] Chained division: `$a / $b / $c`
- [x] Regex with division-like content: `/\//`
- [x] Match operator: `$x =~ /pat/`
- [x] Substitution: `$x =~ s/old/new/`
- [x] Implicit match: `print if /pat/`
- [x] Division after function call: `func() / 2`
- [x] Edge case: `time / 86400` vs `time /pattern/`
- [x] Defined-or: `$x // $y`
- [x] Division assignment: `$x /= 2`
- [x] Empty regex: `$x =~ //`
- [x] Performance: no hang on pathological input

## Proposed Solution

### Status: ✅ RESOLVED

The ambiguous slash issue is **fully implemented** with comprehensive test coverage. The mode-aware lexer correctly handles all documented cases.

### Remaining Tasks

1. **Add Fuzz Testing**
   - Add property-based tests for slash disambiguation
   - Test with randomly generated Perl code containing slashes

## Test Plan

### Existing Tests

| Test File | Coverage |
|-----------|----------|
| [`lexer_slash_timeout_tests.rs`](../../../crates/perl-lexer/tests/lexer_slash_timeout_tests.rs) | 21 test cases for slash disambiguation |
| [`hang_risk_slash_ambiguity_tests.rs`](../../../crates/perl-lexer/tests/hang_risk_slash_ambiguity_tests.rs) | Comprehensive slash ambiguity tests |
| [`comprehensive_unit_tests.rs`](../../../crates/perl-lexer/tests/comprehensive_unit_tests.rs) | Context-sensitive slash disambiguation |

### Validation Steps

1. Run existing test suite: `cargo test -p perl-lexer --test lexer_slash_timeout_tests`
2. Run hang risk tests: `cargo test -p perl-lexer --test hang_risk_slash_ambiguity_tests`
3. Verify performance: `cargo test -p perl-lexer -- --test-threads=1 lexer_slash`

## Dependencies

None - implementation is complete.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Edge case failure | Low | Medium | Comprehensive test coverage |
| Performance regression | Low | Low | Performance tests in place |
| Incorrect disambiguation | Low | Medium | Mode tracking validation |

## Conclusion

**Status: COMPLETE** - No further implementation required. The mode-aware lexer provides comprehensive protection against ambiguous slash parsing issues. The remaining work is optional additional regression hardening.

## References

- [Issue Documentation](../corpus/gaps/timeout-hang-risks/ambiguous-slash-division-regex.md)
- [Lexer Mode Implementation](../../../crates/perl-lexer/src/mode.rs)
- [Slash Disambiguation Logic](../../../crates/perl-lexer/src/lib.rs)
