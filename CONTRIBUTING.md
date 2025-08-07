# Contributing to Pure Rust Perl Parser

Thank you for your interest in contributing to the Pure Rust Perl Parser! This document provides guidelines for contributing to the project.

## Table of Contents

- [Project Structure](#project-structure)
- [Development Setup](#development-setup)
- [Testing Guidelines](#testing-guidelines)
- [Adding New Features](#adding-new-features)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)

## Project Structure

```
tree-sitter-perl/
├── crates/tree-sitter-perl-rs/     # Pure Rust Perl Parser
│   ├── src/
│   │   ├── grammar.pest            # Pest PEG grammar for Perl 5
│   │   ├── pure_rust_parser.rs     # Main parser implementation
│   │   ├── edge_case_handler.rs    # Edge case handling system
│   │   └── lib.rs                  # Public API
│   └── Cargo.toml
├── docs/                           # Architecture and design docs
├── xtask/                          # Development automation
├── benches/                        # Performance benchmarks
└── tree-sitter-perl/               # Legacy reference (corpus tests)
```

## Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/EffortlessSteven/tree-sitter-perl.git
   cd tree-sitter-perl
   ```

2. **Install dependencies**
   ```bash
   # Rust toolchain
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Node.js (for tree-sitter CLI)
   npm install -g tree-sitter-cli
   ```

3. **Build the project**
   ```bash
   # Build the Pure Rust parser (default)
   cd crates/tree-sitter-perl-rs
   cargo build --features pure-rust
   
   # Or use xtask from root
   cargo xtask build --features pure-rust
   ```

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo xtask test

# Run specific test suite
cargo test --features pure-rust --test comprehensive_feature_tests

# Run corpus tests with diagnostics
cargo xtask corpus --diagnose

# Run a single test
cargo test test_name
```

### Writing Tests

#### 1. Unit Tests

Add unit tests directly in the source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let input = "my $var = 42;";
        let result = parse(input);
        assert!(result.is_ok());
    }
}
```

#### 2. Integration Tests

Create new test files in `crates/tree-sitter-perl-rs/tests/`:

```rust
// tests/my_feature_test.rs
use tree_sitter_perl::PureRustParser;

#[test]
fn test_complex_feature() {
    let parser = PureRustParser::new();
    let input = r#"
        package MyPackage;
        use strict;
        my $x = 42;
    "#;
    
    let result = parser.parse(input);
    assert!(result.is_ok());
    // Add more specific assertions
}
```

#### 3. Corpus Tests

Add corpus tests to `tree-sitter-perl/test/corpus/`:

```
==================
Test Name Here
==================

my $var = "hello";
print $var;

---

(source_file
  (variable_declaration
    (scalar_variable)
    (string))
  (function_call
    (identifier)
    (scalar_variable)))
```

### Test Categories

When adding tests, consider these categories:

1. **Positive Tests**: Valid Perl code that should parse successfully
2. **Negative Tests**: Invalid code that should fail with appropriate errors
3. **Edge Cases**: Boundary conditions and unusual constructs
4. **Performance Tests**: Large files or complex nested structures
5. **Regression Tests**: Previously broken cases

## Adding New Features

### 1. Grammar Changes

#### For Tree-sitter (C parser):
1. Edit `tree-sitter-perl/grammar.js`
2. Regenerate the parser:
   ```bash
   cd tree-sitter-perl
   npx tree-sitter generate
   ```

#### For Pest (Rust parser):
1. Edit `crates/tree-sitter-perl-rs/src/grammar.pest`
2. Update AST nodes in `pure_rust_parser.rs`
3. Update the `build_node` method

### 2. Scanner Updates

If your feature requires scanner changes:

1. Identify the token type needed
2. Update the scanner interface in `scanner/mod.rs`
3. Implement in both C and Rust scanners
4. Add tests for the new tokens

### 3. Testing New Features

1. Add unit tests for the parser changes
2. Add integration tests showing real usage
3. Add corpus tests for tree-sitter compatibility
4. Run comparison tests to ensure consistency

### Example: Adding a New Operator

```rust
// 1. Update grammar.pest
operator = { 
    // existing operators...
    | "**"  // new exponentiation operator
}

// 2. Update AST builder
fn build_operator(pair: Pair<Rule>) -> Node {
    match pair.as_str() {
        "**" => Node::new("exponentiation_operator"),
        // other cases...
    }
}

// 3. Add tests
#[test]
fn test_exponentiation() {
    let cases = vec![
        "2 ** 3",
        "$x ** $y",
        "2 ** 3 ** 4",  // right associative
    ];
    
    for input in cases {
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
}
```

## LSP Development

### Adding LSP Features

To add new LSP capabilities:

1. **Implement the trait** in `crates/perl-parser/src/lsp.rs`:
   ```rust
   impl YourProvider for LanguageService {
       fn your_method(&self, params: YourParams) -> Result<YourResponse> {
           // Implementation
       }
   }
   ```

2. **Add the handler** in `crates/perl-parser/src/lsp_server.rs`:
   ```rust
   "textDocument/yourMethod" => {
       self.handle_your_method(request.params)
   }
   ```

3. **Update capabilities** in the initialize response

4. **Add tests** in `crates/perl-parser/tests/lsp_*_test.rs`

### Testing LSP Features

```bash
# Run LSP tests
cargo test -p perl-parser lsp

# Test manually with logging
RUST_LOG=debug perl-lsp --stdio --log

# Use the capabilities demo
cargo run -p perl-parser --example lsp_capabilities
```

## Code Style

### Rust Code

- Follow standard Rust conventions
- Use `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Add documentation comments for public APIs

### Code Quality Standards

The project maintains high code quality standards. Before committing:

1. **Format your code**
   ```bash
   cargo fmt --all
   ```

2. **Fix clippy warnings**
   ```bash
   cargo clippy --all -- -W clippy::all
   ```

3. **Follow Rust best practices**
   - Prefer `.first()` over `.get(0)` for accessing first element
   - Use `.push(char)` instead of `.push_str("x")` for single characters
   - Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
   - Avoid unnecessary `.clone()` on types that implement Copy
   - Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions
   - Use `format!()` directly without `&` when passing to functions expecting String
   - Replace `&mut Vec<T>` parameters with `&mut [T]` where possible

```rust
/// Parses a Perl source file and returns an AST.
/// 
/// # Arguments
/// * `input` - The Perl source code to parse
/// 
/// # Returns
/// * `Ok(Node)` - The parsed AST
/// * `Err(ParseError)` - If parsing fails
pub fn parse(input: &str) -> Result<Node, ParseError> {
    // implementation
}
```

### Commit Messages

Follow conventional commits format:

```
feat: add support for heredoc syntax
fix: handle escaped characters in strings
test: add tests for regex patterns
docs: update README with new features
refactor: simplify scanner state machine
perf: optimize string interpolation parsing
```

## Pull Request Process

1. **Fork and create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Write code
   - Add tests
   - Update documentation

3. **Run tests locally**
   ```bash
   cargo xtask test
   cargo xtask check --all
   ```

4. **Create a pull request**
   - Fill in the PR template
   - Link related issues
   - Describe your changes

5. **Address review feedback**
   - Make requested changes
   - Push updates to your branch
   - Re-request review when ready

### PR Checklist

- [ ] Tests pass locally
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventions
- [ ] PR description explains the changes

## Getting Help

- **Issues**: Check existing issues or create a new one
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: See CLAUDE.md for project-specific guidance

## Recognition

Contributors will be recognized in:
- The project README
- Release notes
- The contributors graph

Thank you for contributing to tree-sitter-perl!