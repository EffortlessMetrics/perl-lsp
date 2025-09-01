# Contributing to tree-sitter-perl

**Diataxis Framework Guide** - Comprehensive contribution guidelines using structured documentation principles

Thank you for your interest in contributing to tree-sitter-perl! This document provides complete guidelines for contributing to the project using the Diataxis documentation framework.

## 📋 Table of Contents

**Tutorials** (Learning-oriented):
- [First-Time Contributor Setup](#first-time-contributor-setup)
- [Making Your First Contribution](#making-your-first-contribution)

**How-to Guides** (Task-oriented):
- [Development Setup](#development-setup)  
- [Testing Guidelines](#testing-guidelines)
- [Adding New Features](#adding-new-features)
- [LSP Feature Development](#lsp-feature-development)
- [Performance Optimization](#performance-optimization)

**Reference** (Information-oriented):
- [Project Structure](#project-structure)
- [Code Style Standards](#code-style-standards)
- [API Compatibility](#api-compatibility)

**Explanation** (Understanding-oriented):
- [Architecture Decisions](#architecture-decisions)
- [Parser Design Principles](#parser-design-principles)
- [Testing Philosophy](#testing-philosophy)

---

## 📚 Tutorials (Learning-oriented)

### First-Time Contributor Setup

**Goal**: Get your development environment ready for your first contribution

#### Prerequisites
- Rust 1.89+ with 2024 edition support
- Git for version control
- Basic familiarity with Rust and parser concepts

#### Step-by-Step Setup
1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR-USERNAME/tree-sitter-perl
   cd tree-sitter-perl
   ```

2. **Install Development Tools**
   ```bash
   # Install required Rust components
   rustup component add rustfmt clippy
   
   # Install development dependencies
   cargo install cargo-nextest  # Fast test runner
   cargo install cargo-watch    # File watching
   ```

3. **Verify Installation**
   ```bash
   # Build all crates
   cargo build --all
   
   # Run basic tests
   cargo nextest run -p perl-parser
   
   # Check code formatting
   cargo fmt --all -- --check
   ```

4. **Run Your First Parse**
   ```bash
   # Test the parser on a simple Perl script
   echo 'my $x = 42; print $x;' > test.pl
   cargo run -p perl-parser --example parse_file test.pl
   ```

### Making Your First Contribution

**Goal**: Submit your first pull request successfully

#### Choose Your First Issue
1. **Good First Issues**: Look for `good-first-issue` label
2. **Documentation**: README improvements, typo fixes
3. **Tests**: Add test cases for existing functionality
4. **Small Features**: Add support for minor Perl constructs

#### Development Workflow
1. **Create Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes Following TDD**
   ```bash
   # 1. Write failing test
   cargo nextest run test_your_feature  # Should fail
   
   # 2. Implement feature
   # Edit relevant files...
   
   # 3. Make test pass
   cargo nextest run test_your_feature  # Should pass
   ```

3. **Quality Checks**
   ```bash
   # Format code
   cargo fmt --all
   
   # Fix linting issues  
   cargo clippy --all -- -W clippy::all
   
   # Run full test suite
   cargo xtask test
   ```

---

## 🛠️ How-to Guides (Task-oriented)

### Development Setup

**Reference**: Complete project structure and development environment

#### Project Structure (v0.8.8+)
```
tree-sitter-perl/
├── crates/                         # Published crates ecosystem
│   ├── perl-parser/                # Main parser & LSP server ⭐
│   │   ├── src/
│   │   │   ├── parser.rs           # Recursive descent parser
│   │   │   ├── lsp_server.rs       # LSP implementation  
│   │   │   └── ast.rs              # AST definitions
│   │   └── bin/perl-lsp.rs         # LSP server binary
│   ├── perl-lexer/                 # Context-aware tokenizer
│   │   └── src/
│   │       ├── lib.rs              # Lexer API
│   │       └── token.rs            # Token definitions
│   ├── perl-corpus/                # Test corpus and validation
│   │   ├── src/lib.rs              # Corpus API
│   │   └── tests/*.pl              # Test files
│   └── perl-parser-pest/           # Legacy Pest parser (deprecated)
├── docs/                           # Architecture documentation
├── xtask/                          # Development automation
├── benches/                        # Performance benchmarks  
└── tree-sitter-perl/              # Original C reference
```

#### Development Environment
```bash
# Build the native parser (recommended)
cargo build -p perl-lexer -p perl-parser

# Build LSP server
cargo build -p perl-parser --bin perl-lsp --release

# Build with all features
cargo build --all --all-features
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

### No New Ignored Tests Policy
We enforce a strict "no new ignored tests" policy. The CI guard (`ci/check_ignored.sh`) tracks the count of `#[ignore]` attributes and will fail if the count increases.

**Current baseline: 39 ignored tests**

To check current count:
```bash
./ci/check_ignored.sh
```

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

### CI-only Test Behavior

Some timing-sensitive LSP cancellation tests are ignored on CI and still run locally:

- Tests use `#[cfg_attr(ci, ignore = "...")]` to skip on CI
- CI sets `RUSTFLAGS="--cfg=ci"` in `.github/run_all_tests.sh`
- To simulate CI locally:

```bash
export RUSTFLAGS="--cfg=ci"
cargo test -p perl-parser --test lsp_cancel_test
```

This keeps CI signal high while preserving cancellation coverage for local runs.

#### Feature-Gated Tests (Aspirational Features)
Some tests are gated behind feature flags for functionality that's planned but not yet implemented:

```bash
# Test advanced constant pragma parsing
cargo test -p perl-parser --features constant-advanced

# Test qw delimiter variants
cargo test -p perl-parser --features qw-variants

# Test package-qualified subroutine resolution
cargo test -p perl-parser --features package-qualified

# Test next-gen error classification
cargo test -p perl-parser --features error-classifier-v2

# Test advanced LSP features
cargo test -p perl-parser --features lsp-advanced
```

These tests run nightly in CI to ensure they don't rot. You can manually trigger the nightly run via GitHub Actions UI.

### Writing Tests

When adding new tests:
1. **Prefer fixing over ignoring**: If a test fails, fix the underlying issue
2. **Use feature flags for aspirational features**: Instead of `#[ignore]`, use:
   ```rust
   #[cfg_attr(not(feature = "your-feature"), ignore = "Requires your-feature")]
   ```
3. **Document why**: If you must ignore a test, always provide a reason:
   ```rust
   #[ignore = "Parser doesn't yet support X - see issue #123"]
   ```

#### Lowering the Ignored Baseline

When you fix an ignored test:
1. Remove the `#[ignore]` attribute
2. Run `./ci/check_ignored.sh` to see the new count
3. Update `ci/ignored_baseline.txt` with the new (lower) number
4. Commit both changes together

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

## VSCode Extension Development

### Running Extension Locally

1. **Setup Development Environment**
   ```bash
   cd vscode-extension
   npm install
   npm run compile
   ```

2. **Launch Extension in Debug Mode**
   - Open VSCode in the `vscode-extension` directory
   - Press `F5` to launch a new VSCode instance with the extension loaded
   - The extension will use the development version

3. **Configure Local LSP Server**
   
   For testing with a local LSP server build:
   ```json
   // .vscode/settings.json in test workspace
   {
     "perl-lsp.serverPath": "/path/to/your/target/debug/perl-lsp",
     "perl-lsp.autoDownload": false
   }
   ```

4. **Debug Extension and Server Together**
   ```bash
   # Terminal 1: Build and run LSP with logging
   cargo build -p perl-parser --bin perl-lsp
   RUST_LOG=debug target/debug/perl-lsp --stdio --log
   
   # Terminal 2: Launch VSCode extension
   cd vscode-extension
   code .
   # Press F5 to debug
   ```

5. **Test Auto-Download Feature**
   - Remove local perl-lsp from PATH
   - Set `"perl-lsp.autoDownload": true`
   - Extension will download from GitHub releases

### Publishing Extension

1. **Update Version**
   ```bash
   cd vscode-extension
   npm version patch/minor/major
   ```

2. **Package Extension**
   ```bash
   npm run package
   # Creates perl-lsp-x.x.x.vsix
   ```

3. **Test VSIX Locally**
   ```bash
   code --install-extension perl-lsp-x.x.x.vsix
   ```

4. **Publish to Marketplace**
   - Automated via GitHub Actions on tag push
   - Manual: `vsce publish` (requires VSCE_PAT)

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

## Incremental Parsing Development

**Diataxis: How-to Guides** - Step-by-step instructions for working with incremental parsing

### Using Incremental Parsing in Your Application

The incremental parsing system provides high-performance document editing with subtree reuse. Here's how to integrate it:

#### Basic Usage

```rust
use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::IncrementalEdit;

// Create a new incremental document
let source = r#"
    my $x = 42;
    my $y = 100;
    print $x + $y;
"#.to_string();

let mut doc = IncrementalDocument::new(source)?;

// Apply an edit (change 42 to 99)
let edit = IncrementalEdit::new(
    20,  // start_byte
    22,  // old_end_byte
    "99".to_string(), // new_text
);

doc.apply_edit(edit)?;

// Check performance metrics
println!("Parse time: {:.2}ms", doc.metrics().last_parse_time_ms);
println!("Nodes reused: {}", doc.metrics().nodes_reused);
println!("Nodes reparsed: {}", doc.metrics().nodes_reparsed);
```

#### Advanced Usage - Multiple Edits

```rust
use perl_parser::incremental_edit::IncrementalEditSet;

let mut edits = IncrementalEditSet::new();

// Add multiple edits (processed in batch)
edits.add(IncrementalEdit::new(8, 10, "15".to_string()));
edits.add(IncrementalEdit::new(19, 21, "25".to_string()));

// Apply all edits atomically
doc.apply_edits(&edits)?;
```

#### LSP Integration Pattern

```rust
use perl_parser::incremental_integration::{DocumentParser, IncrementalConfig};

// Enable incremental parsing in LSP context
unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
let config = IncrementalConfig::default();

// Create document parser with incremental support
let mut parser = DocumentParser::new(initial_source, &config)?;

// Apply LSP text changes
let lsp_changes = vec![/* ... LSP TextDocumentContentChangeEvent objects ... */];
parser.apply_changes(&lsp_changes, &config)?;
```

### Testing Incremental Parsing Features

#### Unit Testing

```rust
#[test]
fn test_incremental_single_token() {
    let source = "my $x = 42;";
    let mut doc = IncrementalDocument::new(source.to_string()).unwrap();
    
    let edit = IncrementalEdit::new(8, 10, "99".to_string());
    doc.apply_edit(edit).unwrap();
    
    // Verify performance characteristics
    assert!(doc.metrics.nodes_reused > 0);
    assert!(doc.metrics.last_parse_time_ms < 1.0);
    assert!(doc.text().contains("99"));
}
```

#### Integration Testing with Async Harness

```rust
#[cfg(feature = "incremental")]
use serial_test::serial;

#[test]
#[serial]
fn test_lsp_incremental_editing() {
    unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
    
    let config = IncrementalConfig::default();
    let mut doc = DocumentParser::new(source, &config).unwrap();
    
    let start = std::time::Instant::now();
    doc.apply_changes(&changes, &config).unwrap();
    let elapsed = start.elapsed();
    
    // Performance assertions
    assert!(elapsed.as_millis() < 100);
    
    unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
}
```

### Performance Optimization Guidelines

#### Cache Effectiveness
- **Best performance**: Small, localized edits (single token changes)
- **Good performance**: Function-level modifications with unchanged structure
- **Moderate performance**: Large structural changes (still faster than full reparse)

#### Memory Management
- Default cache size: 1000 subtrees (configurable)
- Automatic LRU eviction prevents unbounded growth
- Arc<Node> sharing provides zero-copy reuse

#### Debugging Performance Issues

```rust
// Enable detailed metrics tracking
let metrics = doc.metrics();
println!("Cache hit rate: {:.1}%", 
    (metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64) * 100.0
);

// Identify parse bottlenecks
if metrics.last_parse_time_ms > 5.0 {
    println!("Consider full reparse fallback for large edits");
}
```

### Benchmark Development

Add incremental parsing benchmarks in `benches/incremental_benchmark.rs`:

```rust
use criterion::{BatchSize, Criterion, criterion_group};

fn bench_your_scenario(c: &mut Criterion) {
    let source = "your test source code here";
    
    c.bench_function("incremental your_scenario", |b| {
        b.iter_batched(
            || IncrementalDocument::new(source.to_string()).unwrap(),
            |mut doc| {
                let edit = IncrementalEdit::new(/* your edit parameters */);
                doc.apply_edit(edit).unwrap();
                black_box(doc.metrics.nodes_reused);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_your_scenario);
```

### Configuration Options

#### Environment Variables
- `PERL_LSP_INCREMENTAL=1`: Enable incremental parsing in LSP server
- `PERL_INCREMENTAL_DEBUG=1`: Enable debug logging for cache operations
- `PERL_INCREMENTAL_CACHE_SIZE=2000`: Override default cache size

#### Runtime Configuration
```rust
let config = IncrementalConfig {
    enabled: true,
    cache_size: 2000,
    timeout_ms: 1000, // Fallback timeout
    ..Default::default()
};
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