# Issue: Ambiguity Resolution Tests for Perl Corpus

**Status**: Open  
**Priority**: P2  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks specific tests for ambiguous syntax edge cases where the parser must make deterministic choices between multiple valid interpretations. Perl has many ambiguous constructs that can be parsed in multiple ways, and without explicit tests, we cannot ensure the parser handles these correctly.

Without ambiguity resolution tests, we cannot ensure:
1. **Deterministic parsing** - Parser consistently resolves ambiguities
2. **Edge case coverage** - Corner cases in ambiguous constructs are tested
3. **Parser correctness** - Ambiguities are resolved according to Perl spec
4. **LSP accuracy** - LSP provides correct symbols and diagnostics for ambiguous code
5. **Regression prevention** - Changes to parser don't break ambiguity resolution

## Impact Assessment

**Why This Matters:**

1. **Parser Correctness** - Ambiguity is a common source of parser bugs
2. **User Experience** - Ambiguous code should provide consistent LSP experience
3. **Spec Compliance** - Parser should follow Perl 5 specification for ambiguous cases
4. **Code Quality** - Ambiguity resolution affects code quality perception
5. **Testing Confidence** - Without ambiguity tests, we can't validate parser robustness

**Current State:**
- No dedicated ambiguity test files
- Ambiguous constructs may not be explicitly tested
- Parser may handle ambiguities incorrectly
- No validation against Perl 5 specification
- LSP behavior on ambiguous code is untested

## Current State

**What's Missing:**

1. **Ambiguity test files** - Tests for Perl's ambiguous syntax constructs
2. **Edge case tests** - Tests for edge cases in ambiguous constructs
3. **Spec compliance tests** - Tests validating parser follows Perl 5 specification
4. **LSP ambiguity tests** - Tests for LSP behavior on ambiguous code
5. **Parser decision tests** - Tests for parser's deterministic choices

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) has some tests but no dedicated ambiguity tests
- [`crates/perl-parser/tests/`](../crates/perl-parser/tests/) has general parser tests
- No corpus-specific ambiguity test infrastructure

## Recommended Path Forward

### Phase 1: Identify Ambiguous Perl Constructs

**Objective**: Catalog ambiguous Perl syntax constructs

**Steps:**
1. Research Perl 5 ambiguity specification:
   - Read perldoc documentation on ambiguous constructs
   - Consult perlfunc reference
   - Review Perl 5.36+ release notes
2. Identify ambiguous syntax areas:
   - Shift vs. relational operators
   - List vs. array context
   - Subroutine vs. method vs. bareword
   - Package separator (::) vs. indirect object syntax
   - Regex modifier interactions
   - Prototype/attribute interactions
   - Quote-like operator edge cases
   - Heredoc delimiter edge cases
   - Eval context edge cases
3. Document each ambiguous construct with:
   - Example code showing ambiguity
   - Perl 5 specification resolution
   - Expected parser behavior
   - LSP expected behavior
4. Create ambiguity taxonomy document

**Deliverable**: `docs/perl5_ambiguity_taxonomy.md`

### Phase 2: Create Ambiguity Test Files

**Objective**: Implement tests for each ambiguous construct

**Steps:**
1. Create `test_corpus/ambiguity/` directory
2. For each ambiguous construct, create test file:
   ```perl
   # Ambiguity: Shift vs. relational operator
   # @tags: ambiguity, syntax, shift, relational
   # @perl: 5.10+
   
   # Test 1: Shift operator in scalar context
   my ($a) = shift @array;
   say "Shift in scalar: $a";  # Should be shift
   
   # Test 2: Shift operator in list context
   my (@list) = (1, 2, 3);
   my ($b) = shift @list;
   say "Shift in list: $b";  # Should be shift
   
   # Test 3: Relational operator in scalar context
   my ($c) = 1 < 2;
   say "Relational in scalar: $c";  # Should be 1
   
   # Test 4: Relational operator in list context
   my ($d) = 1 < 2;
   say "Relational in list: $d";  # Should be 1
   
   __END__
   
   # Parser assertions:
   # 1. Scalar shift should be recognized as shift
   # 2. List shift should be recognized as shift
   # 3. Scalar relational should be recognized as relational
   # 4. List relational should be recognized as relational
   ```
3. Add expected behavior comments for each test
4. Validate parser handles each case correctly

**Deliverable**: 15-20 ambiguity test files

### Phase 3: Add LSP Ambiguity Tests

**Objective**: Test LSP behavior on ambiguous code

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with ambiguity test suite
2. Implement LSP tests for ambiguous scenarios:
   - `test_lsp_ambiguity_shift_vs_relational()` - Test symbol resolution
   - `test_lsp_ambiguity_list_context()` - Test symbol resolution
   - `test_lsp_ambiguity_package_separator()` - Test package resolution
   - `test_lsp_ambiguity_indirect_object()` - Test method resolution
   - `test_lsp_ambiguity_diagnostics()` - Test diagnostic accuracy
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - Correct symbol type inferred
   - Correct symbol location
   - No false positives in diagnostics
   - Correct completion suggestions

**Deliverable**: LSP ambiguity test suite

### Phase 4: Add Edge Case Tests

**Objective**: Test edge cases in ambiguous constructs

**Steps:**
1. Identify edge cases for each ambiguous construct:
   - Empty list/array context
   - Nested ambiguous constructs
   - Multiple ambiguities in same expression
   - Ambiguity with pragmas/versions
2. Create edge case test files:
   ```perl
   # Edge case: Empty list context with shift
   # @tags: ambiguity, edge, shift, empty
   
   my @empty_list = ();
   my ($result) = shift @empty_list;
   # Should handle empty list gracefully
   ```
3. Validate parser doesn't crash on edge cases
4. Document expected edge case behavior

**Deliverable**: Edge case test files

### Phase 5: Add Spec Compliance Tests

**Objective**: Validate parser follows Perl 5 specification

**Steps:**
1. For each ambiguous construct, create spec compliance test:
   ```perl
   # Spec compliance: Package separator (::) vs indirect object
   # @tags: ambiguity, spec, package, indirect_object
   # @perl: 5.00+
   
   # Test 1: Package separator syntax
   package MyClass {
       sub new_method { }
       package OtherClass;
       
       # Indirect object syntax
       my $obj = OtherClass->new();
       $obj->method();
   }
   ```
2. Validate parser follows spec for:
   - Package separator resolution
   - Indirect object resolution
   - Subroutine vs. method distinction
3. Document spec reference for each test
4. Add tests for spec changes (e.g., Perl 5.36+ signatures)

**Deliverable**: Spec compliance test suite

## Priority Level

**P2 - Medium Priority**

This is a P2 issue because:
1. Important for parser correctness but not blocking
2. Can be addressed incrementally
3. Existing test infrastructure provides foundation
4. Lower risk than syntax coverage gaps
5. Many ambiguous constructs are already handled correctly

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Identify Ambiguous Constructs): 2-3 days
- Phase 2 (Create Ambiguity Tests): 4-6 days
- Phase 3 (Add LSP Ambiguity Tests): 3-4 days
- Phase 4 (Add Edge Case Tests): 2-3 days
- Phase 5 (Add Spec Compliance Tests): 3-4 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Error Recovery Tests](corpus-coverage-error-recovery.md) - Related robustness testing

## References

- [perldoc - perlfunc](https://perldoc.perl.org/perlfunc) - Perl function reference
- [Perl 5.36+ Release Notes](https://metacpan.org/pod/perl5260delta) - Modern Perl changes
- [Perl 5.005.028 Release Notes](https://metacpan.org/pod/perl5.005.028) - Signatures changes
- [Perl 5.010.000 Release Notes](https://metacpan.org/pod/perl5.010.000) - try/catch changes

## Success Criteria

1. Ambiguity taxonomy documented with 15-20 constructs
2. 15-20 ambiguity test files created and validated
3. LSP ambiguity tests implemented and passing
4. Edge case tests implemented
5. Spec compliance tests implemented
6. No ambiguity-related bugs in existing features
7. Parser consistently resolves ambiguities per Perl 5 spec
8. LSP provides accurate symbols and diagnostics for ambiguous code

## Open Questions

1. Which ambiguous constructs are highest priority?
2. Should ambiguity tests be added to main corpus or separate directory?
3. How should parser handle ambiguous constructs that don't have a clear spec resolution?
4. Should there be version-specific ambiguity tests (e.g., Perl 5.36+ signatures)?
5. What LSP behavior is expected for ambiguous code that doesn't follow best practices?
