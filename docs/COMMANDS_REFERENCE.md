# Commands Reference (*Diataxis: Reference* - Complete command specifications)

*This reference provides all available commands for building, testing, and using the tree-sitter-perl ecosystem.*

## Installation Commands (*Diataxis: How-to Guide* - Step-by-step installation)

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

## Build Commands (*Diataxis: How-to Guide* - Development builds)

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

### xtask Exclusion Strategy (*Diataxis: Explanation* - Design decisions)
The xtask crate is excluded from the workspace to maintain clean builds while preserving advanced functionality:
- **Why excluded**: xtask depends on excluded crates (tree-sitter-perl-rs with libclang)
- **How to use**: Run from xtask directory: `cd xtask && cargo run <command>`
- **Benefits**: Workspace builds remain system-dependency-free
- **Advanced features**: Dual-scanner corpus comparison requires libclang-dev

## Test Commands

### Workspace Testing (v0.8.9)
```bash
# Test core published crates (workspace members only)
cargo test                              # Tests perl-lexer, perl-parser, perl-corpus, perl-lsp
                                        # Excludes crates with system dependencies

# Test individual published crates
cargo test -p perl-parser               # Main parser library tests (195 tests)
cargo test -p perl-lexer                # Lexer tests (40 tests)  
cargo test -p perl-corpus               # Corpus tests (12 tests)
cargo test -p perl-lsp                  # LSP integration tests

# Advanced test commands (excluded from workspace, run from xtask directory)
# cd xtask && cargo run test            # Advanced xtask test suite
# cd xtask && cargo run corpus          # Dual-scanner corpus comparison
```

### Comprehensive Integration Testing
```bash
# LSP E2E tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # 33 LSP E2E tests

# Symbol documentation tests (comment extraction)
cargo test -p perl-parser --test symbol_documentation_tests

# File completion tests
cargo test -p perl-parser --test file_completion_tests

# DAP tests
cargo test -p perl-parser --test dap_comprehensive_test
cargo test -p perl-parser --test dap_integration_test -- --ignored  # Full integration test

# Incremental parsing tests
cargo test -p perl-parser --test incremental_integration_test --features incremental
cargo test -p perl-parser --features incremental
cargo test -p perl-parser incremental_v2::tests            # IncrementalParserV2 tests

# Performance and benchmark tests  
cargo test -p perl-parser --test incremental_perf_test
cargo bench incremental --features incremental
```

### Enhanced Workspace Navigation Tests (v0.8.9)
```bash
# Test comprehensive AST traversal with ExpressionStatement support
cargo test -p perl-parser --test workspace_comprehensive_traversal_test

# Test enhanced code actions and refactoring
cargo test -p perl-parser code_actions_enhanced

# Test improved call hierarchy provider
cargo test -p perl-parser call_hierarchy_provider

# Test enhanced workspace indexing and symbol resolution
cargo test -p perl-parser workspace_index workspace_rename

# Test TDD basic functionality enhancements
cargo test -p perl-parser tdd_basic
```

### Thread-Aware Testing (*Diataxis: How-to Guide* - CI and constrained environment testing)

The testing infrastructure includes adaptive threading configuration that scales timeouts and concurrency based on system constraints. This ensures reliable test execution in both high-performance development environments and resource-constrained CI systems.

```bash
# CI environment testing with adaptive timeouts (recommended for GitHub Actions, GitLab CI)
RUST_TEST_THREADS=2 cargo test -p perl-lsp              # 15-second adaptive timeouts
RUST_TEST_THREADS=2 cargo test -p perl-parser           # Extended timeouts for LSP tests

# Single-threaded testing (maximum timeout extension for heavily constrained environments)
RUST_TEST_THREADS=1 cargo test --test lsp_comprehensive_e2e_test  # Up to 45-second timeouts

# Development environment (normal timeouts)
cargo test -p perl-lsp                                   # 5-second timeouts for >4 threads
cargo test                                               # Full workspace with standard timeouts

# Custom timeout override (bypasses adaptive timeouts)
LSP_TEST_TIMEOUT_MS=20000 cargo test -p perl-lsp        # Force 20-second timeouts regardless of threads
LSP_TEST_SHORT_MS=1000 cargo test -p perl-lsp           # Custom short timeout (default 500ms)

# Verbose thread debugging
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --nocapture
```

#### Thread Configuration Reference (*Diataxis: Reference* - Timeout scaling matrix)

| Thread Count | Base Timeout | Adaptive Sleep | Use Case |
|-------------|-------------|----------------|----------|
| ≤2 threads  | 15 seconds  | 3x multiplier  | CI/GitHub Actions |
| ≤4 threads  | 10 seconds  | 2x multiplier  | Constrained development |
| >4 threads  | 5 seconds   | 1x multiplier  | Full development workstations |

#### Thread-Aware Test Examples (*Diataxis: Tutorial* - Common testing patterns)

```bash
# GitHub Actions CI configuration
- name: Run LSP tests
  run: RUST_TEST_THREADS=2 cargo test -p perl-lsp
  timeout-minutes: 10

# Local development on limited hardware
RUST_TEST_THREADS=4 cargo test -p perl-parser --test lsp_comprehensive_e2e_test

# High-performance workstation (default behavior)
cargo test  # Uses all available threads, standard 5-second timeouts

# Debug timeout issues
RUST_LOG=debug LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test -p perl-lsp --test specific_test
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

### Core LSP Testing (*Diataxis: How-to Guide* - Development workflows)

```bash
# Run LSP tests with performance optimizations (v0.8.9+)
cargo test -p perl-parser lsp

# Run LSP integration tests with fast mode (99.5% timeout reduction)
LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp

# Run specific performance-sensitive tests
cargo test -p perl-lsp test_completion_detail_formatting
cargo test -p perl-lsp test_workspace_symbol_search

# Run formatting capability tests (robust across environments)
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test test_e2e_document_formatting
cargo test -p perl-lsp --test lsp_perltidy_test test_formatting_provider_capability

# Test LSP server manually
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio

# Run with incremental parsing enabled (production-ready feature)
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio

# Test incremental parsing with LSP protocol
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio < test_requests.jsonrpc

# Run with a test file
perl-lsp --stdio < test_requests.jsonrpc
```

### LSP Testing Environment Variables (*Diataxis: Reference* - Configuration options)

**LSP_TEST_FALLBACKS** (**NEW in v0.8.9**):
```bash
# Enable fast testing mode (reduces test timeouts by ~75%)
export LSP_TEST_FALLBACKS=1

# Optional external dependencies for enhanced features
export PERLTIDY_PATH="/usr/local/bin/perltidy"    # Custom perltidy location
export PERLCRITIC_PATH="/usr/local/bin/perlcritic" # Custom perlcritic location

# Performance characteristics in fallback mode:
# - Base timeout: 500ms (vs 2000ms)
# - Wait for idle: 50ms (vs 2000ms)  
# - Symbol polling: single 200ms attempt (vs progressive backoff)
# - Result: 99.5% faster test execution (60s+ → 0.26s for workspace tests)

# Use cases:
cargo test -p perl-lsp                    # Fast CI/development testing
LSP_TEST_FALLBACKS=1 cargo test --workspace  # Quick workspace validation
LSP_TEST_FALLBACKS=1 cargo check --workspace # Fast build verification
```

**PERL_LSP_INCREMENTAL**:
```bash
# Enable incremental parsing (production-ready)
export PERL_LSP_INCREMENTAL=1
perl-lsp --stdio

# Performance benefits:
# - <1ms LSP updates with 70-99% node reuse efficiency
# - Production-stable incremental parsing
# - Enterprise-grade workspace refactoring support
```

## Benchmark Commands

### Workspace Benchmarks (v0.8.9)
```bash
# Run parser benchmarks (workspace crates)
cargo bench                             # Benchmarks for published crates
cargo bench -p perl-parser              # Main parser benchmarks (v3)

# Individual crate benchmarks
cargo bench -p perl-lexer               # Lexer performance tests
cargo bench -p perl-corpus              # Corpus validation performance

# Performance validation
cargo test -p perl-parser --test incremental_perf_test  # Incremental parsing performance
```

### Comprehensive C vs Rust Benchmark Framework (v0.8.9)
```bash
# Run complete cross-language benchmark suite with statistical analysis
cargo xtask bench                       # Complete benchmark workflow with C vs Rust comparison

# Individual benchmark components
cargo run -p tree-sitter-perl-rs --bin benchmark_parsers --features pure-rust  # Rust parser benchmarks
cd tree-sitter-perl && node test/benchmark.js  # C implementation benchmarks

# Generate statistical comparison report with configurable thresholds
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results rust_benchmark.json \
  --output comparison.json \
  --report comparison_report.md

# Custom performance gates (5% parse time, 20% memory defaults)
python3 scripts/generate_comparison.py \
  --parse-threshold 3.0 \
  --memory-threshold 15.0 \
  --verbose

# Setup benchmark environment with all dependencies
bash scripts/setup_benchmark.sh

# Run benchmark validation tests (12 comprehensive test cases)
python3 -m pytest scripts/test_comparison.py -v

# Compare all three parsers with memory tracking
cargo xtask compare --report             # Full comparison with memory metrics and statistical analysis
cargo xtask compare --c-only             # Test C implementation only with memory tracking
cargo xtask compare --rust-only          # Test Rust implementation only with memory tracking
cargo xtask compare --validate-only      # Validate existing results without re-running
cargo xtask compare --check-gates        # Check performance gates with memory thresholds

# Memory profiling validation
cargo run --bin xtask -- validate-memory-profiling  # Test dual-mode memory measurement
```

## Code Quality Commands

### Workspace Quality Checks (v0.8.9)
```bash
# Run standard Rust quality checks (workspace crates)
cargo fmt                              # Format workspace code
cargo clippy --workspace              # Lint workspace crates  
cargo clippy --workspace --tests      # Lint tests

# Individual crate checks
cargo clippy -p perl-parser           # Lint main parser crate
cargo clippy -p perl-lsp              # Lint LSP server
cargo test --doc                      # Documentation tests

# Legacy quality commands (excluded from workspace)
# cargo xtask check --all             # xtask excluded from workspace
# cargo xtask fmt                     # xtask excluded from workspace
```

## Dual-Scanner Corpus Comparison (*Diataxis: How-to Guide* - Testing procedures)

### Running Dual-Scanner Corpus Tests (v0.8.9+)
```bash
# Prerequisites: Install libclang-dev for C scanner support
sudo apt-get install libclang-dev  # Ubuntu/Debian
brew install llvm                  # macOS

# Run dual-scanner corpus comparison (requires xtask excluded from workspace)
cd xtask && cargo run corpus                              # Default: compare both scanners
cd xtask && cargo run corpus -- --scanner both           # Explicit dual-scanner mode
cd xtask && cargo run corpus -- --scanner both --diagnose # With detailed analysis

# Individual scanner testing
cd xtask && cargo run corpus -- --scanner c              # C scanner (delegates to Rust)
cd xtask && cargo run corpus -- --scanner rust           # Rust scanner implementation  
cd xtask && cargo run corpus -- --scanner v3             # V3 parser only

# Diagnostic analysis (*Diataxis: Reference* - detailed comparison)
cd xtask && cargo run corpus -- --diagnose               # Analyze first failing test
cd xtask && cargo run corpus -- --test                   # Test current parser behavior

# Custom corpus path
cd xtask && cargo run corpus -- --path ../test/corpus    # Custom corpus directory
```

### Dual-Scanner Output Analysis (*Diataxis: Explanation* - Understanding results)
```bash
# Scanner mismatch tracking
# When using --scanner both, the system tracks:
# - Total corpus tests run
# - Tests passing both scanners  
# - Tests failing in either scanner
# - Scanner output mismatches (different S-expressions)

# Example output interpretation:
# 📊 Corpus Test Summary:
#    Total: 157
#    Passed: 142 ✅
#    Failed: 15 ❌
#    Scanner mismatches: 23  # C vs Rust differences

# 🔀 Scanner mismatches:
#    corpus_file.txt: test_case_name  # Specific mismatch location
```

### Structural Analysis Features (*Diataxis: Reference* - Analysis capabilities)
```bash
# The dual-scanner system provides:
# - Node count comparison between C and Rust scanners
# - Missing node detection (in C but not Rust output)
# - Extra node detection (in Rust but not C output)  
# - Normalized S-expression comparison (whitespace-independent)
# - Detailed structural diff output for debugging

# Example diagnostic output:
# 🔍 STRUCTURAL ANALYSIS:
# C scanner nodes: 42
# V3 scanner nodes: 41
# ❌ Nodes missing in V3 output:
#   - specific_node_type
# ➕ Extra nodes in V3 output:  
#   - different_node_type
```

### xtask corpus Command Reference (*Diataxis: Reference* - Complete command specification)

```bash
# Basic corpus command structure
cd xtask && cargo run corpus [OPTIONS]

# Command line options:
--path <PATH>              # Corpus directory path (default: ../c/test/corpus)
--scanner <SCANNER>        # Scanner type: c, rust, v3, both (default: both)
--diagnose                 # Run diagnostic analysis on first failing test
--test                     # Test current parser behavior with simple expressions

# Scanner type options:
c       # Use C tree-sitter scanner only (baseline for comparison)
rust    # Use Rust tree-sitter scanner only (PureRustPerlParser)
v3      # Use V3 native parser only (perl_parser::Parser)
both    # Compare C scanner vs Rust scanner (legacy testing - C now delegates to Rust)

# Prerequisites for dual-scanner mode:
# Ubuntu/Debian: sudo apt-get install libclang-dev
# macOS: brew install llvm
# Fedora: sudo dnf install clang-devel

# Exit codes:
# 0  - All tests passed, no scanner mismatches
# 1  - Test failures or scanner mismatches detected

# Output format:
# 📊 Corpus Test Summary:
#    Total: <number>         # Total corpus tests processed
#    Passed: <number> ✅     # Tests passing in all scanners
#    Failed: <number> ❌     # Tests failing in any scanner
#    Scanner mismatches: <number>  # Different outputs between scanners
#
# ❌ Failed Tests:           # List of failing tests
#    filename: test_name
#
# 🔀 Scanner mismatches:     # List of scanner differences
#    filename: test_name
```

### Corpus Test File Structure (*Diataxis: Reference* - Test format specification)

```
Test Case Name
================================================================================
source code here
----
(expected s_expression output here)

Next Test Case Name
================================================================================
more source code
----
(expected_output)
```

## Highlight Testing Commands (*Diataxis: Reference* - Tree-Sitter Highlight Test Runner)

### Basic Highlight Testing (*Diataxis: Tutorial* - Getting started with highlight tests)

```bash
# Prerequisites: Navigate to xtask directory for highlight testing
cd xtask

# Run all highlight tests with perl-parser AST integration
cargo run --no-default-features -- highlight

# Test specific highlight directory
cargo run --no-default-features -- highlight --path ../crates/tree-sitter-perl/test/highlight

# Test with specific scanner (for compatibility testing)
cargo run --no-default-features -- highlight --scanner v3
```

### Highlight Integration Testing (*Diataxis: How-to Guide* - Running comprehensive tests)

```bash
# Run perl-corpus highlight integration tests (4 comprehensive tests)
cargo test -p perl-corpus --test highlight_integration_test

# Individual integration test scenarios
cargo test -p perl-corpus highlight_integration_test::test_highlight_runner_integration     # Basic AST integration
cargo test -p perl-corpus highlight_integration_test::test_complex_highlight_constructs    # Complex Perl constructs  
cargo test -p perl-corpus highlight_integration_test::test_highlight_error_handling        # Edge case handling
cargo test -p perl-corpus highlight_integration_test::test_highlight_performance           # Performance validation

# Performance characteristics validation (<100ms for complex code)
cargo test -p perl-corpus highlight_integration_test::test_highlight_performance -- --nocapture
```

### Creating Highlight Test Fixtures (*Diataxis: How-to Guide* - Adding new test cases)

```bash
# Navigate to highlight test fixture directory
cd crates/tree-sitter-perl/test/highlight

# Create new highlight test file (follow existing naming conventions)
touch new_feature.pm

# Highlight test file format:
# Working highlight test examples
# 
# Simple variable assignment
# my $name = "John";
# # <- keyword  
# #    ^ punctuation.special
# #     ^ variable
# #            ^ string
# 
# Number operations  
# 42;
# # <- number
# 
# Use statement
# use strict;
# # <- keyword
# #   ^ type

# Supported highlight scopes mapped to perl-parser AST nodes:
# - keyword        → VariableDeclaration
# - punctuation.special → Variable (sigil mapping)
# - variable       → Variable
# - string         → string
# - number         → number
# - operator       → binary_+ (binary operations)
# - function       → SubDeclaration
# - type           → UseStatement
# - label          → HereDocEnd

# Test your new fixture
cd ../../../../xtask
cargo run --no-default-features -- highlight --path ../crates/tree-sitter-perl/test/highlight
```

### Highlight Test Runner Reference (*Diataxis: Reference* - Complete command specification)

```bash
# Command structure
cd xtask && cargo run --no-default-features -- highlight [OPTIONS]

# Command line options:
--path <PATH>         # Path to highlight test directory [default: c/test/highlight]
--scanner <SCANNER>   # Run with specific scanner [possible values: c, rust, both, v3]

# Default behavior:
# - Uses v3 parser (perl-parser native recursive descent)
# - Processes all .pm files in highlight directory
# - Maps highlight scopes to AST node kinds
# - Reports test results with pass/fail statistics

# Test fixture format requirements:
# - Files must have .pm extension
# - Comments starting with # define expected highlight scopes
# - Source code lines contain the Perl code to be highlighted
# - Empty lines separate test cases within a file
# - Position markers: ^ or <- indicate highlight scope location

# Performance characteristics:
# - ~540ms for 21 test cases (reasonable performance)
# - Integration with comprehensive perl-parser AST traversal
# - Secure path handling with WalkDir max_depth protection
```

### Highlight Test Architecture (*Diataxis: Explanation* - System design and integration)

The highlight test runner integrates deeply with the perl-parser AST generation system:

**Parser Integration**: 
- Uses `perl_parser::Parser` for native recursive descent parsing
- Leverages comprehensive AST node kind collection via `collect_node_kinds()`
- Maps tree-sitter highlight scopes to perl-parser NodeKind variants

**AST Node Mapping Strategy**:
```rust
// Highlight scope → AST NodeKind mapping
"keyword"           → NodeKind::VariableDeclaration
"punctuation.special" → NodeKind::Variable (Perl sigils)
"variable"          → NodeKind::Variable
"string"            → NodeKind::String
"number"            → NodeKind::Number  
"operator"          → NodeKind::Binary with specific operators (+, -, *, etc.)
"function"          → NodeKind::Subroutine
"type"              → NodeKind::Use
```

**Integration with perl-corpus Testing**:
- Comprehensive integration tests validate highlight runner functionality
- 4/4 integration tests passing with performance validation (<100ms)
- Tests cover basic constructs, complex scenarios, error handling, and performance

**Security and Path Handling**:
- Uses `WalkDir` with `max_depth(1)` for secure directory traversal
- Validates file extensions (`.pm` only)
- Proper error handling for parse failures and missing files

**Performance Optimizations**:
- Efficient AST traversal using manual recursion over NodeKind variants
- HashMap-based node counting for fast scope matching
- Progress indication with `indicatif` for user feedback

### Advanced Diagnostic Features (*Diataxis: Reference* - Analysis capabilities)

```bash
# Structural analysis when using --diagnose:
🔍 DIAGNOSTIC: test_name
Input Perl code:
```perl
source code being tested
```

📊 C scanner S-expression:
(program (expression_statement (number "1")))

📊 V3 scanner S-expression:  
(program (expression_statement (literal "1")))

🔍 STRUCTURAL ANALYSIS:
C scanner nodes: 15
V3 scanner nodes: 14
❌ Nodes missing in V3 output:
  - number
➕ Extra nodes in V3 output:
  - literal
```

## Scanner Architecture Testing (*Diataxis: How-to Guide* - Unified scanner validation)

The project uses a unified scanner architecture where both `c-scanner` and `rust-scanner` features use the same Rust implementation, with `CScanner` serving as a compatibility wrapper that delegates to `RustScanner`.

### Scanner Implementation Testing (*Diataxis: Reference* - Scanner validation commands)

```bash
# Test core Rust scanner implementation directly
cargo test -p tree-sitter-perl-rs --features rust-scanner

# Test C scanner wrapper (delegates to Rust implementation internally)
cargo test -p tree-sitter-perl-rs --features c-scanner

# Validate scanner delegation functionality
cargo test -p tree-sitter-perl-rs rust_scanner_smoke

# Test scanner state management and serialization
cargo test -p tree-sitter-perl-rs scanner_state
```

### Scanner Compatibility Validation (*Diataxis: How-to Guide* - Ensuring backward compatibility)

```bash
# Verify both scanner interfaces work correctly
cargo test -p tree-sitter-perl-rs --features rust-scanner,c-scanner

# Test C scanner API compatibility (should delegate to Rust without changes)
cargo test -p tree-sitter-perl-rs c_scanner::tests::test_c_scanner_delegates

# Performance testing (both scanners use same Rust implementation)
cargo bench -p tree-sitter-perl-rs --features rust-scanner
cargo bench -p tree-sitter-perl-rs --features c-scanner
```

### Scanner Build Configuration (*Diataxis: Reference* - Feature flag usage)

```bash
# Build with Rust scanner only (direct usage)
cargo build -p tree-sitter-perl-rs --features rust-scanner

# Build with C scanner wrapper (delegates to Rust internally)
cargo build -p tree-sitter-perl-rs --features c-scanner

# Build with both scanner interfaces available
cargo build -p tree-sitter-perl-rs --features rust-scanner,c-scanner
```

### Understanding Scanner Architecture (*Diataxis: Explanation* - Design rationale)

The unified scanner architecture provides:

- **Single Implementation**: Both `c-scanner` and `rust-scanner` features use the same Rust code
- **Backward Compatibility**: `CScanner` API unchanged, existing benchmark code works without modification  
- **Simplified Maintenance**: One scanner implementation instead of separate C and Rust versions
- **Consistent Performance**: All interfaces benefit from Rust implementation performance

## Edge Case Testing Commands

### Workspace Edge Case Tests (v0.8.9)
```bash  
# Run comprehensive edge case tests (workspace crates)
cargo test -p perl-parser               # Includes all edge case coverage
cargo test -p perl-corpus               # Corpus-based edge case validation

# Specific edge case test suites
cargo test -p perl-parser --test scope_analyzer_tests        # Scope analysis edge cases
cargo test -p perl-parser edge_case                          # Edge case pattern tests
cargo test -p perl-parser regex                              # Regex delimiter tests
cargo test -p perl-parser heredoc                            # Heredoc edge cases
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

## LSP Development Commands

### Testing Comment Documentation
```bash
# Test comprehensive comment extraction (20 tests covering all scenarios)
cargo test -p perl-parser --test symbol_documentation_tests

# Test specific comment patterns and edge cases
cargo test -p perl-parser symbol_documentation_tests::comment_separated_by_blank_line_is_not_captured
cargo test -p perl-parser symbol_documentation_tests::comment_with_extra_hashes_and_spaces
cargo test -p perl-parser symbol_documentation_tests::multi_package_comment_scenarios
cargo test -p perl-parser symbol_documentation_tests::complex_comment_formatting
cargo test -p perl-parser symbol_documentation_tests::unicode_in_comments
cargo test -p perl-parser symbol_documentation_tests::performance_with_large_comment_blocks

# Performance benchmarking (<100µs per iteration target)
cargo test -p perl-parser symbol_documentation_tests::performance_benchmark_comment_extraction -- --nocapture
```

### Testing Position Tracking
```bash
# Run position tracking tests
cargo test -p perl-parser --test parser_context -- test_multiline_positions
cargo test -p perl-parser --test parser_context -- test_utf16_position_mapping
cargo test -p perl-parser --test parser_context -- test_crlf_line_endings

# Test with specific edge cases
cargo test -p perl-parser parser_context_tests::test_multiline_string_token_positions
```

### Testing File Completion
```bash
# Run file completion specific tests
cargo test -p perl-parser --test file_completion_tests

# Test individual scenarios
cargo test -p perl-parser file_completion_tests::completes_files_in_src_directory
cargo test -p perl-parser file_completion_tests::basic_security_test_rejects_path_traversal

# Test with various file patterns
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- test_completion
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
1. Run benchmarks before and after changes: `cargo bench`
2. Use comprehensive benchmark framework: `cargo xtask bench`
3. Use `cargo xtask compare --report` to compare implementations with memory tracking
4. Check performance gates with statistical analysis: `python3 scripts/generate_comparison.py`
5. Check for performance gates: `cargo xtask compare --check-gates`
6. Monitor incremental parsing performance: `cargo test -p perl-parser --test incremental_perf_test`
7. Validate memory profiling: `cargo run --bin xtask -- validate-memory-profiling`
8. Monitor memory usage patterns with statistical analysis
9. Use dual-mode memory measurement (procfs RSS + peak_alloc) for accurate profiling
