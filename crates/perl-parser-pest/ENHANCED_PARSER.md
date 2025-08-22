# Enhanced Perl Parser

The enhanced parser (`EnhancedFullParser`) provides comprehensive support for Perl's most challenging edge cases that were previously unsupported.

## New Features

### 1. Enhanced Heredoc Support
- **Backtick heredocs**: `<<`CMD`` for command execution
- **Escaped delimiter heredocs**: `<<\EOF` for non-interpolated content  
- **Whitespace flexibility**: `<< 'EOF'` with spaces around operator
- **Multiple heredocs**: Support for multiple heredocs in a single statement
- **Context preservation**: Heredocs in arrays, hashes, and function arguments

### 2. Special Sections
- **DATA sections**: `__DATA__` content extraction
- **END sections**: `__END__` content handling
- **POD documentation**: Full POD extraction and preservation

### 3. Improved Edge Cases
- Heredocs with special terminators (numeric, keywords)
- Complex mixed content (POD + heredocs + DATA)
- Indented heredocs with proper whitespace handling
- Heredocs in complex data structures

## Usage

```rust
use tree_sitter_perl::EnhancedFullParser;

let mut parser = EnhancedFullParser::new();
let ast = parser.parse(perl_code)?;

// Access extracted sections
if let Some(data_start) = parser.data_section_start {
    println!("DATA section starts at line {}", data_start);
}

for (start, end, content) in &parser.pod_sections {
    println!("POD section from line {} to {}", start, end);
}
```

## Architecture

The enhanced parser uses a multi-phase approach:

1. **Special Section Extraction**: Identifies and extracts DATA/END/POD sections
2. **Enhanced Heredoc Processing**: Uses `EnhancedHeredocLexer` for all heredoc variants
3. **Slash Disambiguation**: Context-aware operator handling
4. **Pest Parsing**: Core grammar parsing
5. **AST Building**: Construct typed AST
6. **Post-processing**: Restore heredoc content and add special sections

## Performance

Benchmarks show the enhanced parser maintains excellent performance:
- Simple heredocs: ~2-5 µs
- Complex mixed content: ~10-20 µs
- Minimal overhead compared to base parser

## Test Coverage

Comprehensive test suite covering:
- All heredoc variants (16 test cases)
- DATA/END section handling
- POD extraction
- Complex mixed content scenarios
- Edge cases with special characters and keywords

All tests passing with `cargo test enhanced_parser_tests`.

## Implementation Files

- `src/enhanced_heredoc_lexer.rs`: Advanced heredoc tokenization
- `src/enhanced_full_parser.rs`: Main enhanced parser implementation
- `tests/enhanced_parser_tests.rs`: Comprehensive test suite
- `benches/enhanced_parser_bench.rs`: Performance benchmarks

## Future Improvements

- Format declaration support
- Even more exotic heredoc edge cases
- Streaming parser for large files
- Error recovery improvements