# Comprehensive Edge Case Test Suite for Perl Parser

This document describes the comprehensive edge case test suite designed to validate the Perl parser's robustness under extreme conditions, pathological inputs, and platform-specific scenarios.

## Overview

The edge case test suite consists of four main test files, each focusing on different aspects of parser robustness:

1. **Extreme Input Edge Cases** (`extreme_input_edge_cases.rs`)
2. **Comprehensive Unicode Edge Cases** (`comprehensive_unicode_edge_cases.rs`)
3. **Platform-Specific Edge Cases** (`platform_specific_edge_cases.rs`)
4. **Performance Stress Edge Cases** (`performance_stress_edge_cases.rs`)

## Test Files

### 1. Extreme Input Edge Cases (`extreme_input_edge_cases.rs`)

**Purpose**: Test parser behavior with extremely large inputs that push the parser to its absolute limits.

**Key Test Categories**:
- Extremely large identifiers (1KB to 1MB)
- Extreme nesting depth (500-3000 levels)
- Extremely large strings (10MB to 100MB)
- Massive data structures (1M element arrays/hashes)
- Pathological regular expressions
- Extremely large source files (100K to 1M lines)
- Complex expressions (deep ternary, massive method chains)
- Concurrent parsing with extreme inputs
- Memory pressure scenarios

**Expected Behavior**:
- Parser should never crash or hang
- Should either parse successfully or fail gracefully with meaningful errors
- Parse time should remain within reasonable limits (30 seconds maximum)
- Memory usage should be bounded

### 2. Comprehensive Unicode Edge Cases (`comprehensive_unicode_edge_cases.rs`)

**Purpose**: Test parser's handling of complex Unicode scenarios including bidirectional text, combining characters, normalization forms, and invalid UTF-8 sequences.

**Key Test Categories**:
- Complex Unicode scripts (Arabic, Hebrew, Chinese, Japanese, Korean, Indic, Cyrillic, etc.)
- Bidirectional text and combining characters
- Unicode normalization forms (NFC, NFD, NFKC, NFKD)
- Invalid UTF-8 sequences and error handling
- Unicode in various Perl constructs (packages, subroutines, variables, etc.)
- Unicode with special variables and file operations
- Unicode in advanced Perl features (OO, signatures, try/catch)

**Expected Behavior**:
- Proper handling of all Unicode scripts
- Correct processing of bidirectional text
- Graceful handling of invalid UTF-8
- Preservation of Unicode identifiers in AST
- Support for Unicode in all Perl constructs

### 3. Platform-Specific Edge Cases (`platform_specific_edge_cases.rs`)

**Purpose**: Test parser behavior with platform-specific scenarios including different path separators, line endings, file system behaviors, and environment-specific features.

**Key Test Categories**:
- Different line ending styles (LF, CRLF, CR, mixed)
- Path separator styles (Unix, Windows, mixed)
- Environment-specific behaviors (Unix vs Windows)
- Character encoding and BOM handling
- File system edge cases (reserved names, long filenames, Unicode)
- Networking edge cases (IPv4/IPv6, URLs, protocols)
- Platform-specific operations (signals, permissions, processes)

**Expected Behavior**:
- Correct handling of all line ending styles
- Proper parsing of different path formats
- Platform-specific feature detection
- Graceful handling of encoding issues
- Cross-platform compatibility

### 4. Performance Stress Edge Cases (`performance_stress_edge_cases.rs`)

**Purpose**: Test parser performance under extreme stress conditions including massive data structures, pathological patterns, resource exhaustion, and concurrent parsing.

**Key Test Categories**:
- Massive data structures (1M+ elements)
- Pathological regex patterns (catastrophic backtracking)
- Extremely large source files (100MB+)
- Deeply nested constructs (10K+ levels)
- Complex expressions (10K+ operations)
- Concurrent parsing stress (16 threads)
- Memory pressure scenarios
- Resource exhaustion scenarios

**Expected Behavior**:
- Performance should remain within acceptable bounds
- No memory leaks or excessive memory usage
- Graceful degradation under stress
- Thread safety in concurrent scenarios
- Reasonable error recovery

## Running the Tests

### Individual Test Files

```bash
# Run extreme input edge cases
cargo test -p perl-parser -- test_extreme_input_edge_cases

# Run Unicode edge cases
cargo test -p perl-parser -- test_comprehensive_unicode_edge_cases

# Run platform-specific edge cases
cargo test -p perl-parser -- test_platform_specific_edge_cases

# Run performance stress edge cases
cargo test -p perl-parser -- test_performance_stress_edge_cases
```

### All Edge Cases

```bash
# Run all edge case tests
cargo test -p perl-parser -- edge_cases

# Run with verbose output
cargo test -p perl-parser -- edge_cases -- --nocapture

# Run with specific test filter
cargo test -p perl-parser -- edge_cases -- --test-threads=1
```

### Performance Considerations

Some edge case tests are resource-intensive and may take significant time to complete:

- **Performance stress tests**: May take 1-5 minutes
- **Extreme input tests**: May take 30 seconds to 2 minutes
- **Unicode tests**: Typically complete within 10 seconds
- **Platform-specific tests**: Typically complete within 5 seconds

For CI environments, consider running these tests with increased timeouts:

```bash
# Increase test timeout for edge cases
RUST_TEST_THREADS=1 cargo test -p perl-parser -- edge_cases -- --test-threads=1
```

## Test Metrics and Monitoring

### Performance Metrics

Each test tracks:
- **Parse time**: Time taken to parse the input
- **Memory usage**: Estimated memory consumption
- **AST size**: Size of the resulting AST
- **Success rate**: Whether parsing succeeded or failed gracefully

### Thresholds

- **Maximum parse time**: 30-60 seconds (varies by test category)
- **Maximum memory usage**: 500MB-1GB (varies by test category)
- **Minimum success rate**: At least some tests should succeed even under stress

### Monitoring

Tests output detailed information about:
- Which specific test cases are being executed
- Parse times for each case
- Success/failure status
- Any errors encountered

## Expected Outcomes

A robust parser should demonstrate:

### Stability
- Never crash or hang on any input
- Handle memory pressure gracefully
- Recover from errors cleanly

### Performance
- Maintain reasonable performance even with extreme inputs
- Scale linearly with input size where possible
- Avoid exponential time complexity

### Correctness
- Produce correct ASTs for valid inputs
- Provide meaningful error messages for invalid inputs
- Handle all Unicode and platform-specific cases correctly

### Robustness
- Handle malformed input gracefully
- Work correctly across different platforms
- Maintain thread safety in concurrent scenarios

## Troubleshooting

### Common Issues

1. **Test timeouts**: Increase timeout values or reduce test data size
2. **Memory exhaustion**: Run tests on machines with sufficient memory
3. **Platform-specific failures**: Verify platform-specific features are available
4. **Unicode issues**: Ensure proper UTF-8 support in test environment

### Debugging

Run tests with verbose output to identify issues:

```bash
# Run with detailed output
cargo test -p perl-parser -- edge_cases -- --nocapture

# Run specific failing test
cargo test -p perl-parser -- test_specific_test_name -- --nocapture

# Run with backtrace on panic
RUST_BACKTRACE=1 cargo test -p perl-parser -- edge_cases
```

## Contributing

When adding new edge case tests:

1. **Categorize appropriately**: Choose the correct test file based on the edge case type
2. **Document thoroughly**: Add clear descriptions of what each test validates
3. **Set reasonable limits**: Use appropriate time and memory thresholds
4. **Test across platforms**: Ensure tests work on Unix, Windows, and macOS
5. **Consider performance**: Avoid tests that take excessively long to complete

## Future Enhancements

Potential areas for expanding edge case testing:

1. **Fuzzing integration**: Automated generation of pathological inputs
2. **Regression testing**: Automated detection of performance regressions
3. **Cross-platform CI**: Testing on multiple operating systems
4. **Memory profiling**: Detailed memory usage analysis
5. **Benchmarking**: Performance tracking over time

## Conclusion

This comprehensive edge case test suite ensures the Perl parser remains robust, performant, and correct under extreme conditions. Regular execution of these tests helps maintain parser quality and prevents regressions in edge case handling.

The tests are designed to be thorough yet efficient, providing confidence in the parser's ability to handle real-world scenarios and adversarial inputs alike.