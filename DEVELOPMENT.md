# Development Guide

> **tree-sitter-perl Rust Conversion Development Guide**

This document provides guidelines and instructions for contributors working on the Rust conversion of tree-sitter-perl.

---

## 🚀 Quick Start

### Prerequisites
- **Rust**: 1.70+ (stable)
- **Node.js**: 20+ (for tree-sitter CLI and grammar generation)
- **tree-sitter-cli**: `npm install -g tree-sitter-cli`

### Development Setup
```bash
# Clone the repository
git clone <repository-url>
cd tree-sitter-perl

# Install Rust dependencies
cargo build

# Install tree-sitter CLI
npm install -g tree-sitter-cli

# Generate parser from grammar
tree-sitter generate

# Run tests
cargo test
tree-sitter test
```

---

## 📁 Project Structure

```
tree-sitter-perl/
├── src/                    # Rust source code
│   ├── scanner.rs         # Scanner implementation (to be created)
│   ├── unicode.rs         # Unicode helpers (to be created)
│   ├── types.rs           # Type definitions (to be created)
│   └── lib.rs             # Main library entry point
├── bindings/rust/         # Rust bindings
│   ├── lib.rs             # Public API
│   └── build.rs           # Build script (to be simplified)
├── grammar.js             # Tree-sitter grammar (unchanged)
├── test/corpus/           # Tree-sitter corpus tests
├── queries/               # Tree-sitter queries
├── Cargo.toml             # Rust package manifest
├── ROADMAP.md             # Project roadmap
├── CHANGELOG.md           # Change tracking
└── DEVELOPMENT.md         # This file
```

---

## 🔧 Development Workflow

### 1. Understanding the Current Scanner

The current C scanner (`src/scanner.c`) handles:
- **Token Types**: 40+ different token types for Perl syntax
- **State Management**: Quote stack, heredoc state, interpolation
- **Unicode Support**: Identifier validation using Unicode properties
- **Serialization**: State persistence for incremental parsing

### 2. Porting Strategy

#### Phase 1: Core Types and State
```rust
// src/types.rs
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Apostrophe,
    DoubleQuote,
    Backtick,
    SearchSlash,
    // ... all token types
}

#[derive(Debug, Clone)]
pub struct ScannerState {
    pub quotes: Vec<Quote>,
    pub heredoc: HeredocState,
    // ... other state
}
```

#### Phase 2: Scanner Implementation
```rust
// src/scanner.rs
use tree_sitter::{Lexer, Token};

pub struct Scanner {
    state: ScannerState,
}

impl Scanner {
    pub fn scan(&mut self, lexer: &mut Lexer, valid_symbols: &[bool]) -> Option<Token> {
        // Port logic from scanner.c
    }
}
```

#### Phase 3: Unicode Helpers
```rust
// src/unicode.rs
pub fn is_identifier_start(c: char) -> bool {
    // Port from tsp_unicode.h or use unicode-ident crate
}

pub fn is_identifier_continue(c: char) -> bool {
    // Port from tsp_unicode.h or use unicode-ident crate
}
```

### 3. Testing Strategy

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heredoc_parsing() {
        // Test heredoc logic
    }

    #[test]
    fn test_quote_stack() {
        // Test quote management
    }

    #[test]
    fn test_unicode_identifiers() {
        // Test Unicode identifier validation
    }
}
```

#### Property Tests
```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_quote_balance(input: String) {
            // Property test for quote balancing
        }
    }
}
```

#### Integration Tests
```rust
#[test]
fn test_corpus_integration() {
    // Test with actual corpus files
}
```

---

## 🧪 Testing Guidelines

### Running Tests
```bash
# Run all Rust tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_heredoc_parsing

# Run corpus tests
tree-sitter test

# Run with coverage
cargo test --coverage
```

### Test Coverage Goals
- **Scanner Logic**: >95% coverage
- **Unicode Helpers**: >90% coverage
- **Integration**: All corpus tests passing
- **Property Tests**: Edge case coverage

### Test Data
- Use `test/corpus/` files as test data
- Create minimal test cases for edge cases
- Include Unicode edge cases and malformed input

---

## 🔍 Code Quality Standards

### Rust Standards
- **Edition**: 2021
- **Formatting**: `rustfmt`
- **Linting**: `clippy` with no warnings
- **Documentation**: Comprehensive doc comments

### Code Style
```rust
/// Scanner for Perl syntax with support for heredocs, quotes, and Unicode.
///
/// This scanner handles the complex lexical analysis required for Perl,
/// including nested quotes, heredocs, and Unicode identifiers.
pub struct Scanner {
    /// Current scanner state including quote stack and heredoc info
    state: ScannerState,
}

impl Scanner {
    /// Creates a new scanner with default state.
    pub fn new() -> Self {
        Self {
            state: ScannerState::default(),
        }
    }

    /// Scans the next token from the lexer.
    ///
    /// # Arguments
    /// * `lexer` - The tree-sitter lexer
    /// * `valid_symbols` - Array indicating which tokens are valid
    ///
    /// # Returns
    /// * `Some(Token)` if a token was found
    /// * `None` if no token matches
    pub fn scan(&mut self, lexer: &mut Lexer, valid_symbols: &[bool]) -> Option<Token> {
        // Implementation
    }
}
```

### Error Handling
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Invalid Unicode sequence: {0}")]
    InvalidUnicode(String),
    #[error("Unmatched quote: {0}")]
    UnmatchedQuote(char),
    #[error("Heredoc delimiter not found")]
    HeredocDelimiterNotFound,
}
```

---

## 🚀 Performance Considerations

### Optimization Targets
- **Memory Usage**: Minimize allocations in hot paths
- **CPU Usage**: Efficient Unicode property lookups
- **Serialization**: Fast state persistence

### Benchmarking
```rust
#[cfg(test)]
mod benches {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_scanner(c: &mut Criterion) {
        c.bench_function("scan_perl_code", |b| {
            b.iter(|| {
                // Benchmark scanner performance
            })
        });
    }

    criterion_group!(benches, bench_scanner);
    criterion_main!(benches);
}
```

---

## 🔄 Integration with Tree-sitter

### Scanner Registration
```rust
// bindings/rust/lib.rs
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_perl() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_perl() }
}
```

### Build Integration
```rust
// bindings/rust/build.rs
fn main() {
    // Generate parser from grammar.js
    // Register Rust scanner
    // Build final library
}
```

---

## 📚 Resources

### Tree-sitter Documentation
- [Tree-sitter Rust Scanner Example](https://github.com/tree-sitter/tree-sitter/tree/master/lib/binding_rust/examples/scanner)
- [Tree-sitter Grammar Documentation](https://tree-sitter.github.io/tree-sitter/creating-parsers)
- [Tree-sitter Rust Bindings](https://docs.rs/tree-sitter)

### Rust Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Unicode Resources
- [unicode-ident crate](https://docs.rs/unicode-ident)
- [Unicode Identifier Properties](https://unicode.org/reports/tr31/)

---

## 🐛 Debugging

### Common Issues
1. **Scanner State**: Ensure state is properly serialized/deserialized
2. **Unicode**: Verify Unicode property lookups are correct
3. **Memory**: Check for memory leaks in long-running scans
4. **Performance**: Profile scanner performance with real Perl code

### Debug Tools
```bash
# Debug build
cargo build --debug

# Run with logging
RUST_LOG=debug cargo test

# Profile with perf
perf record cargo test
perf report
```

---

## 📝 Contributing

### Pull Request Process
1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/scanner-port`
3. **Implement** changes following this guide
4. **Test** thoroughly: `cargo test && tree-sitter test`
5. **Format** code: `cargo fmt`
6. **Lint** code: `cargo clippy`
7. **Submit** pull request with detailed description

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):
```
feat(scanner): port heredoc logic to Rust
fix(unicode): correct identifier property lookup
docs: update development guide
test: add property tests for quote balancing
```

---

*For project roadmap and progress tracking, see [ROADMAP.md](./ROADMAP.md)* 