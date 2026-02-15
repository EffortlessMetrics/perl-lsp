# tree-sitter-perl-rs

> ⚠️ **This crate is a Tree-sitter validation harness (benches/examples) for internal use.**  
> **It is not published to crates.io.**  
> For the v2 Pest-based library crate, see **[`perl-parser-pest`](https://crates.io/crates/perl-parser-pest)**.  
> For the production parser, use **[`perl-parser` (v3)](https://crates.io/crates/perl-parser)**.

## Purpose

This crate serves as an internal validation and benchmarking harness for comparing:
- v1: C-based Tree-sitter parser
- v2: Pest-based parser (`perl-parser-pest`)
- v3: Native lexer+parser (`perl-parser`)

## Usage (Internal Development Only)

```bash
# Run benchmarks
cargo run --bin ts_benchmark_parsers --features pure-rust

# Test parsers
cargo run --bin ts_test_parsers --features pure-rust

# Parse a file
cargo run --bin ts-parse-rust --features pure-rust -- file.pl

# Opt in to the microcrate bridge for v2 Pest modules
# (routes pure_rust_parser/pratt_parser/sexp_formatter through perl-parser-pest)
cargo check --features pure-rust,v2-pest-microcrate
```

## Do Not Use in Production

This crate:
- Contains duplicate code for testing purposes
- Is not maintained for external use
- Will not be published to crates.io
- May change or be removed without notice

The `v2-pest-microcrate` feature is an incremental extraction path and is currently opt-in.

For production use, choose:
- **`perl-parser`** - Recommended v3 native parser
- **`perl-parser-pest`** - v2 Pest-based parser (legacy/migration)

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
├── pure_rust_parser.rs      # In-crate v2 parser (default path)
├── pure_rust_parser_bridge.rs # Optional bridge to perl-parser-pest
├── enhanced_full_parser.rs  # Enhanced parser with all features
├── streaming_parser.rs      # Memory-efficient streaming
├── error_recovery.rs        # Robust error handling
├── sexp_formatter.rs        # S-expression output
├── sexp_formatter_bridge.rs # Optional bridge to perl-parser-pest
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

For more information, see the [main repository](https://github.com/EffortlessMetrics/tree-sitter-perl).
