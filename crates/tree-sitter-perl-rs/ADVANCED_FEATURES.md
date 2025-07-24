# Advanced Features - Tree-sitter Perl

This document describes the advanced features implemented in the tree-sitter-perl-rs parser beyond the standard parsing capabilities.

## Table of Contents

1. [Enhanced Parser](#enhanced-parser)
2. [S-Expression Formatter](#s-expression-formatter)
3. [Streaming Parser](#streaming-parser)
4. [Error Recovery](#error-recovery)
5. [Performance Considerations](#performance-considerations)

## Enhanced Parser

The `EnhancedFullParser` provides comprehensive support for Perl's most challenging features:

### Features

- **Advanced Heredocs**: All variants including backtick, escaped, indented, and whitespace-flexible
- **Special Sections**: DATA/END section extraction and POD documentation parsing
- **Multi-phase Processing**: Handles context-sensitive features through careful preprocessing

### Usage

```rust
use tree_sitter_perl::EnhancedFullParser;

let mut parser = EnhancedFullParser::new();
let ast = parser.parse(perl_code)?;

// Access extracted sections
println!("POD sections: {:?}", parser.pod_sections);
println!("DATA section at line: {:?}", parser.data_section_start);
```

### Supported Heredoc Variants

```perl
# Backtick heredoc (command execution)
my $output = <<`CMD`;
ls -la
CMD

# Escaped delimiter (no interpolation)
my $literal = <<\EOF;
No $interpolation here
EOF

# Whitespace around operator
my $spaced = << 'EOF';
Content here
EOF

# Indented heredoc
my $indented = <<~'END';
    This preserves
    relative indentation
END
```

## S-Expression Formatter

The `SexpFormatter` provides tree-sitter compatible S-expression output with advanced features:

### Features

- Position tracking (byte offsets)
- Field names for all node properties
- Compact and pretty-print modes
- Error node handling

### Usage

```rust
use tree_sitter_perl::sexp_formatter::SexpFormatter;

let formatter = SexpFormatter::new(source_code)
    .with_positions(true)  // Include byte positions
    .compact(false);       // Pretty-print mode

let sexp = formatter.format(&ast);
```

### Output Format

```lisp
(source_file [0-100]
  (subroutine_declaration [0-50] name: foo
    (body [10-48]
      (block [15-48]
        (statement [20-40]
          (assignment [20-39]
            operator: =
            (left (scalar_variable [20-22] name: $x))
            (right (number [25-27] value: 42))))))))
```

## Streaming Parser

The `StreamingParser` enables parsing of large files without loading them entirely into memory:

### Features

- Chunk-based processing
- Event-driven parsing
- Special section detection
- Memory-efficient for large codebases

### Usage

```rust
use tree_sitter_perl::streaming_parser::{StreamingParser, StreamConfig};
use std::fs::File;

let file = File::open("large_script.pl")?;
let config = StreamConfig {
    buffer_size: 8192,
    max_statement_size: 65536,
    emit_partial: false,
};

let mut parser = StreamingParser::new(file, config);

for event in parser.parse() {
    match event {
        ParseEvent::Node(ast) => println!("Parsed: {:?}", ast),
        ParseEvent::SpecialSection { kind, start_line, .. } => {
            println!("Found {:?} section at line {}", kind, start_line);
        }
        ParseEvent::Error { line, message } => {
            eprintln!("Error at line {}: {}", line, message);
        }
        _ => {}
    }
}
```

### Event Types

- `StatementStart`: Beginning of a new statement
- `Node`: Complete AST node parsed
- `StatementEnd`: End of statement
- `Error`: Parse error with recovery
- `SpecialSection`: POD/DATA/END sections

## Error Recovery

The `ErrorRecoveryParser` provides robust parsing even with malformed input:

### Features

- Multiple recovery strategies
- Configurable recovery attempts
- Error node generation
- Detailed error reporting

### Recovery Strategies

```rust
use tree_sitter_perl::error_recovery::{ErrorRecoveryParser, RecoveryStrategy};

let mut parser = ErrorRecoveryParser::new()
    .with_strategies(vec![
        RecoveryStrategy::CreateErrorNode,     // Skip single token
        RecoveryStrategy::ParseAsExpression,    // Try as expression
        RecoveryStrategy::SkipToStatementEnd,   // Skip to ; or }
        RecoveryStrategy::SkipLine,            // Skip entire line
        RecoveryStrategy::SkipBlock,           // Skip to matching }
    ])
    .with_max_attempts(10);

let ast = parser.parse(malformed_code)?;
let errors = parser.errors();
```

### Error Information

Each error contains:
- Line and column position
- Expected vs. found tokens
- Recovery strategy used
- Partial content that failed

## Performance Considerations

### Benchmarks

| Feature | Simple Code | Complex Code | Large File (1MB) |
|---------|------------|--------------|------------------|
| Enhanced Parser | ~5 µs | ~20 µs | ~2 ms |
| Streaming Parser | ~10 µs/chunk | ~30 µs/chunk | ~5 ms total |
| Error Recovery | ~8 µs | ~50 µs | ~10 ms |
| S-Expression | ~2 µs | ~10 µs | ~1 ms |

### Memory Usage

- **Enhanced Parser**: O(n) where n is input size
- **Streaming Parser**: O(1) constant memory per chunk
- **Error Recovery**: O(e) additional where e is error count

### Optimization Tips

1. Use streaming parser for files > 1MB
2. Enable error recovery only when needed
3. Use compact S-expressions for performance
4. Disable position tracking if not required

## Integration Example

Complete example using all features:

```rust
use tree_sitter_perl::{
    EnhancedFullParser,
    streaming_parser::stream_parse_file,
    error_recovery::ErrorRecoveryParser,
    sexp_formatter::SexpFormatter,
};

fn parse_perl_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // For small files, use enhanced parser
    let content = std::fs::read_to_string(path)?;
    
    if content.len() < 1_000_000 {
        let mut parser = EnhancedFullParser::new();
        match parser.parse(&content) {
            Ok(ast) => {
                let formatter = SexpFormatter::new(&content);
                println!("{}", formatter.format(&ast));
            }
            Err(_) => {
                // Fall back to error recovery
                let mut recovery = ErrorRecoveryParser::new();
                let ast = recovery.parse(&content)?;
                println!("Recovered with {} errors", recovery.errors().len());
            }
        }
    } else {
        // For large files, use streaming
        for event in stream_parse_file(path)? {
            println!("{:?}", event);
        }
    }
    
    Ok(())
}
```

## Future Enhancements

Planned improvements include:

1. **Incremental Parsing**: Re-parse only changed portions
2. **Parallel Parsing**: Multi-threaded parsing for large files
3. **Language Server Protocol**: Full LSP implementation
4. **Semantic Analysis**: Type checking and symbol resolution
5. **Code Generation**: AST to Perl code generation

## Contributing

See the main README for contribution guidelines. Key areas for contribution:

- Additional error recovery strategies
- Performance optimizations
- Language feature coverage
- Documentation improvements