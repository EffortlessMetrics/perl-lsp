# Tree-sitter Perl - Rust Architecture

This document describes the architecture of the tree-sitter-perl Rust crate, which provides a high-performance Perl parser with both Rust-native and C scanner implementations.

## Overview

The tree-sitter-perl crate is designed with a modular, feature-based architecture that supports:

- **Dual Scanner Support**: Both Rust-native and C scanner implementations
- **Comprehensive Error Handling**: Detailed error types and recovery mechanisms
- **Unicode Support**: Full Unicode normalization and validation
- **Performance Optimization**: Benchmarks and performance monitoring
- **Extensive Testing**: Unit tests, integration tests, and property-based testing

## Directory Structure

```
src/
├── lib.rs                 # Main library entry point
├── error.rs              # Error types and handling
├── scanner/              # Scanner implementations
│   ├── mod.rs           # Scanner trait and common types
│   ├── rust_scanner.rs  # Rust-native scanner
│   └── c_scanner.rs     # C scanner wrapper
├── unicode/              # Unicode handling
│   └── mod.rs           # Unicode utilities
├── test_utils/           # Testing utilities
│   └── mod.rs           # Test helpers and data generation
├── tests.rs              # Integration tests
└── benchmarks/           # Performance benchmarks
    ├── scanner_benchmarks.rs
    └── parser_benchmarks.rs

benches/                   # Criterion benchmarks
├── scanner_benchmarks.rs
└── parser_benchmarks.rs

build.rs                  # Build script
```

## Core Components

### 1. Library Entry Point (`src/lib.rs`)

The main library provides:
- Language loading and parser creation
- High-level parsing functions
- Feature-based compilation control

```rust
// Get the tree-sitter language
pub fn language() -> Language

// Create a new parser instance
pub fn parser() -> Parser

// Parse Perl source code
pub fn parse(source: &str) -> Result<Tree, ParseError>
```

### 2. Error Handling (`src/error.rs`)

Comprehensive error types for different parsing scenarios:

```rust
pub enum ParseError {
    ParseFailed,
    ScannerError { message: String, position: Option<(usize, usize)> },
    UnicodeError { message: String },
    InvalidToken { token: String, position: (usize, usize) },
    UnterminatedString { position: (usize, usize) },
    UnterminatedBlock { position: (usize, usize) },
    // ... more error types
}
```

### 3. Scanner Module (`src/scanner/`)

#### Scanner Trait (`mod.rs`)
Defines the interface for lexical analysis:

```rust
pub trait PerlScanner {
    fn scan(&mut self, lexer: &mut Lexer) -> ParseResult<Option<u16>>;
    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()>;
    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()>;
    fn is_eof(&self) -> bool;
    fn position(&self) -> (usize, usize);
}
```

#### Rust Scanner (`rust_scanner.rs`)
High-performance Rust-native implementation with:
- Unicode-aware tokenization
- Comprehensive Perl syntax support
- Error recovery mechanisms
- State serialization/deserialization

#### C Scanner (`c_scanner.rs`)
Wrapper around the existing C implementation for:
- Backward compatibility
- Performance comparison
- Gradual migration support

### 4. Unicode Support (`src/unicode/`)

Handles Unicode normalization and validation:

```rust
pub struct UnicodeUtils;

impl UnicodeUtils {
    pub fn normalize(input: &str, form: NormalizationForm) -> ParseResult<String>;
    pub fn is_identifier_start(ch: char) -> bool;
    pub fn is_identifier_continue(ch: char) -> bool;
    pub fn validate_surrogate_pair(high: u16, low: u16) -> ParseResult<char>;
    // ... more Unicode utilities
}
```

### 5. Test Utilities (`src/test_utils/`)

Comprehensive testing support:

```rust
pub struct TestUtils;

impl TestUtils {
    pub fn parse_perl_code(code: &str) -> ParseResult<Tree>;
    pub fn validate_tree_no_errors(tree: &Tree) -> ParseResult<()>;
    pub fn tree_to_sexp(tree: &Tree) -> String;
    pub fn compare_trees(tree1: &Tree, tree2: &Tree) -> Vec<String>;
    pub fn generate_test_data() -> Vec<String>;
}
```

## Features

The crate supports several compile-time features:

### `rust-scanner` (default)
Enables the Rust-native scanner implementation.

### `c-scanner`
Enables the C scanner implementation for backward compatibility.

### `test-utils`
Includes testing utilities and test data generation.

## Usage Examples

### Basic Parsing

```rust
use tree_sitter_perl::{parse, language};

// Parse Perl code
let source = "my $var = 42; print 'Hello, World!';";
let tree = parse(source)?;

// Get the root node
let root = tree.root_node();
println!("{}", root.to_sexp());
```

### Custom Parser Configuration

```rust
use tree_sitter_perl::{parser, language};
use tree_sitter::Parser;

let mut parser = parser();
parser.set_language(language())?;

let tree = parser.parse("my $var = 42;", None)?;
```

### Error Handling

```rust
use tree_sitter_perl::parse;

match parse("my $var = 1 +;") {
    Ok(tree) => {
        // Handle successful parse
    }
    Err(ParseError::InvalidToken { token, position }) => {
        println!("Invalid token '{}' at {:?}", token, position);
    }
    Err(ParseError::UnterminatedString { position }) => {
        println!("Unterminated string at {:?}", position);
    }
    Err(e) => {
        println!("Parse error: {}", e);
    }
}
```

## Performance

The crate includes comprehensive benchmarks to measure performance:

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench scanner_benchmarks

# Run with specific features
cargo bench --features rust-scanner
cargo bench --features c-scanner
```

### Benchmark Categories

1. **Scanner Benchmarks**: Compare Rust vs C scanner performance
2. **Parser Benchmarks**: Overall parsing performance
3. **Memory Usage**: Memory consumption analysis
4. **Error Recovery**: Performance with malformed input
5. **Unicode Handling**: Performance with Unicode content

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with specific features
cargo test --features rust-scanner
cargo test --features c-scanner
cargo test --features test-utils

# Run specific test modules
cargo test integration_tests
cargo test scanner_tests
cargo test error_tests
```

### Test Categories

1. **Integration Tests**: End-to-end parsing functionality
2. **Scanner Tests**: Lexical analysis correctness
3. **Error Tests**: Error handling and recovery
4. **Property Tests**: Property-based testing with proptest
5. **Performance Tests**: Performance regression detection

## Build System

### Build Script (`build.rs`)

The build script handles:
- C parser compilation (always required)
- C scanner compilation (when `c-scanner` feature is enabled)
- Binding generation for C functions
- Feature-based compilation control

### Dependencies

#### Required Dependencies
- `tree-sitter`: Core tree-sitter functionality
- `tree-sitter-language`: Language support
- `unicode-ident`: Unicode identifier validation
- `unicode-normalization`: Unicode normalization
- `thiserror`: Error handling

#### Development Dependencies
- `proptest`: Property-based testing
- `criterion`: Performance benchmarking
- `pretty_assertions`: Enhanced test assertions
- `test-case`: Parameterized testing

#### Build Dependencies
- `cc`: C compilation
- `bindgen`: C binding generation

## Migration Strategy

### Phase 1: Dual Scanner Support
- Maintain both C and Rust scanner implementations
- Use feature flags to switch between implementations
- Comprehensive testing to ensure parity

### Phase 2: Performance Optimization
- Benchmark both implementations
- Optimize Rust scanner based on performance data
- Address any performance regressions

### Phase 3: Rust Scanner Default
- Make Rust scanner the default implementation
- Deprecate C scanner (but keep for compatibility)
- Update documentation and examples

### Phase 4: C Scanner Removal
- Remove C scanner implementation
- Simplify build system
- Update dependencies

## Contributing

### Development Setup

1. Clone the repository
2. Install dependencies: `cargo build`
3. Run tests: `cargo test`
4. Run benchmarks: `cargo bench`

### Code Style

- Follow Rust formatting guidelines (`cargo fmt`)
- Use clippy for linting (`cargo clippy`)
- Write comprehensive tests for new features
- Add benchmarks for performance-critical code

### Testing Guidelines

- Write unit tests for all public APIs
- Include integration tests for end-to-end functionality
- Use property-based testing for complex logic
- Add performance benchmarks for new features

## Future Enhancements

### Planned Features

1. **Incremental Parsing**: Optimize for editor integration
2. **Syntax Highlighting**: Query-based highlighting support
3. **Language Server**: LSP support for IDEs
4. **Plugin System**: Extensible parsing capabilities
5. **WebAssembly**: WASM compilation for web usage

### Performance Improvements

1. **SIMD Optimization**: Vectorized text processing
2. **Memory Pooling**: Reduce allocation overhead
3. **Parallel Parsing**: Multi-threaded parsing for large files
4. **Caching**: Parse result caching for repeated content

### Documentation

1. **API Documentation**: Comprehensive API docs
2. **Examples**: More usage examples
3. **Tutorials**: Step-by-step guides
4. **Performance Guide**: Performance optimization tips 