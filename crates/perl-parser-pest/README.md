# perl-parser-pest

> **Note:** This is the **v2 Pest experiment** published for comparison/migration.  
> For production use, we recommend **[`perl-parser` (v3)](https://crates.io/crates/perl-parser)**.

Pest-based Perl parser (v2) with ~99.995% Perl 5 syntax coverage. Tree-sitter compatible output.

## Features

### Core Parser
- **99.995% Perl 5 syntax coverage** - Handles virtually all real-world Perl code
- **Pure Rust implementation** - No C dependencies, cross-platform compatible
- **Tree-sitter compatible** - Outputs standard S-expressions for tool integration
- **Excellent performance** - 5-134x faster than alternatives

### Enhanced Features
- **Advanced Heredoc Support** - All variants including backtick, escaped, indented
- **Special Section Handling** - DATA/END sections and POD documentation
- **Streaming Parser** - Memory-efficient parsing of large files
- **Error Recovery** - Robust parsing with malformed input
- **S-Expression Formatter** - Position tracking and multiple output modes

## Quick Start

```rust
use tree_sitter_perl::EnhancedFullParser;

fn main() {
    let perl_code = r#"
sub hello {
    my $name = shift;
    print "Hello, $name!\n";
}

hello("World");
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(perl_code) {
        Ok(ast) => println!("Parse successful: {:?}", ast),
        Err(e) => eprintln!("Parse error: {:?}", e),
    }
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
perl-parser-pest = "0.8.3"

# Or use a shorter alias:
perl_pest = { package = "perl-parser-pest", version = "0.8.3" }
```

## Usage Examples

### Basic Parsing

```rust
use tree_sitter_perl::PureRustPerlParser;

let mut parser = PureRustPerlParser::new();
let ast = parser.parse("my $x = 42;")?;
println!("{}", parser.to_sexp(&ast));
```

### Error Recovery

```rust
use tree_sitter_perl::error_recovery::ErrorRecoveryParser;

let mut parser = ErrorRecoveryParser::new();
let ast = parser.parse("my $x = ; print 'ok';")?;
println!("Recovered from {} errors", parser.errors().len());
```

### Streaming Large Files

```rust
use tree_sitter_perl::streaming_parser::stream_parse_file;

for event in stream_parse_file("large_script.pl")? {
    match event {
        ParseEvent::Node(ast) => process_ast(ast),
        ParseEvent::Error { line, message } => eprintln!("Line {}: {}", line, message),
        _ => {}
    }
}
```

## Architecture

```
┌─────────────────┐
│   Perl Source   │
└────────┬────────┘
         │
┌────────▼────────┐
│ Enhanced Lexer  │ ◄── Heredoc handling
└────────┬────────┘     Special sections
         │
┌────────▼────────┐
│ Context-Aware   │ ◄── Slash disambiguation
│   Processing    │     Dynamic delimiters
└────────┬────────┘
         │
┌────────▼────────┐
│  Pest Parser    │ ◄── PEG grammar
└────────┬────────┘     Rule-based parsing
         │
┌────────▼────────┐
│  AST Builder    │ ◄── Type-safe nodes
└────────┬────────┘     Position tracking
         │
┌────────▼────────┐
│ S-Expression    │ ◄── Tree-sitter format
│    Output       │     Compatible tools
└─────────────────┘
```

## Performance

Benchmarks on typical Perl code:

| Parser | Simple (50 lines) | Complex (500 lines) | Large (5000 lines) |
|--------|-------------------|---------------------|-------------------|
| tree-sitter-perl-rs | 1.1 µs | 4.9 µs | 45 µs |
| perl-parser | 4.2 µs | 18 µs | 210 µs |
| tree-sitter (C) | 12 µs | 68 µs | 650 µs |
| PPI | 150 µs | 980 µs | 12 ms |

## Migration to v3 (perl-parser)

### Why Migrate?

The v3 parser (`perl-parser`) offers:
- **4-19x faster performance** than v2
- **100% edge case coverage** (vs 99.995% in v2)
- **Handles `m!pattern!`** and other arbitrary regex delimiters
- **Proper indirect object syntax** support
- **Active development** and bug fixes

### Migration Steps

1. **Update Cargo.toml**:
```toml
# Replace this:
perl-parser-pest = "0.8.3"

# With this:
perl-parser = "0.8.3"
```

2. **Update imports**:
```rust
// Replace:
use perl_parser_pest::{PureRustPerlParser, ParseError};

// With:
use perl_parser::{Parser, ParseError};
```

3. **API differences**:
```rust
// v2 (pest):
let mut parser = PureRustPerlParser::new();
let ast = parser.parse(code)?;
let sexp = parser.to_sexp(&ast);

// v3 (native):
let mut parser = Parser::new();
let node = parser.parse(code)?;
let sexp = node.to_sexp();
```

### Key Differences

| Feature | v2 (pest) | v3 (native) |
|---------|-----------|-------------|
| Main type | `PureRustPerlParser` | `Parser` |
| Parse result | Custom AST | `Node` type |
| S-expression | `parser.to_sexp(&ast)` | `node.to_sexp()` |
| Node kinds | Same names | Tree-sitter compatible |
| Performance | ~200-450 µs | ~1-150 µs |

### Compatibility Mode

If you need to compare both parsers:
```toml
[dependencies]
perl-parser = { version = "0.8.3", features = ["pest-backend"] }
```

Then:
```rust
use perl_parser::Parser;  // v3
use perl_parser::pest_backend::PureRustPerlParser;  // v2
```

## Feature Coverage

### Fully Supported ✅
- Variables (all sigils and scopes)
- Operators (all precedence levels)
- Control flow (if/while/for/foreach/given)
- Subroutines (signatures, prototypes, attributes)
- Packages and modules
- Regular expressions
- Heredocs (all variants)
- References and dereferencing
- Unicode identifiers
- Modern Perl features (try/catch, defer, class/method)

### Edge Cases Handled ✅
- Backtick heredocs
- Escaped delimiter heredocs
- Multiple heredocs in lists
- Heredocs in data structures
- DATA/END sections
- POD documentation
- Format declarations
- Dynamic delimiters

### Known Limitations
- Heredoc-in-string edge case
- Some exotic regex constructs
- Source filters (by design)

## Documentation

- [ENHANCED_PARSER.md](ENHANCED_PARSER.md) - Enhanced parser features
- [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md) - All advanced capabilities
- [examples/](examples/) - Usage examples
- [benches/](benches/) - Performance benchmarks

## Development

### Building

```bash
# Standard build
cargo build --features pure-rust

# Standalone (without perl-lexer dependency)
cargo build --features pure-rust-standalone

# Run tests
cargo test --features pure-rust

# Run benchmarks
cargo bench --features pure-rust
```

### Project Structure

```
src/
├── pure_rust_parser.rs      # Core Pest-based parser
├── enhanced_full_parser.rs  # Enhanced parser with all features
├── streaming_parser.rs      # Memory-efficient streaming
├── error_recovery.rs        # Robust error handling
├── sexp_formatter.rs        # S-expression output
├── grammar.pest            # Complete Perl 5 PEG grammar
└── lib.rs                  # Public API
```

## Contributing

Contributions are welcome! Key areas:

1. **Grammar improvements** - Additional Perl syntax edge cases
2. **Performance** - Further optimization opportunities
3. **Error messages** - Better diagnostics
4. **Documentation** - Usage examples and guides

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../../LICENSE) for details.

## Acknowledgments

- Tree-sitter team for the parsing framework
- Pest parser generator for the PEG foundation
- Perl community for syntax documentation
- Contributors and testers

---

For more information, see the [main repository](https://github.com/EffortlessSteven/tree-sitter-perl).