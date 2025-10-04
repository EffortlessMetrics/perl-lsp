# Corpus Integration Reference Fixtures

This directory contains references and integration points to the comprehensive Perl test corpus maintained in the `perl-corpus` crate.

## Purpose

The perl-dap crate leverages the existing comprehensive Perl syntax coverage from the perl-corpus crate to:

1. **Validate DAP Breakpoint Handling**: Test breakpoint verification across ~100% Perl syntax coverage
2. **Test Variable Inspection**: Validate variable rendering for all Perl data types and edge cases
3. **Benchmark Performance**: Use realistic Perl code samples for performance testing
4. **Property-Based Testing**: Leverage corpus test generation infrastructure for DAP fuzzing

## Integration Pattern

Rather than duplicating comprehensive Perl test files, DAP tests should reference the perl-corpus crate directly:

```rust
use perl_corpus::get_test_files;

#[test]
fn test_dap_breakpoints_comprehensive_corpus() {
    let corpus_files = get_test_files();

    for file in corpus_files {
        // Test DAP breakpoint validation on corpus file
        // Verify AST-based breakpoint validation works across all syntax patterns
    }
}
```

## Available Corpus Files

### Fuzz Testing Files
- `/crates/perl-corpus/fuzz/transliteration_parser_issue.pl` - Transliteration operator edge cases

### Property-Based Test Generation
The perl-corpus crate includes property-based testing infrastructure that can generate:
- Random Perl syntax patterns
- Edge case combinations
- Unicode identifiers and emoji
- Complex nested data structures
- All Perl operator variants

## DAP-Specific Test Requirements

### Breakpoint Validation Corpus
DAP tests should validate breakpoints against:
- ✅ All executable statement types
- ✅ BEGIN/END/CHECK/INIT blocks
- ✅ Heredoc boundaries
- ✅ POD documentation boundaries
- ✅ Comment-only lines
- ✅ Multiline statements
- ✅ String literals spanning multiple lines

### Variable Rendering Corpus
DAP tests should validate variable inspection for:
- ✅ Scalars (strings, numbers, references)
- ✅ Arrays (flat, nested, circular references)
- ✅ Hashes (flat, nested, circular references)
- ✅ Blessed objects
- ✅ Unicode strings (emoji, CJK, RTL languages)
- ✅ Large data structures (>10KB)
- ✅ Typeglobs and special variables

### Performance Benchmark Corpus
Use corpus files for performance testing:
- Small files (<100 lines) - baseline measurements
- Medium files (100-1000 lines) - typical development scenarios
- Large files (1000-10000 lines) - enterprise codebase simulation
- Extra-large files (>10000 lines) - stress testing

## Usage in Tests

### Example: Comprehensive Breakpoint Matrix Test
```rust
#[test]
fn test_breakpoint_validation_comprehensive_corpus() {
    let corpus_files = perl_corpus::get_all_test_files();

    for file_path in corpus_files {
        let source = std::fs::read_to_string(&file_path).unwrap();
        let ast = perl_parser::parse(&source).unwrap();

        // Test breakpoint validation on every line
        for line_num in 1..=source.lines().count() {
            let is_valid = validate_breakpoint_location(&ast, line_num);

            // Verify AST-based validation matches expected behavior
            if is_valid {
                assert!(is_executable_line(&ast, line_num));
            }
        }
    }
}
```

### Example: Variable Rendering Edge Cases
```rust
#[test]
fn test_variable_rendering_corpus_data_structures() {
    let test_cases = perl_corpus::get_complex_data_structure_tests();

    for test_case in test_cases {
        let variables = extract_variables_from_scope(&test_case);

        for var in variables {
            let rendered = render_variable_for_dap(&var);

            // Validate rendering quality
            assert!(rendered.len() <= 1024); // 1KB truncation
            assert!(is_valid_utf8(&rendered)); // Unicode safety
            assert!(!has_circular_reference_infinite_loop(&rendered));
        }
    }
}
```

## Quality Standards

All corpus integration tests must:
- ✅ Cover ~100% Perl syntax patterns from perl-corpus
- ✅ Validate DAP protocol compliance for all corpus files
- ✅ Maintain <50ms breakpoint validation performance
- ✅ Handle Unicode safely (PR #153 symmetric position conversion)
- ✅ Support incremental parsing (<1ms updates)
- ✅ Integrate with property-based testing infrastructure

## Cross-Reference

- **perl-corpus crate**: `/crates/perl-corpus/`
- **perl-parser comprehensive tests**: `/crates/perl-parser/tests/`
- **DAP breakpoint validation**: `DAP_BREAKPOINT_VALIDATION_GUIDE.md`
- **DAP implementation spec**: `DAP_IMPLEMENTATION_SPECIFICATION.md`
