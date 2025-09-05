# Commands Reference

## Build Commands

### LSP Server
```bash
# Quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Homebrew (macOS)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# Build from source
cargo build -p perl-lsp --release

# Install globally
cargo install --path crates/perl-lsp

# Run the LSP server
perl-lsp --stdio  # For editor integration
perl-lsp --stdio --log  # With debug logging
```

### DAP Server (Debug Adapter)
```bash
# Build DAP server
cargo build -p perl-parser --bin perl-dap --release

# Install DAP server globally
cargo install --path crates/perl-parser --bin perl-dap

# Run the DAP server (for VSCode integration)
perl-dap --stdio  # Standard DAP transport
```

### Published Crates
```bash
# Install from crates.io
cargo install perl-lsp                     # LSP server
cargo add perl-parser                      # As library dependency
cargo add perl-corpus --dev                # For testing

# Build from source
cargo build -p perl-parser --release
cargo build -p perl-lexer --release
cargo build -p perl-corpus --release
cargo build -p perl-parser-pest --release  # Legacy
```

### Native Parser (Recommended)
```bash
# Build the lexer and parser
cargo build -p perl-lexer -p perl-parser

# Build with incremental parsing support
cargo build -p perl-parser --features incremental

# Build in release mode
cargo build -p perl-lexer -p perl-parser --release

# Build with incremental parsing in release mode
cargo build -p perl-parser --features incremental --release

# Build everything
cargo build --all
```

## Workspace Configuration (v0.8.9+)

The workspace uses an exclusion strategy to ensure reliable builds across all platforms:

```bash
# Workspace tests (production crates only)
cargo test  # Tests perl-parser, perl-lsp, perl-lexer, perl-corpus

# Check workspace configuration
cargo check  # Should build cleanly without system dependencies

# Workspace status report (see WORKSPACE_TEST_REPORT.md)
# - Excludes tree-sitter-perl-c (requires libclang/system dependencies)
# - Excludes example crates with feature conflicts 
# - Focuses on published crate stability
```

### Workspace Architecture Benefits
- **Clean Builds**: No system dependency failures (libclang, parser.c)
- **CI Stability**: Consistent test results across platforms
- **Production Focus**: Tests only published crate APIs
- **Platform Independence**: Works without tree-sitter C toolchain

## Test Commands

```bash
# Run all workspace tests
cargo xtask test

# Run corpus tests (main integration tests)
cargo xtask corpus

# Run corpus tests with diagnostics (shows first failure in detail)
cargo xtask corpus --diagnose

# Run specific test suite
cargo xtask test --suite unit
cargo xtask test --suite integration

# Run a single test
cargo test test_name

# Test pure Rust parser
cargo test --features pure-rust

# Run LSP tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test

# Run import optimizer tests
cargo test -p perl-parser --test import_optimizer_tests

# Run symbol documentation tests (comment extraction)
cargo test -p perl-parser --test symbol_documentation_tests

# Run file completion tests
cargo test -p perl-parser --test file_completion_tests

# Run DAP tests
cargo test -p perl-parser --test dap_comprehensive_test
cargo test -p perl-parser --test dap_integration_test -- --ignored  # Full integration test

# Run incremental parsing tests
cargo test -p perl-parser --test incremental_integration_test --features incremental

# Run all incremental parsing tests with feature flag
cargo test -p perl-parser --features incremental

# Run IncrementalParserV2 tests specifically
cargo test -p perl-parser incremental_v2::tests

# Run incremental performance tests
cargo test -p perl-parser --test incremental_perf_test

# Benchmark incremental parsing performance
cargo bench incremental --features incremental

# CONCURRENCY-CAPPED TEST COMMANDS (recommended for stability)
# Quick capped test (2 threads)
cargo t2

# Capped tests with preflight system checks
./scripts/test-capped.sh

# Capped E2E tests with resource gating
./scripts/test-e2e-capped.sh

# Manual capped test run
RUST_TEST_THREADS=2 cargo test -- --test-threads=2

# Container-isolated tests (hard resource limits)
docker-compose -f docker-compose.test.yml up rust-tests
docker-compose -f docker-compose.test.yml up rust-e2e-tests
docker-compose -f docker-compose.test.yml up rust-lsp-tests
```

## Parser Commands

### Native Parser (perl-parser)
```bash
# Parse a Perl file (create a simple wrapper first)
# The v3 parser is a library - use it programmatically or via examples:

# Test regex patterns including m!pattern!
cargo run -p perl-parser --example test_regex

# Test comprehensive edge cases
cargo run -p perl-parser --example test_edge_cases

# Test all edge cases (shows coverage)
cargo run -p perl-parser --example test_more_edge_cases

# Test LSP capabilities demo
cargo run -p perl-parser --example lsp_capabilities
```

## LSP Development Commands

```bash
# Run LSP tests
cargo test -p perl-parser lsp

# Test LSP server manually
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio

# Run with incremental parsing enabled (production-ready feature)
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio

# Test incremental parsing with LSP protocol
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio < test_requests.jsonrpc

# Run with a test file
perl-lsp --stdio < test_requests.jsonrpc
```

## Benchmark Commands

```bash
# Run all parser benchmarks
cargo bench

# Run v2 parser benchmarks
cargo bench --features pure-rust

# Run v3 parser benchmarks
cargo bench -p perl-parser

# Compare all three parsers
cargo xtask compare
```

## Code Quality Commands

```bash
# Run all checks (formatting + clippy)
cargo xtask check --all

# Format code
cargo xtask fmt

# Run clippy
cargo xtask check --clippy
```

## Edge Case Testing Commands

```bash
# Run comprehensive edge case tests
cargo xtask test-edge-cases

# Run with performance benchmarks
cargo xtask test-edge-cases --bench

# Generate coverage report
cargo xtask test-edge-cases --coverage

# Run specific edge case test
cargo xtask test-edge-cases --test test_dynamic_delimiters

# Run scope analyzer tests specifically
cargo test -p perl-parser --test scope_analyzer_tests
```

## Scope Analyzer Testing

```bash
# Run all scope analyzer tests (38 comprehensive tests)
cargo test -p perl-parser --test scope_analyzer_tests

# Test enhanced variable resolution patterns
cargo test -p perl-parser scope_analyzer_tests::test_hash_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_array_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_complex_variable_patterns

# Test hash key context detection
cargo test -p perl-parser scope_analyzer_tests::test_hash_key_context_detection
```

## Parser Generation Commands

```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## Common Development Tasks

### Adding a New Perl Feature
1. Update `src/grammar.pest` with new syntax rules
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update `build_node()` method to handle new constructs
4. Add tests in `tests/` directory
5. Run tests: `cargo test --features pure-rust`
6. Run benchmarks: `cargo bench --features pure-rust`

### Debugging Parse Failures
1. Use `cargo xtask corpus --diagnose` for detailed error info
2. For Pest parser: Check parse error messages which show exact location
3. Use `cargo xtask parse-rust file.pl --ast` to see AST structure

### Performance Optimization
1. Run benchmarks before and after changes
2. Use `cargo xtask compare` to compare implementations
3. Check for performance gates: `cargo xtask compare --check-gates`