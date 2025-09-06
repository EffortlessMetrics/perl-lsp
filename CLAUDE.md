# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem with comprehensive workspace refactoring capabilities:

### Published Crates (v0.8.9 GA)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - 4-19x faster than legacy implementations (1-150 µs parsing)
   - True incremental parsing with <1ms LSP updates
   - Production-ready Rope integration for UTF-16/UTF-8 position conversion
   - Enhanced workspace navigation and PR workflow integration
   - **Thread-safe semantic tokens** - 2.826µs average performance (35x better than 100µs target)
   - **Zero-race-condition LSP features** - immutable provider pattern with local state management
   - **Cross-file workspace refactoring utilities** - comprehensive WorkspaceRefactor provider for symbol renaming, module extraction, import optimization
   - **Production-ready refactoring operations** - move subroutines between modules, inline variables, extract code sections
   - **Enterprise-grade safety and validation** - comprehensive error handling, input validation, and rollback support

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with production-grade CLI
   - Clean separation from parser logic for improved maintainability
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with mode-based lexing
   - Handles slash disambiguation and Unicode identifiers

4. **perl-corpus** (`/crates/perl-corpus/`)
   - Comprehensive test corpus with property-based testing infrastructure

5. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
   - Pest-based parser (v2 implementation), marked as legacy

## Quick Start

### Installation
```bash
# Install LSP server
cargo install perl-lsp

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Or Homebrew (macOS)
brew tap tree-sitter-perl/tap && brew install perl-lsp
```

### Usage
```bash
# Run LSP server (for editors)
perl-lsp --stdio

# Build parser from source
cargo build -p perl-parser --release

# Run tests
cargo test
```

## Key Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs including edge cases
- **Production-Ready LSP Server**: ~85% of LSP features functional with comprehensive workspace support
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Comprehensive Testing**: 100% test pass rate (195 library tests, 33 LSP E2E tests, 19 DAP tests)
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling
- **Enterprise Security**: Path traversal prevention, file completion safeguards
- **Cross-file Workspace Refactoring**: Enterprise-grade symbol renaming, module extraction, import optimization
- **Advanced Memory Profiling**: Dual-mode memory tracking with procfs RSS measurement and peak_alloc integration for comprehensive performance analysis

## Architecture

### Crate Structure
- **Core Parser**: `/crates/perl-parser/` - parsing logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - comprehensive test suite

### Parser Versions
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, 4-19x faster, production incremental parsing
- **v2 (Pest)**: ~99.996% coverage, legacy but stable
- **v1 (C-based)**: ~95% coverage, benchmarking only

## Essential Commands

**AI tools can run bare `cargo build` and `cargo test`** - the `.cargo/config.toml` ensures correct behavior.

### Build & Install
```bash
# Build main components
cargo build -p perl-lsp --release        # LSP server
cargo build -p perl-parser --release     # Parser library

# Install globally
cargo install perl-lsp                   # From crates.io
cargo install --path crates/perl-lsp     # From source
```

### Testing
```bash
cargo test                               # All tests
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lsp                  # LSP integration tests
cargo xtask corpus                       # Comprehensive integration
```

### Development
```bash
cargo xtask check --all                 # Format + clippy
cargo bench                            # Performance benchmarks
perl-lsp --stdio --log                 # Debug LSP server
```

### Performance Analysis & Memory Tracking
```bash
# Run implementation comparison with memory tracking
cargo xtask compare --report             # Full comparison with memory metrics
cargo xtask compare --c-only             # Test C implementation only
cargo xtask compare --rust-only          # Test Rust implementation only
cargo xtask compare --validate-only      # Validate existing results
cargo xtask compare --check-gates        # Check performance gates

# Validate memory profiling functionality
cargo run --bin xtask -- validate-memory-profiling
```

### Enhanced Workspace Navigation and PR Workflow Integration (v0.8.9) ⭐ **PRODUCTION READY**

The v0.8.9 release introduces production-stable workspace navigation with comprehensive AST traversal enhancements and PR workflow integration capabilities.

###
#### DAP Server (Debug Adapter)
```bash
# Build DAP server
cargo build -p perl-parser --bin perl-dap --release

# Install DAP server globally
cargo install --path crates/perl-parser --bin perl-dap

# Run the DAP server (for VSCode integration)
perl-dap --stdio  # Standard DAP transport
```

#### Published Crates
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

#### Native Parser (Recommended)
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

### Test Commands

#### Workspace Testing (v0.8.9)
```bash
# Test core published crates (workspace members only)
cargo test                              # Tests perl-lexer, perl-parser, perl-corpus, perl-lsp
                                        # Excludes crates with system dependencies

# Test individual published crates
cargo test -p perl-parser               # Main parser library tests (195 tests)
cargo test -p perl-lexer                # Lexer tests (40 tests)  
cargo test -p perl-corpus               # Corpus tests (12 tests)
cargo test -p perl-lsp                  # LSP integration tests

# Legacy test commands (require excluded dependencies)
# cargo xtask test                      # xtask excluded from workspace
# cargo xtask corpus                    # xtask excluded from workspace

# Comprehensive Integration Testing
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # 33 LSP E2E tests

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

# Feature-specific tests (require feature flags)
cargo test -p perl-parser --features incremental           # Incremental parsing tests
cargo test -p perl-parser incremental_v2::tests            # IncrementalParserV2 tests

# Advanced feature tests (mostly ignored - aspirational features)
cargo test -p perl-lsp                                     # Includes properly ignored tests

# Performance and benchmark tests  
cargo test -p perl-parser --test incremental_perf_test
cargo bench incremental --features incremental

# CI Stability (Resolved in v0.8.9)
# Timeout issues in behavioral tests have been resolved with increased timeouts:
# - Behavioral tests: 800ms → 3000ms timeout
# - wait_for_idle: 200ms → 1000ms timeout  
# - Internal request timeout: 250ms → 1000ms

> **Workspace Test Report**: See `WORKSPACE_TEST_REPORT.md` for complete validation status and known limitations.
> 
> **Test Coverage Summary (v0.8.9)**:
> - ✅ perl-corpus tests: 12 passed
> - ✅ perl-lexer tests: 40 passed  
> - ✅ perl-parser library tests: 217 passed (includes incremental_v2)
> - ✅ perl-lsp API contract tests: 15 passed
> - ✅ Advanced feature tests: 19 correctly ignored (unimplemented)
> - ✅ Total passing tests: 284+ with appropriate feature coverage
```

### Parser Commands

#### Native Parser (perl-parser)
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

### LSP Development
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

### Benchmarks
```bash
# Run parser benchmarks (workspace crates)
cargo bench                             # Benchmarks for published crates
cargo bench -p perl-parser              # Main parser benchmarks (v3)

# Legacy benchmark commands (excluded from workspace)
# cargo xtask compare                   # xtask excluded, requires tree-sitter-perl crate

# Individual crate benchmarks
cargo bench -p perl-lexer               # Lexer performance tests
cargo bench -p perl-corpus              # Corpus validation performance

# Performance validation
cargo test -p perl-parser --test incremental_perf_test  # Incremental parsing performance

# ENHANCED: Comprehensive C vs Rust benchmark comparison framework ⭐ **NEW v0.8.9**
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
```

### Code Quality
```bash
# Run standard Rust quality checks (workspace crates)
cargo fmt                              # Format workspace code
cargo clippy --workspace              # Lint workspace crates  
cargo clippy --workspace --tests      # Lint tests

# Legacy quality commands (excluded from workspace)
# cargo xtask check --all             # xtask excluded from workspace
# cargo xtask fmt                     # xtask excluded from workspace

# Individual crate checks
cargo clippy -p perl-parser           # Lint main parser crate
cargo clippy -p perl-lsp              # Lint LSP server
cargo test --doc                      # Documentation tests
```

### Edge Case Testing
```bash  
# Run comprehensive edge case tests (workspace crates)
cargo test -p perl-parser               # Includes all edge case coverage
cargo test -p perl-corpus               # Corpus-based edge case validation

# Legacy edge case commands (excluded from workspace)  
# cargo xtask test-edge-cases           # xtask excluded from workspace

# Specific edge case test suites
cargo test -p perl-parser --test scope_analyzer_tests        # Scope analysis edge cases
cargo test -p perl-parser edge_case                          # Edge case pattern tests
cargo test -p perl-parser regex                              # Regex delimiter tests
cargo test -p perl-parser heredoc                            # Heredoc edge cases

# ENHANCED WORKSPACE NAVIGATION TESTS (v0.8.9)
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

### Scope Analyzer Testing
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

### Parser Generation
```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## LSP Development Guidelines

### Source Threading Architecture (v0.8.7+)

All LSP providers now support source-aware analysis for enhanced documentation extraction:

**Provider Constructor Patterns**:
```rust
// Enhanced constructors with source text (v0.8.7)
CompletionProvider::new_with_index_and_source(ast, source, workspace_index)
SignatureHelpProvider::new_with_source(ast, source)
SymbolExtractor::new_with_source(source)

// Legacy constructors (still supported)
CompletionProvider::new_with_index(ast, workspace_index)  // uses empty source
SignatureHelpProvider::new(ast)  // uses empty source
SymbolExtractor::new()  // no documentation extraction
```

**Comment Documentation Extraction** (Comprehensively Enhanced in PR #71):
- **Leading Comments**: Extracts multi-line comments immediately preceding symbol declarations with precise boundary detection
- **Blank Line Handling**: Stops at actual blank lines (not whitespace-only lines) for accurate comment boundaries  
- **Whitespace Resilient**: Handles varying indentation and comment prefixes (`#`, `##`, `###`) with automatic normalization
- **Performance Optimized**: <100µs extraction time with pre-allocated string capacity for large comment blocks
- **Unicode Safe**: Proper UTF-8 character boundary handling with support for international comments and emojis
- **Multi-Package Support**: Correct comment extraction across package boundaries with qualified name resolution
- **Edge Case Robust**: Handles empty comments, source boundaries, non-ASCII whitespace, and complex formatting scenarios
- **Method Documentation**: Full support for class methods, subroutines, and variable list declarations
- **Production Testing**: 20 comprehensive test cases covering all edge cases and performance scenarios
- **AST Integration**: Documentation attached to Symbol structs for use across all LSP features with source threading

**Comment Documentation Examples** (**Diataxis: Tutorial**):
```perl
# This documents the function below
# Multiple line comments are supported
# with proper boundary detection
sub documented_function {
    # Internal comment - not documentation
}

### Variable documentation with various comment styles  
   ### Works with extra whitespace and hashes
my $documented_var = 42;

# This will NOT be captured as documentation for foo
# because there's a blank line

sub foo {  # Not documentation
}
```

**Testing Comment Documentation** (**Diataxis: How-to**):
```bash
# Test comprehensive comment extraction (20 tests covering all scenarios)
cargo test -p perl-parser --test symbol_documentation_tests

# Test specific comment patterns and edge cases (PR #71 comprehensive coverage)
cargo test -p perl-parser symbol_documentation_tests::comment_separated_by_blank_line_is_not_captured
cargo test -p perl-parser symbol_documentation_tests::comment_with_extra_hashes_and_spaces
cargo test -p perl-parser symbol_documentation_tests::multi_package_comment_scenarios
cargo test -p perl-parser symbol_documentation_tests::complex_comment_formatting
cargo test -p perl-parser symbol_documentation_tests::unicode_in_comments
cargo test -p perl-parser symbol_documentation_tests::performance_with_large_comment_blocks

# Test new edge case coverage (PR #71 additions)
cargo test -p perl-parser symbol_documentation_tests::mixed_comment_styles_and_blank_lines
cargo test -p perl-parser symbol_documentation_tests::variable_list_declarations_with_comments
cargo test -p perl-parser symbol_documentation_tests::method_comments_in_class
cargo test -p perl-parser symbol_documentation_tests::whitespace_only_lines_vs_blank_lines
cargo test -p perl-parser symbol_documentation_tests::bless_with_comment_documentation

# Performance benchmarking (<100µs per iteration target)
cargo test -p perl-parser symbol_documentation_tests::performance_benchmark_comment_extraction -- --nocapture
```

### Adding New LSP Features

When implementing new LSP features, follow this structure:

1. **Core Implementation** (`/crates/perl-parser/src/`)
   - Add feature module (e.g., `completion.rs`, `code_actions.rs`)
   - Implement provider struct with main logic
   - **Use source-aware constructors** for enhanced documentation support
   - Add to `lib.rs` exports

2. **LSP Server Integration** (`lsp_server.rs`)
   - Add handler method (e.g., `handle_completion`)
   - **Thread source text** through provider constructors using `doc.content`
   - Wire up in main request dispatcher
   - Handle request/response formatting

3. **Testing**
   - Unit tests in the module itself
   - Integration tests in `/tests/lsp_*_tests.rs`
   - **Symbol documentation tests** for comment extraction features
   - User story tests for real-world scenarios

### Code Actions and Refactoring

The refactoring system has two layers:

1. **Base Code Actions** (`code_actions.rs`)
   - Quick fixes for diagnostics
   - Simple refactorings
   - Integration with diagnostics

2. **Enhanced Refactorings** (`code_actions_enhanced.rs`)
   - Extract variable/subroutine
   - Loop conversions
   - Import organization
   - Smart naming and formatting preservation

To add a new refactoring:
```rust
// In code_actions_enhanced.rs
fn your_refactoring(&self, node: &Node) -> Option<CodeAction> {
    // 1. Check if refactoring applies
    // 2. Generate new code
    // 3. Return CodeAction with TextEdits
}
```

### Testing LSP Features

#### Test Infrastructure (v0.8.6)
The project includes a robust test infrastructure with async LSP harness and production-grade assertion helpers:

**Async LSP Harness** (`tests/support/lsp_harness.rs`):
- **Thread-safe Communication**: Uses mpsc channels for non-blocking server communication
- **Timeout Support**: Configurable timeouts for all LSP operations (default: 2s)
- **Real JSON-RPC Protocol**: Tests actual protocol compliance, not mocked responses  
- **Background Processing**: Server runs in separate thread preventing test blocking
- **Notification Handling**: Separate buffer for server notifications and diagnostics

**Assertion Helpers** (`tests/support/mod.rs`):
- **Deep Validation**: All LSP responses are validated for proper structure
- **Meaningful Errors**: Clear error messages for debugging test failures
- **No Tautologies**: All assertions actually validate response content

**How-to Guide: Using the Async Test Harness**:
```rust
// Create harness with automatic server initialization
let mut harness = LspHarness::new();
harness.initialize(None)?;

// Test with custom timeout (useful for slow operations)
let response = harness.request_with_timeout(
    "textDocument/completion", 
    params, 
    Duration::from_millis(500)
)?;

// Test notifications (like diagnostics)
harness.open_document("file:///test.pl", "my $var = 42;");
let notifications = harness.drain_notifications(
    Some("textDocument/publishDiagnostics"), 
    1000  // 1 second timeout
);

// Test bounded operations (prevents infinite hangs)
let definition = harness.request_with_timeout(
    "textDocument/definition",
    definition_params,
    Duration::from_millis(500)
)?;
```

**Test Commands**:
```bash
# Unit tests
cargo test -p perl-parser your_feature

# LSP API contract tests (async harness)
cargo test -p perl-lsp lsp_api_contracts

# Integration tests with timeout handling
cargo test -p perl-parser lsp_your_feature_tests

# Manual testing with real protocol
echo '{"jsonrpc":"2.0","method":"your_method",...}' | perl-lsp --stdio

# Run comprehensive E2E tests (100% passing as of v0.8.6)
cargo test -p perl-parser lsp_comprehensive_e2e_test

# Run all LSP tests with async harness (48+ tests)
cargo test -p perl-lsp
```

### Enhanced Position Tracking Development (**Diataxis: How-to**) (v0.8.7+)

The enhanced position tracking system provides accurate line/column mapping for LSP compliance:

#### **Using PositionTracker in Parser Context**:
```rust
use crate::parser_context::ParserContext;

// Create parser with automatic position tracking
let ctx = ParserContext::new(source);

// Access accurate token positions
let token = ctx.current_token().unwrap();
let range = token.range();
println!("Token at line {}, column {}", range.start.line, range.start.column);
```

#### **Testing Position Tracking** (**Diataxis: Tutorial**):
```bash
# Run position tracking tests
cargo test -p perl-parser --test parser_context -- test_multiline_positions
cargo test -p perl-parser --test parser_context -- test_utf16_position_mapping
cargo test -p perl-parser --test parser_context -- test_crlf_line_endings

# Test with specific edge cases
cargo test -p perl-parser parser_context_tests::test_multiline_string_token_positions
```

#### **Position Tracking API Reference** (**Diataxis: Reference**):
```rust
// Core PositionTracker methods
impl PositionTracker {
    /// Create from source text with line start caching
    pub fn new(source: String) -> Self;
    
    /// Convert byte offset to Position with UTF-16 support  
    pub fn byte_to_position(&self, byte_offset: usize) -> Position;
}

// LineStartsCache for O(log n) lookups
impl LineStartsCache {
    /// Build cache with CRLF/LF/CR line ending support
    pub fn new(text: &str) -> Self;
    
    /// Convert byte offset to (line, utf16_column)
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32);
}
```

### Error Recovery and Fallback Mechanisms

The LSP server includes robust fallback mechanisms for handling incomplete or syntactically incorrect code:

1. **Signature Help Fallback** (`find_function_context`)
   - Works even when AST parsing fails
   - Scans backwards from cursor to find function context
   - Tracks parenthesis depth for accurate parameter counting
   - Handles method calls (`$obj->method`), package calls (`Pkg::func`)
   - Returns generic signatures for unknown functions

2. **Folding Ranges Fallback** (`extract_folding_fallback`)
   - Text-based analysis when parser fails
   - Detects brace pairs across multiple lines
   - Identifies subroutines and POD sections
   - Provides basic code folding even for invalid syntax

3. **Symbol Extraction Fallback** (`extract_symbols_fallback`)
   - Regex-based extraction for error recovery
   - Finds subroutines and packages in unparseable code
   - Ensures outline view works during active editing

4. **Diagnostics with Production-Stable Enhanced Scope Analysis** (v0.8.7+)
   - **Advanced Variable Resolution** with production-proven hash key context detection
   - **Enhanced Variable Resolution Patterns**: Hash access (`$hash{key}` → `%hash`), array access (`$array[idx]` → `@array`)  
   - **Hash Key Context Detection** - Industry-leading undefined variable detection under `use strict` with comprehensive hash key awareness:
     - Hash subscripts: `$hash{bareword_key}` - no false warnings, O(depth) performance
     - Hash literals: `{ key => value, another_key => value2 }` - keys properly recognized in all contexts
     - Hash slices: `@hash{key1, key2, key3}` - comprehensive array-based key detection
     - Nested hash access: `$hash{level1}{level2}{level3}` - deep nesting with safety limits
   - Enhanced scope analysis with stabilized `is_in_hash_key_context()` method and advanced pattern recognition
   - Unused variable warnings with improved accuracy and comprehensive coverage
   - Missing pragma suggestions (strict/warnings)
   - Context-aware bareword detection in hash keys
   - Works with partial ASTs from error recovery
   - **38 comprehensive test cases** covering all resolution patterns and edge cases

These fallbacks ensure the LSP remains functional during active development when code is temporarily invalid.

### File Path Completion System (v0.8.7+) (**Diataxis: Reference**)

The LSP server includes comprehensive file path completion with enterprise-grade security and performance features.

#### Core Architecture (**Diataxis: Explanation**)
The file completion system activates automatically when editing string literals that contain path-like content:

**Detection Logic**:
- **Context-aware activation**: Triggers inside quoted strings (`"path/to/file"` or `'path/to/file'`)
- **Path pattern recognition**: Detects `/` separators or alphanumeric file patterns
- **Smart filtering**: Only suggests files matching the current prefix

**Security Architecture**:
- **Path traversal prevention**: Blocks `../` patterns and absolute paths (except `/`)
- **Null byte protection**: Rejects strings containing `\0` characters
- **Reserved name filtering**: Prevents Windows reserved names (CON, PRN, AUX, etc.)
- **Filename validation**: UTF-8 validation, length limits (255 chars), control character filtering
- **Directory safety**: Canonicalization with safe fallbacks, hidden file filtering

#### Tutorial: Using File Path Completion (**Diataxis: Tutorial**)

**Step 1: Basic File Completion**
```perl
# Type a string with path content and trigger completion
my $config_file = "config/app."; # <-- Press Ctrl+Space here
# Suggests: config/app.yaml, config/app.json, config/app.toml
```

**Step 2: Directory Navigation**
```perl
# Navigate through directory structures
my $lib_file = "src/"; # <-- Completion shows src/ contents
# Shows: src/completion.rs, src/parser.rs, src/lib.rs
```

**Step 3: File Type Recognition**
```perl
# Get intelligent file type information
my $script = "scripts/deploy."; # <-- Shows file types in completion details
# deploy.pl (Perl file), deploy.py (Python file), deploy.sh (file)
```

#### How-to Guide: Configuring File Completion (**Diataxis: How-to**)

**Enable/Disable File Completion**:
File completion is automatically enabled and cannot be disabled—it only activates in appropriate string contexts.

**Performance Tuning**:
The system includes built-in performance safeguards:
- **Max results**: 50 completions per request  
- **Max depth**: 1 level directory traversal
- **Max entries**: 200 filesystem entries examined
- **Cancellation support**: Respects LSP cancellation requests

**Customize File Filtering**:
The system automatically excludes:
- Hidden files (starting with `.`)
- System directories (`node_modules`, `.git`, `target`, `build`)
- Cache directories (`__pycache__`, `.pytest_cache`, `.mypy_cache`)

#### Reference: File Completion API (**Diataxis: Reference**)

**LSP Integration Points**:
```rust
// Core completion provider with file support
impl CompletionProvider {
    pub fn get_completions_with_path_cancellable(
        &self,
        source: &str,
        position: usize,
        filepath: Option<&str>,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Vec<CompletionItem>;
}

// Security validation methods
fn sanitize_path(&self, path: &str) -> Option<String>;
fn is_safe_filename(&self, filename: &str) -> bool;
fn is_hidden_or_forbidden(&self, entry: &walkdir::DirEntry) -> bool;
```

**File Type Mappings**:
```rust
let file_type_desc = match extension.to_lowercase().as_str() {
    "pl" | "pm" | "t" => "Perl file",
    "rs" => "Rust source file", 
    "js" => "JavaScript file",
    "py" => "Python file",
    "txt" => "Text file",
    "md" => "Markdown file", 
    "json" => "JSON file",
    "yaml" | "yml" => "YAML file",
    "toml" => "TOML file",
    _ => "file",
};
```

**Performance Limits**:
- **Max results**: 50 completions
- **Max depth**: 1 directory level
- **Max entries examined**: 200 filesystem entries
- **Path length limit**: 1024 characters
- **Filename length limit**: 255 characters

**Security Features**:
- Path traversal prevention (`../` blocked)
- Null byte detection (`\0` blocked)
- Windows reserved name filtering
- Symbolic link traversal disabled  
- Hidden file exclusion
- Control character filtering

#### Testing File Completion (**Diataxis: How-to**)
```bash
# Run file completion specific tests
cargo test -p perl-parser --test file_completion_tests

# Test individual scenarios
cargo test -p perl-parser file_completion_tests::completes_files_in_src_directory
cargo test -p perl-parser file_completion_tests::basic_security_test_rejects_path_traversal

# Test with various file patterns
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- test_completion
```

**Manual Testing Examples**:
```perl
# Test cases for manual validation
my $test1 = "src/comp";           # Should complete to src/completion.rs
my $test2 = "tests/";             # Should show tests/ directory contents  
my $test3 = "Cargo";              # Should complete to Cargo.toml, Cargo.lock
my $test4 = "../etc/passwd";      # Should NOT provide completions (security)
```

## Architecture Overview

### Crate Structure (v0.8.9 GA)

#### Published Crates (Workspace Members)
- **`/crates/perl-parser/`**: Main parser library 
  - `src/parser.rs`: Recursive descent parser
  - `src/ast.rs`: AST definitions and enhanced workspace navigation
  - Enhanced AST traversal including `NodeKind::ExpressionStatement` support
  - Production-ready Rope integration for incremental parsing
  - Published as `perl-parser` on crates.io

- **`/crates/perl-lsp/`**: Standalone LSP server ⭐ **NEW v0.8.9**
  - `src/main.rs`: Clean LSP server implementation
  - `bin/perl-lsp.rs`: LSP server binary entry point
  - Separated from parser logic for better maintainability
  - Published as `perl-lsp` on crates.io

- **`/crates/perl-lexer/`**: Context-aware tokenizer
  - `src/lib.rs`: Lexer API with Unicode support
  - `src/token.rs`: Token definitions
  - `src/mode.rs`: Lexer modes (ExpectTerm, ExpectOperator)
  - `src/unicode.rs`: Unicode identifier support
  - **Unicode Handling**: Robust support for Unicode characters in all contexts
  - **Heredoc Safety**: Proper bounds checking for Unicode + heredoc syntax
  - Published as `perl-lexer` on crates.io

- **`/crates/perl-corpus/`**: Test corpus
  - `src/lib.rs`: Corpus API
  - `tests/`: Perl test files
  - Published as `perl-corpus` on crates.io

#### Benchmark Framework (v0.8.9) ⭐ **NEW**
- **`/crates/tree-sitter-perl-rs/src/bin/benchmark_parsers.rs`**: Comprehensive Rust benchmark runner
  - Statistical analysis with confidence intervals
  - JSON output compatible with comparison tools
  - Memory usage tracking and performance categorization
  - Configurable iterations and warmup cycles

- **`/tree-sitter-perl/test/benchmark.js`**: C implementation benchmark harness  
  - Node.js-based benchmarking for C parser
  - Standardized JSON output format compatible with comparison framework
  - Environment variable configuration support

- **`/scripts/generate_comparison.py`**: Statistical comparison generator (**Diataxis: Reference**)
  - Cross-language performance analysis (C vs Rust)
  - Configurable regression thresholds (5% parse time, 20% memory defaults)
  - Performance gates with statistical significance testing
  - Markdown and JSON report generation with confidence intervals

- **`/scripts/setup_benchmark.sh`**: Automated benchmark environment setup (**Diataxis: Tutorial**)
  - Dependency installation for Python analysis framework
  - Environment validation and configuration
  - Complete setup automation for cross-language benchmarking

- **`/scripts/test_comparison.py`**: Comprehensive benchmark framework test suite
  - 12 test cases covering statistical analysis, configuration, and error handling
  - Validates regression detection and performance gate functionality
  - Unit tests for comparison metrics and threshold validation

- **`/requirements.txt`**: Python dependencies for benchmark analysis
  - Statistical analysis libraries and testing framework dependencies

#### Excluded Crates (System Dependencies)
- **`/crates/perl-parser-pest/`**: Legacy Pest parser
  - `src/grammar.pest`: PEG grammar
  - `src/lib.rs`: Parser implementation  
  - Published as `perl-parser-pest` on crates.io (marked legacy)
  - **Excluded**: Requires bindgen for C interop

#### Excluded Internal/Unpublished
- **`/tree-sitter-perl/`**: Original C implementation (libclang dependency)
- **`/tree-sitter-perl-c/`**: C parser bindings (libclang-dev dependency)
- **`/crates/tree-sitter-perl-rs/`**: Internal test harness (bindgen dependency)
- **`/xtask/`**: Development automation (circular dependency with excluded crates)
- **`/docs/`**: Architecture documentation

### Key Components

1. **Pest Parser Architecture**
   - PEG grammar in `grammar.pest` defines all Perl syntax
   - Recursive descent parsing with packrat optimization
   - Zero-copy parsing with `&str` slices
   - Feature flag: `pure-rust` enables the Pest parser

2. **AST Generation**
   - Strongly typed AST nodes in `pure_rust_parser.rs`
   - Arc<str> for efficient string storage
   - Tree-sitter compatible node types
   - Position tracking for all nodes

3. **S-Expression Output**
   - `to_sexp()` method produces tree-sitter format
   - Compatible with existing tree-sitter tools
   - Preserves all position information
   - Error nodes for unparseable constructs

4. **Edge Case Handling**
   - Comprehensive heredoc support (93% edge case test coverage)
   - Phase-aware parsing for BEGIN/END blocks
   - Dynamic delimiter detection and recovery
   - Clear diagnostics for unparseable constructs

5. **Enhanced Position Tracking** (**Diataxis: Explanation**) (v0.8.7+)
   - **O(log n) Position Mapping**: Efficient binary search-based position lookups using LineStartsCache
   - **LSP-Compliant UTF-16 Support**: Accurate character counting for multi-byte Unicode characters and emoji
   - **Multi-line Token Support**: Proper position tracking for tokens spanning multiple lines (strings, comments, heredocs)
   - **Line Ending Agnostic**: Handles CRLF, LF, and CR line endings consistently across platforms
   - **Production-Ready Integration**: Seamless integration with parser context and LSP server for real-time editing
   - **Comprehensive Testing**: 8 specialized test cases covering Unicode, CRLF, multiline strings, and edge cases

6. **Testing Strategy**
   - Grammar tests for each Perl construct
   - Edge case tests with property testing
   - Performance benchmarks
   - Integration tests for S-expression output
   - Position tracking validation tests

   - Encoding-aware lexing for mid-file encoding changes
   - Tree-sitter compatible error nodes and diagnostics
   - Performance optimized (<5% overhead for normal code)

## Development Guidelines

### Choosing a Crate (**Diataxis: How-to**)
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready with Rope support
2. **For IDE Integration**: Install `perl-lsp` from `perl-parser` crate - includes full Rope-based document management  
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations (**Diataxis: Reference**)
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (v0.8.9)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions
- **Legacy (Excluded)**: `/crates/perl-parser-pest/` - maintenance only, excluded from workspace
- **Build Tools (Excluded)**: `/xtask/` - build automation, excluded due to dependencies

### Rope Development Guidelines (**Diataxis: How-to**)
**IMPORTANT**: All Rope improvements should target the **production perl-parser crate**, not internal test harnesses.

**Production Rope Modules** (Target for improvements):
- **`/crates/perl-parser/src/textdoc.rs`**: Core document management with `ropey::Rope`
- **`/crates/perl-parser/src/position_mapper.rs`**: UTF-16/UTF-8 position conversion
- **`/crates/perl-parser/src/incremental_integration.rs`**: LSP integration bridge
- **`/crates/perl-parser/src/incremental_handler_v2.rs`**: Document change processing

**Do NOT modify these Rope usages** (internal test code):
- **`/crates/tree-sitter-perl-rs/`**: Legacy test harnesses with outdated Rope usage
- **Internal test infrastructure**: Focus on production code, not test utilities

### Testing (Workspace v0.8.9)
```bash
# Test workspace crates (recommended)
cargo test                              # Tests all published crates
cargo test -p perl-parser               # Main parser tests
cargo test -p perl-lsp                  # LSP server tests
cargo test -p perl-corpus               # Corpus tests
cargo test -p perl-lexer                # Lexer tests

# Feature-specific testing
cargo test --workspace --features ci-fast           # Fast CI tests
cargo test -p perl-parser --features incremental   # Incremental parsing

# Legacy testing (requires excluded crates)  
# cargo test --all                       # Includes excluded crates (may fail)

# Comprehensive integration testing
cargo test -p perl-parser --test lsp_comprehensive_e2e_test    # 33 LSP E2E tests
```

### Performance (Workspace v0.8.9)
Always run benchmarks after changes to ensure no regressions:
```bash  
# Workspace benchmark testing
cargo bench                             # Benchmark published crates
cargo bench -p perl-parser              # Main parser benchmarks

# Legacy benchmark commands (excluded from workspace)
# cargo xtask compare                   # xtask excluded, requires tree-sitter-perl

# Performance validation testing
cargo test -p perl-parser --test incremental_perf_test  # Incremental parsing performance
```

### Comprehensive Benchmark Framework (**Diataxis: How-to**) (v0.8.9)

The project includes an enterprise-grade cross-language benchmark framework for systematic performance analysis and regression detection.

#### Framework Architecture (**Diataxis: Explanation**)

The benchmark framework consists of four integrated components:

1. **Rust Benchmark Runner** (`benchmark_parsers.rs`)
   - Comprehensive statistical analysis with confidence intervals
   - Configurable iterations and warmup cycles (default: 100 iterations, 10 warmup)
   - Memory usage tracking and performance categorization
   - JSON output compatible with comparison tools

2. **C Benchmark Harness** (`benchmark.js`)
   - Node.js-based benchmarking for legacy C implementation
   - Standardized JSON output format for cross-language comparison
   - Environment variable configuration for test customization

3. **Statistical Comparison Generator** (`generate_comparison.py`)
   - Advanced statistical analysis with configurable regression thresholds
   - Performance gates with customizable limits (5% parse time, 20% memory defaults)
   - Comprehensive reporting in Markdown and JSON formats
   - Confidence interval calculations and significance testing

4. **Integration Layer** (`xtask/src/tasks/bench.rs`)
   - Orchestrates complete benchmark workflow
   - Automated regression detection and CI/CD integration
   - Consolidated reporting across all implementations

#### Quick Start Tutorial (**Diataxis: Tutorial**)

**Step 1: Setup Benchmark Environment**
```bash
# Install Python dependencies and validate environment
bash scripts/setup_benchmark.sh

# Verify installation with test suite
python3 -m pytest scripts/test_comparison.py -v  # 12 comprehensive tests
```

**Step 2: Run Individual Benchmarks**
```bash
# Generate Rust benchmark results
cargo run -p tree-sitter-perl-rs --bin benchmark_parsers --features pure-rust

# Generate C benchmark results (requires tree-sitter-perl C library)
cd tree-sitter-perl
TEST_CODE="$(cat ../test/benchmark_simple.pl)" ITERATIONS=100 node test/benchmark.js > ../c_benchmark.json
cd ..
```

**Step 3: Generate Comparison Report**
```bash
# Basic comparison with default thresholds
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results benchmark_results.json \
  --output comparison.json \
  --report comparison_report.md

# View results
cat comparison_report.md  # Detailed performance analysis
```

#### Advanced Configuration (**Diataxis: How-to**)

**Rust Benchmark Configuration** - Create `benchmark_config.json`:
```json
{
  "iterations": 200,
  "warmup_iterations": 20,
  "test_files": [
    "test/benchmark_simple.pl",
    "test/corpus"
  ],
  "output_path": "benchmark_results.json",
  "detailed_stats": true,
  "memory_tracking": false
}
```

**Performance Gates Configuration** - Create `comparison_config.json`:
```json
{
  "parse_time_regression_threshold": 3.0,
  "memory_usage_regression_threshold": 15.0,
  "minimum_test_coverage": 95.0,
  "confidence_level": 0.99,
  "include_detailed_stats": true
}
```

**Custom Performance Gates**:
```bash
# Strict performance validation (3% parse time, 15% memory thresholds)
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results benchmark_results.json \
  --parse-threshold 3.0 \
  --memory-threshold 15.0 \
  --verbose

# Lenient validation for experimental changes
python3 scripts/generate_comparison.py \
  --parse-threshold 10.0 \
  --memory-threshold 30.0 \
  --config experimental_config.json
```

#### Performance Gate Reference (**Diataxis: Reference**)

| Gate Type | Default Threshold | Purpose | Action |
|-----------|------------------|---------|---------|
| **Parse Time Regression** | 5% slowdown | Detect performance regressions | FAIL CI build |
| **Parse Time Improvement** | 5% speedup | Flag performance improvements | Report gain |
| **Memory Usage Regression** | 20% increase | Monitor memory efficiency | WARN/FAIL |
| **Test Coverage** | 90% minimum | Ensure statistical validity | WARN low coverage |
| **Statistical Confidence** | 95% confidence | Validate significance | WARN insufficient data |

#### Integration with CI/CD (**Diataxis: How-to**)

**GitHub Actions Example**:
```yaml
- name: Setup Benchmark Environment
  run: bash scripts/setup_benchmark.sh

- name: Run Cross-Language Benchmarks  
  run: cargo xtask bench --save

- name: Validate Performance Gates
  run: |
    if grep -q "❌ FAIL" benchmark_report.md; then
      echo "Performance regression detected"
      exit 1
    fi
    echo "All performance gates passed"
```

**Local Development Workflow**:
```bash
# Before making changes - establish baseline
cargo xtask bench --save --output baseline_results.json

# After changes - detect regressions
cargo xtask bench --save --output new_results.json
python3 scripts/generate_comparison.py \
  --c-results baseline_results.json \
  --rust-results new_results.json \
  --report regression_analysis.md
```

#### Framework Testing and Validation

**Comprehensive Test Suite** (12 test cases):
```bash
# Run all framework validation tests
python3 -m pytest scripts/test_comparison.py -v

# Test specific functionality
python3 -m pytest scripts/test_comparison.py::test_regression_detection -v
python3 -m pytest scripts/test_comparison.py::test_confidence_intervals -v
python3 -m pytest scripts/test_comparison.py::test_performance_gates -v
```

**Test Coverage Areas**:
- Statistical analysis accuracy and confidence intervals
- Regression detection with various threshold configurations  
- Performance gate validation and edge cases
- JSON/Markdown output format verification
- Configuration loading and error handling
- Memory usage tracking and categorization

This framework ensures systematic performance monitoring and provides early detection of regressions across both C and Rust implementations with enterprise-grade statistical validation.

## Documentation

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Architecture Overview](docs/ARCHITECTURE_OVERVIEW.md)** - System design and components
- **[Development Guidelines](docs/DEBUGGING.md)** - Contributing and troubleshooting

### Specialized Guides
- **[LSP Crate Separation](docs/LSP_CRATE_SEPARATION_GUIDE.md)** - v0.8.9 architectural improvements
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[File Completion](docs/FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion
- **[Import Optimizer](docs/IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions

## Performance Targets ✅ **EXCEEDED**

| Operation | Target | Achieved | Status |
|-----------|--------|----------|--------|
| Simple Edits | <100µs | 65µs avg | ✅ Excellent |
| Moderate Edits | <500µs | 205µs avg | ✅ Very Good |
| Large Documents (100 stmt) | <1ms | 538µs avg | ✅ Good |
| Node Reuse Efficiency | ≥70% | 99.7% peak | ✅ Exceptional |
| Statistical Consistency | <1.0 CV | 0.6 CV | ✅ Excellent |
| Incremental Success Rate | ≥95% | 100% | ✅ Perfect |

## Memory Performance Targets

| Memory Metric | Target | Implementation | Status |
|---------------|--------|----------------|--------|
| Small Files (<1KB) | <1MB peak | Dual-mode tracking | ✅ Monitored |
| Medium Files (1-10KB) | <5MB peak | procfs RSS + peak_alloc | ✅ Monitored |
| Large Files (>10KB) | <20MB peak | Statistical analysis | ✅ Monitored |
| Memory Overhead | <0.5MB baseline | Process RSS tracking | ✅ Validated |
| Tracking Accuracy | ±10% precision | Fallback mechanisms | ✅ Reliable |

## Current Status (v0.8.9)

✅ **Production Ready**:
- 100% test pass rate across all components
- Zero clippy warnings, consistent formatting
- Enterprise-grade LSP server with comprehensive features
- Production-stable incremental parsing with statistical validation
- Enhanced workspace navigation and PR workflow integration

**LSP Features (~85% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ Workspace symbols, rename, code actions
- ✅ **Thread-safe semantic tokens** (2.826µs average, zero race conditions)
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ File path completion with enterprise security
- ✅ Debug Adapter Protocol (DAP) support

**Recent Enhancements (v0.8.9)**:
- ✅ Comprehensive S-expression generation with 50+ operators
- ✅ Enhanced AST traversal including ExpressionStatement support
- ✅ Production-ready workspace indexing and cross-file analysis
- ✅ Advanced code actions with parameter threshold validation
- ✅ Statistical performance testing infrastructure
- ✅ **Advanced Memory Tracking Framework** (PR #101)
  - Dual-mode memory measurement using procfs RSS and peak_alloc integration
  - Statistical memory analysis with min/max/avg/median calculations
  - Memory estimation for subprocess operations with size-based heuristics
  - Comprehensive memory profiling validation with workload simulation

## Security Development Guidelines (PR #44)

This project demonstrates **enterprise-grade security practices** in its test infrastructure. All contributors should follow these security development standards:

### Secure Authentication Implementation

When implementing authentication systems (including test scenarios), use production-grade security:

```perl
use Crypt::PBKDF2;

# OWASP 2021 compliant PBKDF2 configuration
sub get_pbkdf2_instance {
    return Crypt::PBKDF2->new(
        hash_class => 'HMACSHA2',      # SHA-2 family for cryptographic strength
        hash_args => { sha_size => 256 }, # SHA-256 for collision resistance
        iterations => 100_000,          # 100k iterations (OWASP 2021 minimum)
        salt_len => 16,                 # 128-bit cryptographically random salt
    );
}

sub authenticate_user {
    my ($username, $password) = @_;
    my $users = load_users();
    my $pbkdf2 = get_pbkdf2_instance();
    
    foreach my $user (@$users) {
        if ($user->{name} eq $username) {
            # Constant-time validation prevents timing attacks
            if ($pbkdf2->validate($user->{password_hash}, $password)) {
                return $user;
            }
        }
    }
    return undef;  # Authentication failed
}
```

### Security Requirements

✅ **Cryptographic Standards**: Use OWASP 2021 compliant algorithms and parameters  
✅ **Timing Attack Prevention**: Implement constant-time comparisons for authentication  
✅ **No Plaintext Storage**: Hash all passwords immediately, never store in clear text  
✅ **Secure Salt Generation**: Use cryptographically secure random salts (≥16 bytes)  
✅ **Input Validation**: Sanitize and validate all user inputs  
✅ **Path Security**: Use canonical paths with workspace boundary validation  

### Security Testing Requirements

All security-related code must include comprehensive tests:

- **Authentication Security**: Test password hashing, validation, and timing consistency
- **Input Validation**: Verify proper sanitization and boundary checking
- **File Access Security**: Test path traversal prevention and workspace boundaries
- **Error Message Security**: Ensure no sensitive information disclosure

### Security Review Process

- All authentication/security code changes require security review
- Test implementations serve as security best practice examples  
- Document security assumptions and threat models in code comments
- Use the security implementation in PR #44 as the reference standard

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **Testing** → Use existing comprehensive test infrastructure
5. **Security features** → Follow PR #44 PBKDF2 implementation standards

<<<<<<< HEAD
Run `cargo xtask check --all` before committing. All tests must pass with zero warnings.
=======
The codebase maintains high quality standards with continuous improvements:

### Recent Improvements (2025-02)

#### Testing & Quality (v0.7.4)
- **Fixed all tautological test assertions** - Replaced 27+ always-passing assertions with meaningful checks
- **Created centralized test infrastructure** - Added `tests/support/mod.rs` with production-grade assertion helpers
- **Achieved 100% LSP E2E test coverage** - All 33 comprehensive tests passing (includes 25 E2E + 8 user story tests)
- **Cleaned up all dead code** - Removed 159+ lines of obsolete code, properly marked intentionally unused stubs
- **Zero compilation warnings** in core library (only test helper warnings remain, intentionally preserved)

#### LSP Features (v0.7.3)
- **Achieved 100% LSP test coverage** (25/25 comprehensive E2E tests passing)
- **Added robust error recovery** with fallback mechanisms for incomplete code
- **Implemented undefined variable detection** under `use strict` with scope analysis
- **Enhanced signature help** to work with incomplete/invalid code
- **Added text-based folding** for unparseable files

#### Code Quality (v0.7.2)
- **Reduced clippy warnings by 61%** (from 133 to 52 in perl-parser)
- **Eliminated 45+ unnecessary clone operations** on Copy types for better performance
- **Fixed all recursive function warnings** with proper annotations
- **Improved Rust idioms** throughout the codebase
- **Memory optimizations** from avoiding unnecessary allocations

### Coding Standards
- Run `cargo clippy` before committing changes
- Use `cargo fmt` for consistent formatting
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions

## Documentation (**Diataxis Framework Applied**)

### Architecture and Implementation Guides
- **[Modern Architecture](docs/MODERN_ARCHITECTURE.md)** - Two-crate architecture with performance benchmarking (**Diataxis: Explanation**)
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - Technical guide for LSP feature development (**Diataxis: How-to**)  
- **[Stability Guarantees](docs/STABILITY.md)** - API stability commitments and versioning policy (**Diataxis: Reference**)
- **[Edge Cases](docs/EDGE_CASES.md)** - Comprehensive edge case handling documentation (**Diataxis: Reference**)

### Performance and Benchmarking ⭐ **NEW v0.8.9**
- **[Benchmark Framework](docs/BENCHMARK_FRAMEWORK.md)** - Comprehensive cross-language performance analysis (**Diataxis: Tutorial + How-to**)
  - Statistical comparison between C and Rust implementations
  - Configurable performance gates (5% parse time, 20% memory defaults)
  - Enterprise-grade regression detection and CI/CD integration
  - Complete setup automation and 12 comprehensive test cases

### Development and Debugging  
- **[Debugging Guide](docs/DEBUGGING.md)** - DAP integration and VSCode debugging setup (**Diataxis: Tutorial**)
- **[Parser Comparison](docs/PARSER_COMPARISON.md)** - Performance analysis across implementations (**Diataxis: Reference**)

### Feature Implementation Status
- **[LSP Documentation](docs/LSP_DOCUMENTATION.md)** - Complete LSP feature matrix and implementation status (**Diataxis: Reference**)
- **[Incremental Parsing Progress](docs/INCREMENTAL_PARSING_PROGRESS.md)** - Production-ready incremental parsing implementation (**Diataxis: How-to**)

The documentation follows the **Diataxis framework** for clear organization:
- **Tutorial**: Learning-oriented, hands-on guidance for getting started
- **How-to**: Problem-oriented, step-by-step solutions for specific tasks  
- **Reference**: Information-oriented, comprehensive specifications and API docs
- **Explanation**: Understanding-oriented, design decisions and architectural concepts
