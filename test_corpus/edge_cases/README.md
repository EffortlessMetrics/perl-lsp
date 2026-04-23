# Edge Cases Test Suite

This directory contains comprehensive edge case tests for the Perl parser to ensure robustness and correctness when handling complex, ambiguous, or malformed Perl code.

## Test Files

### 1. deeply_nested_constructs.pl
Tests parser recursion limits and stack overflow protection with:
- Deeply nested if-else statements (20 levels)
- Deeply nested loops (15 levels)
- Mixed nesting types (loops within conditionals within blocks)
- Complex nested data structures
- Deeply nested function calls
- Complex nested ternary operations
- Nested grep/map/sort operations
- Complex regex with nested groups
- Deeply nested BEGIN/END blocks
- Nested try-catch blocks with eval
- Complex nested heredocs

### 2. ambiguous_syntax_scenarios.pl
Tests parser's ability to disambiguate complex syntax with:
- Slash disambiguation (division vs regex)
- Hash vs block ambiguity in various contexts
- Indirect object syntax vs method calls
- Function vs method call disambiguation
- Ambiguous parentheses and precedence
- Ambiguous dereferencing
- Ambiguous quote-like operators
- Ambiguous regex modifiers
- Ambiguous statement modifiers
- Ambiguous bareword handling
- Ambiguous prototype handling
- Ambiguous attribute syntax

### 3. unicode_encoding_edge_cases.pl
Tests parser's handling of Unicode and various encodings with:
- Unicode identifiers with complex scripts (Japanese, Arabic, Chinese, Cyrillic, Hebrew)
- Unicode in string literals and regex patterns
- Bidirectional text and combining characters
- Unicode normalization forms
- UTF-8 encoding issues and BOM handling
- Invalid UTF-8 sequences and surrogate pairs
- Unicode in various Perl constructs
- Unicode with quote-like operators
- Unicode in heredocs
- Unicode with special variables
- Unicode with pack/unpack operations
- Unicode with tr/// operations
- Unicode with sprintf/printf formats
- Unicode with sort and comparison

### 4. performance_stress_scenarios.pl
Tests parser performance with large inputs and pathological cases:
- Very large strings (100KB+)
- Massive arrays and data structures
- Deep nested structures with many levels
- Pathological regex patterns that could cause backtracking
- Simulated massive files (10K lines)
- Memory-intensive operations
- Complex nested operations
- Large heredocs (50K lines)
- Complex regex with many alternatives
- Massive symbol tables
- Complex nested conditionals
- Large string concatenations
- Complex data structure traversal
- Performance with Unicode stress
- Complex eval statements
- Large symbol table with complex names
- Memory stress with many file handles (simulated)
- Complex string operations
- Large sort operations
- Complex hash operations
- Stress with recursive functions

### 5. error_recovery_edge_cases.pl
Tests parser's ability to recover from malformed code with:
- Severely malformed statements (unmatched brackets, parentheses, quotes)
- Recovery from incomplete statements
- Unexpected tokens in various contexts
- Error recovery at different nesting levels
- Mixed syntax errors
- Recovery from garbled input
- Context-specific error recovery
- Recovery from operator precedence issues
- Recovery from prototype mismatches
- Recovery from package and namespace issues
- Recovery from malformed attributes
- Recovery from format statement errors
- Recovery from signal handler errors
- Recovery from typeglob manipulation errors
- Recovery from eval errors
- Recovery from do block errors
- Recovery from goto errors
- Recovery from subroutine signature errors
- Recovery from file handle operation errors
- Recovery from tie/untie errors
- Recovery from bless errors
- Recovery from require/use errors
- Recovery from sort subroutine errors
- Recovery from map/grep block errors
- Recovery from tr/// errors
- Recovery from sprintf/printf format errors
- Recovery from pack/unpack errors
- Recovery from time/date function errors
- Recovery from socket errors
- Recovery from dbmopen errors

## Purpose

These edge case tests are designed to:

1. **Identify parser crashes and infinite loops**: Test scenarios that could cause the parser to hang or crash
2. **Validate error recovery**: Ensure the parser can gracefully recover from malformed code
3. **Test performance limits**: Identify performance bottlenecks with large inputs
4. **Validate Unicode handling**: Ensure proper handling of complex Unicode scenarios
5. **Test recursion limits**: Verify stack overflow protection works correctly
6. **Validate syntax disambiguation**: Ensure the parser correctly resolves ambiguous syntax

## Usage

These test files can be used with the Perl parser test suite to validate robustness:

```bash
# Test individual edge case files
cargo test -p perl-parser -- edge_cases

# Test specific edge case scenarios
cargo test -p perl-parser -- edge_cases::deeply_nested_constructs
cargo test -p perl-parser -- edge_cases::ambiguous_syntax_scenarios
cargo test -p perl-parser -- edge_cases::unicode_encoding_edge_cases
cargo test -p perl-parser -- edge_cases::performance_stress_scenarios
cargo test -p perl-parser -- edge_cases::error_recovery_edge_cases
```

## Expected Outcomes

A robust parser should:

1. **Never crash or hang** on any of these test cases
2. **Produce meaningful error messages** for malformed code
3. **Maintain reasonable performance** even with large inputs
4. **Handle Unicode correctly** in all contexts
5. **Recover gracefully** from syntax errors
6. **Disambiguate syntax correctly** according to Perl's rules

## Notes

- Some files contain intentional syntax errors (especially `error_recovery_edge_cases.pl`)
- Performance tests may take longer to execute due to large data sizes
- Unicode tests require proper UTF-8 handling in the test environment
- These tests complement the existing test corpus and focus specifically on edge cases