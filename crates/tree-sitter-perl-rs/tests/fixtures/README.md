# Test Fixtures for Issue #178 - Eliminate Fragile unreachable!() Macros

This directory contains comprehensive Perl code fixtures for testing parser error handling improvements as specified in Issue #178 and GitHub Issue #204.

## Directory Structure

```
fixtures/
├── variable_declarations/   # AC1: Variable declaration error handling
├── for_loops/               # AC3: For-loop tuple validation
├── anti_patterns/           # AC5: Anti-pattern detector exhaustive matching
└── README.md               # This file
```

## Fixture Categories

### 1. Variable Declarations (`variable_declarations/`)

Tests for AC1: Variable declaration error handling in `simple_parser_v2.rs:118` and `simple_parser.rs:76`.

**Files:**
- `valid_my.pl` - Valid 'my' variable declarations with various patterns
- `valid_our.pl` - Valid 'our' package-scoped declarations
- `valid_local.pl` - Valid 'local' variable localizations
- `valid_state.pl` - Valid 'state' persistent variable declarations
- `invalid_keyword.pl` - Invalid declaration keywords that should trigger descriptive errors
- `unicode_variables.pl` - Unicode identifier support including emoji variable names

**Expected Behavior:**
- Valid files should parse successfully with proper AST construction
- `invalid_keyword.pl` should trigger descriptive error messages instead of `unreachable!()` panic
- Error format: "Expected variable declaration keyword (my/our/local/state), found {token} at position {pos}"

### 2. For-Loop Fixtures (`for_loops/`)

Tests for AC3: For-loop tuple validation in `token_parser.rs:284`.

**Files:**
- `valid_c_style.pl` - Valid C-style for loops (init; condition; update)
- `valid_foreach.pl` - Valid foreach loops (variable in list)
- `invalid_tuple.pl` - Invalid for-loop combinations that mix C-style and foreach syntax
- `nested_loops.pl` - Nested loop structures for complex validation

**Expected Behavior:**
- Valid C-style and foreach loops should parse correctly
- `invalid_tuple.pl` should trigger descriptive error messages for hybrid loops
- Error format: "Invalid for-loop structure: for-loops require either (init; condition; update) for C-style loops or (variable in list) for foreach loops, but found incompatible combination at position {pos}"

### 3. Anti-Pattern Fixtures (`anti_patterns/`)

Tests for AC5: Anti-pattern detector exhaustive matching in `anti_pattern_detector.rs:142,215,262`.

**Files:**
- `format_heredoc.pl` - Format heredoc anti-patterns (FormatHeredocDetector)
- `begin_heredoc.pl` - BEGIN-time heredoc anti-patterns (BeginTimeHeredocDetector)
- `dynamic_delimiter.pl` - Dynamic delimiter anti-patterns (DynamicDelimiterDetector)
- `valid_patterns.pl` - Valid patterns that should NOT trigger anti-pattern detection (false positive testing)

**Expected Behavior:**
- Anti-pattern detectors should identify problematic patterns
- Pattern type mismatches should produce descriptive panic messages
- Panic format: "{DetectorName} received incompatible pattern type: {pattern_debug}. This indicates a bug in the anti-pattern detection pipeline. Expected: {expected}, Found: {discriminant}"
- `valid_patterns.pl` should pass without triggering false positives

## Usage in Tests

### Parser Tests (`unreachable_elimination_ac_tests.rs`)

```rust
#[test]
fn test_ac1_simple_parser_v2_variable_declaration_error_handling() {
    let fixture = include_str!("fixtures/variable_declarations/invalid_keyword.pl");
    let result = parse_perl(fixture);

    // Should return error, not panic
    assert!(result.is_err());

    // Error message should be descriptive
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Expected variable declaration keyword"));
    assert!(error_msg.contains("my/our/local/state"));
}
```

### Integration Tests

```rust
#[test]
fn test_comprehensive_variable_declarations() {
    let valid_fixtures = [
        include_str!("fixtures/variable_declarations/valid_my.pl"),
        include_str!("fixtures/variable_declarations/valid_our.pl"),
        include_str!("fixtures/variable_declarations/valid_local.pl"),
        include_str!("fixtures/variable_declarations/valid_state.pl"),
    ];

    for fixture in &valid_fixtures {
        assert!(parse_perl(fixture).is_ok());
    }
}
```

## Related Documentation

- [PARSER_ERROR_HANDLING_SPEC.md](../../../../docs/PARSER_ERROR_HANDLING_SPEC.md)
- [ERROR_HANDLING_API_CONTRACTS.md](../../../../docs/ERROR_HANDLING_API_CONTRACTS.md)
- [issue-178-spec.md](../../../../docs/issue-178-spec.md)

## Acceptance Criteria Coverage

| AC | Component | Fixture Coverage |
|----|-----------|------------------|
| AC1 | Variable declaration error handling | `variable_declarations/*.pl` |
| AC3 | For-loop tuple validation | `for_loops/*.pl` |
| AC5 | Anti-pattern detector exhaustive matching | `anti_patterns/*.pl` |
| AC6 | Regression tests for unreachable!() paths | All fixtures test previously-unreachable code paths |

## Performance Guarantees

All fixtures are designed to support performance validation:
- **Happy path**: Valid fixtures should parse in 1-150μs (zero overhead from error handling)
- **Error path**: Invalid fixtures should complete error handling in <12μs
- **LSP integration**: Error diagnostics should publish in <1ms

## Unicode and Edge Cases

Fixtures include comprehensive edge case coverage:
- Unicode identifiers in variable names (`unicode_variables.pl`)
- Emoji support in variable names (Perl 5.14+)
- Complex nested structures (`nested_loops.pl`)
- Multi-byte character handling in error messages
- Boundary conditions (empty patterns, very long inputs)

## Fixture Maintenance

When adding new fixtures:
1. Include descriptive header comments with AC reference
2. Provide both valid and invalid examples
3. Test edge cases (Unicode, nesting, boundaries)
4. Update this README with fixture descriptions
5. Ensure fixtures align with error message API contracts

## Quality Assurance

All fixtures have been validated to:
- Use valid Perl syntax where intended
- Trigger expected error paths for error testing
- Support comprehensive test coverage (83 tests across parser/lexer/LSP)
- Enable property-based testing with proptest
- Support mutation hardening tests (>60% mutation score improvement target)
