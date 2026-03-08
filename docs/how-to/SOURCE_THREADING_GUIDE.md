# Source Threading Architecture Guide

## Overview

All LSP providers now support source-aware analysis for enhanced documentation extraction (v0.8.7+):

## Provider Constructor Patterns

```rust
// Enhanced constructors with source text (v0.8.7)
CompletionProvider::new_with_index_and_source(ast, source, workspace_index)
SignatureHelpProvider::new_with_source(ast, source)
SymbolExtractor::new_with_source(source)

// Legacy constructors (still supported)
CompletionProvider::new_with_index(ast, workspace_index)  // uses empty source
SignatureHelpProvider::new(ast)  // uses empty source
SymbolExtractor::new()  // no documentation extraction
```

## Comment Documentation Extraction

Comprehensive enhancements in PR #71:

- **Leading Comments**: Extracts multi-line comments immediately preceding symbol declarations with precise boundary detection
- **Blank Line Handling**: Stops at actual blank lines (not whitespace-only lines) for accurate comment boundaries  
- **Whitespace Resilient**: Handles varying indentation and comment prefixes (`#`, `##`, `###`) with automatic normalization
- **Performance Optimized**: <100µs extraction time with pre-allocated string capacity for large comment blocks
- **Unicode Safe**: Proper UTF-8 character boundary handling with support for international comments and emojis
- **Multi-Package Support**: Correct comment extraction across package boundaries with qualified name resolution
- **Edge Case Robust**: Handles empty comments, source boundaries, non-ASCII whitespace, and complex formatting scenarios
- **Method Documentation**: Full support for class methods, subroutines, and variable list declarations
- **Production Testing**: 20 comprehensive test cases covering all edge cases and performance scenarios
- **AST Integration**: Documentation attached to Symbol structs for use across all LSP features with source threading

## Comment Documentation Examples

```perl
# This documents the function below
# Multiple line comments are supported
# with proper boundary detection
sub documented_function {
    # Internal comment - not documentation
}

### Variable documentation with various comment styles  
   ### Works with extra whitespace and hashes
my $documented_var = 42;

# This will NOT be captured as documentation for foo
# because there's a blank line

sub foo {  # Not documentation
}
```

## Testing Commands

```bash
# Test comprehensive comment extraction (20 tests covering all scenarios)
cargo test -p perl-parser --test symbol_documentation_tests

# Test specific comment patterns and edge cases (PR #71 comprehensive coverage)
cargo test -p perl-parser symbol_documentation_tests::comment_separated_by_blank_line_is_not_captured
cargo test -p perl-parser symbol_documentation_tests::comment_with_extra_hashes_and_spaces
cargo test -p perl-parser symbol_documentation_tests::multi_package_comment_scenarios
cargo test -p perl-parser symbol_documentation_tests::complex_comment_formatting
cargo test -p perl-parser symbol_documentation_tests::unicode_in_comments
cargo test -p perl-parser symbol_documentation_tests::performance_with_large_comment_blocks

# Test new edge case coverage (PR #71 additions)
cargo test -p perl-parser symbol_documentation_tests::mixed_comment_styles_and_blank_lines
cargo test -p perl-parser symbol_documentation_tests::variable_list_declarations_with_comments
cargo test -p perl-parser symbol_documentation_tests::method_comments_in_class
cargo test -p perl-parser symbol_documentation_tests::whitespace_only_lines_vs_blank_lines
cargo test -p perl-parser symbol_documentation_tests::bless_with_comment_documentation

# Performance benchmarking (<100µs per iteration target)
cargo test -p perl-parser symbol_documentation_tests::performance_benchmark_comment_extraction -- --nocapture
```