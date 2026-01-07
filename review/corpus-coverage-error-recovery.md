# Issue: Error Recovery Tests for Perl Corpus

**Status**: Open  
**Priority**: P1  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks comprehensive error recovery tests that validate how the parser handles malformed, incomplete, or invalid Perl code. While the parser should handle errors gracefully, there is limited test coverage for:

1. **Error detection accuracy** - Parser correctly identifies syntax errors
2. **Error recovery behavior** - Parser continues parsing after errors
3. **Error reporting quality** - Useful, actionable error messages
4. **Partial parsing** - Parser extracts valid AST from invalid code
5. **Edge case robustness** - Parser doesn't crash or hang on malformed input

Without error recovery tests, we cannot ensure:
- Parser is robust against malformed input
- Error messages are helpful to users
- Parser can recover and continue parsing
- LSP provides accurate diagnostics
- No crashes or hangs on edge cases

## Impact Assessment

**Why This Matters:**

1. **User Experience**: Users write invalid code; parser should help them understand errors
2. **LSP Reliability**: LSP must provide accurate diagnostics without crashing
3. **Production Robustness**: Parser must handle real-world code gracefully
4. **Security**: Malicious input shouldn't crash the parser
5. **Incremental Parsing**: Errors during incremental edits must be handled
6. **Tooling Integration**: Tools like perltidy/perlcritic need valid AST

**Current State:**
- Corpus focuses on valid Perl syntax
- Limited tests for invalid or malformed code
- No systematic error recovery test suite
- No validation of error message quality
- No tests for parser crash/panic scenarios

## Current State

**What's Missing:**

1. **Error detection tests** - Tests for parser error detection accuracy
2. **Error recovery tests** - Tests validating parser continues after errors
3. **Partial parsing tests** - Tests for AST extraction from invalid code
4. **Error message tests** - Tests for error message quality and usefulness
5. **Crash/panic tests** - Tests ensuring parser doesn't crash on bad input
6. **Incremental error tests** - Tests for error handling during incremental edits
7. **Edge case error tests** - Tests for malformed syntax edge cases
8. **LSP diagnostic tests** - Tests validating LSP diagnostic accuracy

**Existing Infrastructure:**
- [`crates/perl-parser/src/`](../crates/perl-parser/src/) has error handling
- LSP diagnostic provider exists
- No dedicated error recovery test suite
- No corpus files specifically testing error scenarios

## Recommended Path Forward

### Phase 1: Design Error Recovery Test Scenarios

**Objective**: Identify error scenarios to test

**Steps:**
1. Analyze common Perl syntax errors:
   - Missing semicolons
   - Unbalanced brackets/braces/parentheses
   - Invalid operators
   - Malformed regex patterns
   - Invalid quote-like operators
   - Undefined symbols
2. Design test scenarios for each error type:
   - Simple single errors
   - Multiple errors in same file
   - Errors in different contexts (subroutines, packages, eval)
   - Errors in complex expressions
3. Document expected behavior for each scenario:
   - Should parser detect error?
   - Should parser recover and continue?
   - What error message should be shown?
   - What AST should be produced (if any)?
4. Create test file structure for error scenarios

**Deliverable**: `docs/error_recovery_scenarios.md`

### Phase 2: Create Error Recovery Test Files

**Objective**: Implement error recovery test files

**Steps:**
1. Create `test_corpus/error_recovery/` directory
2. For each error scenario, create test file:
   ```perl
   # Error recovery: Missing semicolon
   use v5.36;
   use strict;
   
   sub process_data {
       my ($data) = @_;
       
       # Missing semicolon - should be detected
       my $result = process_data($data)  # Error expected
       
       return $result;
   }
   ```
3. Add metadata tags for error types:
   - `# @tags: error, syntax, recovery`
   - `# @perl: 5.36+`
4. Validate each test file produces expected error
5. Ensure parser doesn't crash on malformed input

**Deliverable**: 20-30 error recovery test files

### Phase 3: Add Error Message Quality Tests

**Objective**: Validate error message quality

**Steps:**
1. Define error message quality criteria:
   - Clear and actionable
   - Points to specific location
   - Suggests corrections
   - Uses correct Perl terminology
2. Create tests for error message validation
3. Add tests for common error message patterns:
   - Missing semicolon messages
   - Unbalanced bracket messages
   - Invalid operator messages
   - Undefined symbol messages
4. Validate error messages are helpful to users
5. Document error message style guide

**Deliverable**: Error message quality test suite

### Phase 4: Add Crash and Panic Tests

**Objective**: Ensure parser robustness against crashes

**Steps:**
1. Identify potential crash scenarios:
   - Extremely long strings
   - Deeply nested structures
   - Malformed regex patterns
   - Invalid Unicode sequences
   - Memory exhaustion patterns
2. Create tests for each crash scenario
3. Use fuzz testing infrastructure:
   - Leverage existing fuzz test framework
   - Add crash detection to fuzz tests
   - Ensure no panics on malformed input
4. Add memory safety tests:
   - Tests for large file handling
   - Tests for memory exhaustion scenarios
5. Document crash prevention strategies

**Deliverable**: Crash and panic prevention test suite

### Phase 5: Add Incremental Error Tests

**Objective**: Test error handling during incremental parsing

**Steps:**
1. Create incremental parsing error scenarios:
   - Error on first edit
   - Error propagation through multiple edits
   - Error recovery after undo
2. Implement incremental error tests:
   - Start with valid code
   - Apply invalid edit
   - Validate error detection
   - Validate AST state after error
   - Validate subsequent edits work correctly
3. Test LSP incremental error handling:
   - Validate LSP publishes correct diagnostics
   - Validate LSP recovers gracefully
   - Validate LSP doesn't crash on errors
4. Document incremental error recovery behavior
5. Add performance tests for error scenarios

**Deliverable**: Incremental error recovery test suite

## Priority Level

**P1 - High Priority**

This is a P1 issue because:
1. **Production Robustness** - Parser must handle real-world code gracefully
2. **LSP Reliability** - Accurate diagnostics are critical for LSP
3. **User Experience** - Helpful error messages improve developer experience
4. **Security** - Prevent crashes on malicious input
5. **Foundation** - Enables better testing of all other features
6. **Risk Mitigation** - Error handling bugs can cause data corruption

## Estimated Effort

**Total Effort**: Medium-High

- Phase 1 (Scenario Design): 2-3 days
- Phase 2 (Error Recovery Tests): 5-7 days
- Phase 3 (Error Message Quality): 2-3 days
- Phase 4 (Crash/Panic Tests): 3-4 days
- Phase 5 (Incremental Error Tests): 3-4 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Performance Benchmarks](corpus-coverage-performance-benchmarks.md) - Related corpus testing

## References

- [`crates/perl-parser/src/`](../crates/perl-parser/src/) - Parser error handling
- [LSP Implementation Guide](../docs/LSP_IMPLEMENTATION_GUIDE.md) - LSP diagnostic provider
- [Error Handling Strategy Guide](../docs/ERROR_HANDLING_STRATEGY.md) - Error handling patterns
- [Fuzz Testing Infrastructure](../crates/perl-parser/tests/fuzz_quote_parser_comprehensive.rs) - Existing fuzz tests

## Success Criteria

1. Error recovery scenarios documented and categorized
2. 20-30 error recovery test files created and validated
3. Error message quality tests implemented and passing
4. Crash and panic prevention tests implemented
5. Incremental error recovery tests implemented
6. No crashes or panics on malformed input
7. Error messages are clear, actionable, and accurate
8. LSP provides accurate diagnostics for all error scenarios

## Open Questions

1. Should error recovery tests be version-specific (e.g., Perl 5.36 only)?
2. How should parser handle code with multiple errors?
3. Should there be different error recovery strategies for LSP vs. parser?
4. What performance thresholds are acceptable for error scenarios?
