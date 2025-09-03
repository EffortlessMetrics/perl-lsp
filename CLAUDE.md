# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation and Enhanced Workspace Navigation
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md) for guarantees

## Project Overview

This repository contains **four published crates** forming a complete Perl parsing ecosystem:

### Published Crates (v0.8.8 GA)

#### 1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
- Native recursive descent parser with operator precedence
- **~100% Perl 5 syntax coverage** with ALL edge cases handled
- **4-19x faster** than legacy implementations (1-150 µs parsing)
- **True incremental parsing** with Rope-based document management and subtree reuse for <1ms LSP updates
- **Production-ready Rope integration** for UTF-16/UTF-8 position conversion and line ending support
- **Enhanced token position tracking** - O(log n) performance with LSP-compliant UTF-16 position mapping (PR #53)
- **Enhanced comment documentation extraction** - comprehensive leading comment parsing with UTF-8 safety and performance optimization (PR #71)
- **Source-aware symbol analysis** - full source text threading through LSP features for better context and documentation
- Tree-sitter compatible output
- Includes LSP server binary (`perl-lsp`) with full Rope-based document state

#### 2. **perl-lexer** (`/crates/perl-lexer/`)
- Context-aware tokenizer
- Mode-based lexing (ExpectTerm, ExpectOperator)
- Handles slash disambiguation at lexing phase
- Zero dependencies
- Used by perl-parser

#### 3. **perl-corpus** (`/crates/perl-corpus/`)
- Comprehensive test corpus
- Property-based testing infrastructure
- Edge case collection
- Used for parser validation
- Feature: `ci-fast` for conditional test execution

#### 4. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
- Pest-based parser (v2 implementation)
- ~99.995% Perl 5 coverage
- Marked as legacy - use perl-parser instead
- Kept for migration/comparison

### LSP Server (`perl-lsp` binary) ✅ **PRODUCTION READY**
- **~85% of LSP features actually work** (all advertised capabilities are fully functional, major reliability improvements in v0.8.9 with enhanced workspace navigation and PR workflow integration)
- **Full Rope-based document management** for efficient text operations and UTF-16/UTF-8 position conversion
- **Fully Working Features (v0.8.8 - Enhanced Bless Parsing and Workspace Navigation)**: 
  - ✅ **Advanced syntax checking and diagnostics** with breakthrough hash key context detection:
    - Hash subscripts: `$hash{bareword_key}` - correctly recognized as legitimate
    - Hash literals: `{ key => value, another_key => value2 }` - all keys properly identified
    - Hash slices: `@hash{key1, key2, key3}` - array-based key detection with full coverage
    - Nested structures: `$hash{level1}{level2}{level3}` - deep nesting handled correctly
    - Performance optimized with O(depth) complexity and safety limits
  - ✅ **Production-stable scope analyzer** with `is_in_hash_key_context()` method - now proven in production with O(depth) performance
  - ✅ **Enhanced S-expression format** with complete tree-sitter compatibility (v0.8.8):
    - Program nodes use tree-sitter format: (source_file) instead of (program)  
    - Variable nodes use proper tree-sitter structure: (scalar (varname)), (array (varname))
    - Number nodes simplified to (number) format without value embedding
    - **Enhanced FunctionCall nodes** - special handling for `bless` and built-in functions with proper tree-sitter structure
    - **Complete bless parsing support** - all 12 bless parsing tests passing with correct AST generation
    - Enhanced subroutine nodes with proper field labels and declaration wrappers
  - ✅ **Complete AST compatibility** for subroutine declarations, signature parsing, and method structures
  - ✅ **Improved corpus test compatibility** - enhanced S-expression generation for tree-sitter integration
  - ✅ **Type Definition and Implementation Providers** for blessed references and ISA relationships
  - ✅ **Incremental parsing with subtree reuse** - <1ms real-time editing performance
  - ✅ **Enhanced code completion** (variables, 150+ built-ins, keywords, **file paths**) with comprehensive comment-based documentation (PR #71)
  - ✅ **File path completion in strings** with comprehensive security and performance safeguards:
    - **Security Features**: Path traversal prevention, null byte detection, safe filename validation
    - **Performance Limits**: 50 max results, controlled filesystem traversal, cancellation support
    - **Cross-platform Support**: Windows/Unix path handling, reserved name checking
    - **Smart Context Detection**: Auto-activates in string literals with path-like content
    - **File Type Recognition**: Perl (.pl, .pm, .t), Rust (.rs), JavaScript (.js), Python (.py), and more
  - ✅ **Enhanced hover information** with robust comment documentation extraction across blank lines and advanced source-aware providers
  - ✅ Go-to-definition with DeclarationProvider
  - ✅ Find references (workspace-wide)
  - ✅ **Document highlights** - comprehensive variable occurrence tracking with enhanced expression statement support and improved symbol extraction
  - ✅ Document symbols and outline with enhanced documentation and complete AST traversal
  - ✅ Document/range formatting (Perl::Tidy)
  - ✅ Folding ranges with text fallback
  - ✅ **Workspace symbols** - search across files with enhanced symbol extraction including `ExpressionStatement` nodes (IMPROVED v0.8.8)
  - ✅ **Rename symbol** - cross-file for `our` vars (NEW)
  - ✅ **Code actions** - quick fixes, perltidy (NEW)
  - ✅ **Semantic tokens** - enhanced highlighting (NEW)
  - ✅ **Inlay hints** - parameter names, types (NEW)
  - ✅ **Document links** - module navigation (NEW)
  - ✅ **Selection ranges** - smart selection (NEW)
  - ✅ **On-type formatting** - auto-indent (NEW)
  - ✅ **Pull diagnostics** - LSP 3.17 support (v0.8.5)
  - ✅ **Type hierarchy** - class/role relationships (v0.8.5)
  - ✅ **Execute command** - Perl::Critic, perltidy, refactorings (v0.8.5)
  - ✅ **Type definition** - blessed references, ISA relationships (v0.8.6)
  - ✅ **Implementation** - class/method implementations (v0.8.6)
- **Partial Implementations** (not advertised):
  - ⚠️ Code lens (~20% functional)
  - ⚠️ Call hierarchy (~15% functional)
- **Debug Adapter Protocol (DAP)** ✅ **BETA**:
  - ✅ **Basic debugging flow** - launch, attach, disconnect
  - ✅ **Breakpoint management** - set, clear, conditional breakpoints
  - ✅ **Step controls** - step in, step out, step over, continue, pause
  - ✅ **Stack inspection** - stack frames, local scopes, variable inspection
  - ✅ **Expression evaluation** - evaluate expressions in debugger context
  - ✅ **Perl debugger integration** - uses built-in `perl -d` debugger
  - ✅ **DAP protocol compliance** - works with VSCode and DAP-compatible editors
- **Test Coverage**: ✅ **EXCELLENT** - 100% pass rate achieved with comprehensive PR workflow integration, Library Tests: 195/195 passing, LSP E2E: 33/33 tests passing, DAP Tests: 19/19 passing, Corpus Tests: 12/12 passing
- **Performance**: <50ms for all operations
- **Architecture**: Contract-driven with `lsp-ga-lock` feature for conservative releases
- Works with VSCode, Neovim, Emacs, Sublime, and any LSP-compatible editor
- **See `LSP_ACTUAL_STATUS.md` for complete feature status**

## Default Build Configuration

The project includes `.cargo/config.toml` which automatically configures:
- Optimized dev builds (`opt-level = 1`) for parser testing
- Full release optimization (`lto = true`) for benchmarks  
- Automatic backtraces (`RUST_BACKTRACE=1`)
- Sparse registry protocol for faster updates

**AI tools can run bare `cargo build` and `cargo test` commands** - the configuration ensures correct behavior.

## Key Commands

### Build Commands

#### LSP Server
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

# Build in release mode
cargo build -p perl-lexer -p perl-parser --release

# Build everything
cargo build --all
```

### Test Commands
```bash
# Run all tests
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

# Run symbol documentation tests (comment extraction)
cargo test -p perl-parser --test symbol_documentation_tests

# Run file completion tests
cargo test -p perl-parser --test file_completion_tests

# Run DAP tests
cargo test -p perl-parser --test dap_comprehensive_test
cargo test -p perl-parser --test dap_integration_test -- --ignored  # Full integration test

> **Heads-up for wrappers:** Don't pass shell redirections like `2>&1` as argv.
> If you need them, run through a real shell (`bash -lc '…'`) or wire stdio directly.
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

# Run with a test file
perl-lsp --stdio < test_requests.jsonrpc
```

### Benchmarks
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

### Code Quality
```bash
# Run all checks (formatting + clippy)
cargo xtask check --all

# Format code
cargo xtask fmt

# Run clippy
cargo xtask check --clippy
```

### Edge Case Testing
```bash
# Run comprehensive edge case tests
cargo xtask test-edge-cases

# Run with performance benchmarks
cargo xtask test-edge-cases --bench

# Generate coverage report
cargo xtask test-edge-cases --coverage

# Run specific edge case test
cargo xtask test-edge-cases --test test_dynamic_delimiters
```

### Parser Generation
```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## LSP Development Guidelines

### Adding New LSP Features

When implementing new LSP features, follow this structure:

1. **Core Implementation** (`/crates/perl-parser/src/`)
   - Add feature module (e.g., `completion.rs`, `code_actions.rs`)
   - Implement provider struct with main logic
   - Add to `lib.rs` exports

2. **LSP Server Integration** (`lsp_server.rs`)
   - Add handler method (e.g., `handle_completion`)
   - Wire up in main request dispatcher
   - Handle request/response formatting

3. **Testing**
   - Unit tests in the module itself
   - Integration tests in `/tests/lsp_*_tests.rs`
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

#### Test Infrastructure (v0.7.4)
The project includes a robust test infrastructure in `tests/support/mod.rs` with production-grade assertion helpers:

- **Assertion Helpers**: `assert_hover_has_text()`, `assert_completion_has_items()`, etc.
- **Deep Validation**: All LSP responses are validated for proper structure
- **Meaningful Errors**: Clear error messages for debugging test failures
- **No Tautologies**: All assertions actually validate response content

```bash
# Unit tests
cargo test -p perl-parser your_feature

# Integration tests
cargo test -p perl-parser lsp_your_feature_tests

# Manual testing with example
cargo run -p perl-parser --example test_your_feature

# Full LSP testing
echo '{"jsonrpc":"2.0","method":"your_method",...}' | perl-lsp --stdio

# Run comprehensive E2E tests (100% passing as of v0.7.4)
cargo test -p perl-parser lsp_comprehensive_e2e_test

# Run all tests (33 comprehensive tests)
cargo test -p perl-parser
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

4. **Diagnostics with Scope Analysis**
   - Undefined variable detection under `use strict`
   - Unused variable warnings
   - Missing pragma suggestions (strict/warnings)
   - Works with partial ASTs from error recovery

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

### Crate Structure (v0.8.7 GA)

#### Production Crates
- **`/crates/perl-parser/`**: Main parser and LSP server
  - `src/parser.rs`: Recursive descent parser
  - `src/lsp_server.rs`: LSP implementation
  - `src/ast.rs`: AST definitions
  - `bin/perl-lsp.rs`: LSP server binary
  - Published as `perl-parser` on crates.io

- **`/crates/perl-lexer/`**: Context-aware tokenizer
  - `src/lib.rs`: Lexer API
  - `src/token.rs`: Token definitions
  - `src/mode.rs`: Lexer modes
  - Published as `perl-lexer` on crates.io

- **`/crates/perl-corpus/`**: Test corpus
  - `src/lib.rs`: Corpus API
  - `tests/`: Perl test files
  - Published as `perl-corpus` on crates.io

- **`/crates/perl-parser-pest/`**: Legacy Pest parser
  - `src/grammar.pest`: PEG grammar
  - `src/lib.rs`: Parser implementation
  - Published as `perl-parser-pest` on crates.io (marked legacy)

#### Internal/Unpublished
- **`/tree-sitter-perl/`**: Original C implementation (benchmarking only)
- **`/crates/tree-sitter-perl-rs/`**: Internal test harness
- **`/xtask/`**: Development automation
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

5. **Testing Strategy**
   - Grammar tests for each Perl construct
   - Edge case tests with property testing
   - Performance benchmarks
   - Integration tests for S-expression output

   - Encoding-aware lexing for mid-file encoding changes
   - Tree-sitter compatible error nodes and diagnostics
   - Performance optimized (<5% overhead for normal code)

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready
2. **For IDE Integration**: Install `perl-lsp` from `perl-parser` crate
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions
- **Legacy**: `/crates/perl-parser-pest/` - maintenance only

### Testing
```bash
# Test main parser
cargo test -p perl-parser

# Test with corpus
cargo test -p perl-corpus

# Fast CI tests (skips slow property tests)
cargo test --workspace --features ci-fast

# Run all tests
cargo test --all
```

### Performance
Always run benchmarks after changes to ensure no regressions:
```bash
cargo bench
cargo xtask compare
```

## Pure Rust Parser Details

### Grammar Extension
To extend the Pest grammar:
1. Edit `src/grammar.pest`
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update the `build_node` method to handle new rules
4. Add tests for new constructs

### Current Grammar Coverage (~99.99%)
- ✅ Variables (scalar, array, hash) with all declaration types (my, our, local, state)
- ✅ Literals (numbers, strings with interpolation, identifiers, lists)
- ✅ All operators with proper precedence including smart match (~~)
- ✅ Control flow (if/elsif/else, unless, while, until, for, foreach, given/when)
- ✅ Subroutines (named and anonymous) with signatures and prototypes
- ✅ Package system (package, use, require, BEGIN/END blocks)
- ✅ Comments and POD documentation
- ✅ String interpolation ($var, @array, ${expr})
- ✅ Regular expressions (qr//, =~, !~, s///, tr///)
- ✅ Substitution operator (s///) with proper pattern, replacement, and modifiers parsing
- ✅ Method calls and complex dereferencing (->@*, ->%*, ->$*)
- ✅ Substitution operators via context-sensitive parsing
- ✅ Heredocs with full multi-phase parsing (all variants)
- ✅ Modern Perl features (try/catch, defer, class/method, signatures)
- ✅ Statement modifiers (print $x if $y)
- ✅ ISA operator for type checking
- ✅ Unicode identifiers and operators
- ✅ Postfix dereferencing
- ✅ Type constraints in signatures (Perl 5.36+)

## Performance Characteristics

- Pure Rust parser: ~200-450 µs for typical files (2.5KB)
- Memory usage: Arc<str> for zero-copy string storage
- Production ready: Handles real-world Perl code
- Predictable: ~180 µs/KB parsing speed
- Legacy C parser: ~12-68 µs (kept for benchmark reference only)

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

## Current Status

### v1: C-based Parser
- **Coverage**: ~95% of Perl syntax
- **Performance**: Fastest for simple parsing (~12-68 µs)
- **Status**: Legacy, kept for benchmarking

### v2: Pest-based Parser
- **Coverage**: ~99.995% of Perl syntax
- **Performance**: ~200-450 µs for typical files
- **Status**: Production ready, excellent for most use cases
- **Limitations**: 
  - Cannot parse `m!pattern!` or other non-slash regex delimiters
  - Struggles with indirect object syntax
  - Heredoc-in-string edge case

### v3: Native Lexer+Parser ⭐ **RECOMMENDED** (v0.8.9)
- **Parser Coverage**: ~100% of Perl syntax (100% of comprehensive edge cases)
- **Parser Performance**: 4-19x faster than v1 (simple: ~1.1 µs, medium: ~50-150 µs)
- **Parser Status**: Production ready, feature complete
- **LSP Status**: ✅ ~85% functional (all advertised features work, including enhanced workspace navigation and PR workflow integration)
- **Recent improvements (v0.8.9 - Production-Stable PR Workflow Integration)**:
  - ✅ **Enhanced AST format compatibility** - Program nodes now use tree-sitter standard (source_file) format while maintaining backward compatibility
  - ✅ **Comprehensive workspace navigation** - Enhanced AST traversal including `NodeKind::ExpressionStatement` support across all providers
  - ✅ **Advanced code actions and refactoring** - Fixed parameter threshold validation and enhanced refactoring suggestions with proper AST handling
  - ✅ **Enhanced call hierarchy provider** - Complete workspace analysis with improved function call tracking and incoming call detection
  - ✅ **Production-ready workspace features** - Improved workspace indexing, symbol tracking, and cross-file rename operations
  - ✅ **Comprehensive test reliability** - 100% test pass rate achieved (195/195 library tests, 33/33 LSP E2E tests, 19/19 DAP tests)
  - ✅ **Quality gate compliance** - Zero clippy warnings, consistent formatting, full architectural compliance maintained
  - ✅ **Enhanced file path completion** - Enterprise-grade security with path traversal prevention, 18 comprehensive tests, 30+ file type recognition
- **Previous improvements (v0.8.4)**:
  - ✅ Added 9 new LSP features - workspace symbols, rename, code actions, semantic tokens, inlay hints, document links, selection ranges, on-type formatting
  - ✅ Contract-driven testing - every capability backed by acceptance tests
  - ✅ Feature flag control - `lsp-ga-lock` for conservative releases
  - ✅ Fallback mechanisms - works with incomplete/invalid code
- **Previous improvements (v0.8.3)**:
  - ✅ Fixed hash literal parsing - `{ key => value }` now correctly produces HashLiteral nodes
  - ✅ Fixed parenthesized expressions with word operators - `($a or $b)` now parses correctly
  - ✅ Fixed qw() parsing - now produces ArrayLiteral nodes with proper word elements
  - ✅ Enhanced LSP go-to-definition to use DeclarationProvider for accurate function location
- **Working LSP features**:
  - ✅ Syntax checking and diagnostics
  - ✅ Basic code completion and hover
  - ✅ Single-file navigation (go-to-definition, find references)
  - ✅ Document formatting
- **Non-functional LSP features**:
  - ❌ Workspace-wide operations (stubs return empty results)
  - ❌ Cross-file navigation
  - ❌ Import optimization
  - ❌ Debug adapter
- **Previous improvements (v0.7.5)**:
  - ✅ Added enterprise-grade release automation with cargo-dist
  - ✅ Created comprehensive CI/CD pipeline with test matrix and coverage
  - ✅ Enhanced type inference for hash literals with smart unification
  - ✅ Added multi-platform binary releases (Linux/macOS/Windows, x86_64/aarch64)
  - ✅ Created Homebrew formula and one-liner installer
  - ✅ Fixed critical test infrastructure bug - recovered 400+ silently skipped tests
  - ✅ Added workspace file operations support (didChangeWatchedFiles, willRenameFiles, etc.)
  - ✅ Created zero-cost compatibility shim for smooth API migration
  - ✅ Now running 526+ tests (was incorrectly showing only 27)
  - ✅ Added CI guards to prevent test discovery regression
- **Previous improvements (v0.7.4)**:
  - ✅ Fixed all tautological test assertions (27+ fixes)
  - ✅ Created centralized test infrastructure with robust helpers
  - ✅ Achieved 100% E2E test coverage (33 tests passing)
  - ✅ Zero compilation warnings in core library
  - ✅ Cleaned up 159+ lines of dead code
- **Previous improvements (v0.7.3)**:
  - ✅ Added fallback mechanisms for incomplete/invalid code
  - ✅ Implemented undefined variable detection with scope analysis
  - ✅ Enhanced error recovery for real-time editing
- **v0.7.2 fixes**:
  - ✅ Fixed operator precedence for word operators (`or`, `and`, `not`, `xor`)
  - ✅ Fixed division operator (`/`) parsing - now correctly recognized
  - ✅ Added complete signatures for 150+ Perl built-in functions
  - ✅ Enhanced LSP signature help with comprehensive parameter hints
- **Successfully handles ALL edge cases**:
  - ✅ Regex with arbitrary delimiters (`m!pattern!`, `m{pattern}`, etc.)
  - ✅ Indirect object syntax (`print $fh "Hello"`, `print STDOUT "msg"`, `new Class`)
  - ✅ Quote operators with custom delimiters
  - ✅ All modern Perl features
  - ✅ Complex prototypes (`sub mygrep(&@) { }`, `sub test(_) { }`)
  - ✅ Emoji identifiers (`my $♥ = 'love'`)
  - ✅ Format declarations (`format STDOUT =`)
  - ✅ Decimal without trailing digits (`5.`)
  - ✅ Defined-or operator (`//`)
  - ✅ Glob dereference (`*$ref`)
  - ✅ Pragma arguments (`use constant FOO => 42`)
  - ✅ List interpolation (`@{[ expr ]}`)
  - ✅ Multi-variable attributes (`my ($x :shared, $y :locked)`)

### Parser Comparison Summary

| Feature | v1 (C) | v2 (Pest) | v3 (Native) |
|---------|--------|-----------|-------------|
| Coverage | ~95% | ~99.996% | ~100% |
| Performance | ~12-68 µs | ~200-450 µs | ~6-21 µs (improved v0.8.9) |
| Regex delimiters | ❌ | ❌ | ✅ |
| Indirect object | ❌ | ❌ | ✅ |
| Unicode identifiers | ✅ | ✅ | ✅ |
| Modern Perl (5.38+) | ❌ | ✅ | ✅ |
| Tree-sitter compatible | ✅ | ✅ | ✅ Enhanced |
| Workspace navigation | ❌ | Limited | ✅ Production |
| Test reliability | Limited | 95% | 100% |
| Active development | ❌ | ✅ | ✅ |
| Edge case tests | Limited | 95% | 100% |

See KNOWN_LIMITATIONS.md for complete details.

### Context-Sensitive Features

The parser includes sophisticated solutions for Perl's context-sensitive features:

#### Slash Disambiguation
1. **Mode-aware lexer** (`perl_lexer.rs`) - Tracks parser state to disambiguate / as division vs regex
2. **Preprocessing adapter** (`lexer_adapter.rs`) - Transforms ambiguous tokens for PEG parsing
3. **Disambiguated parser** (`disambiguated_parser.rs`) - High-level API with automatic handling

See `SLASH_DISAMBIGUATION.md` for full details.

#### Heredoc Support
1. **Multi-phase parser** (`heredoc_parser.rs`) - Three-phase approach to handle stateful heredocs
2. **Full parser** (`full_parser.rs`) - Combines heredoc and slash handling
3. **Complete coverage** - Supports all heredoc variants including indented heredocs

See `HEREDOC_IMPLEMENTATION.md` for full details.

#### Edge Case Handling
1. **Edge case handler** (`edge_case_handler.rs`) - Unified detection and recovery system
2. **Phase-aware parsing** (`phase_aware_parser.rs`) - Handles BEGIN/CHECK/INIT/END blocks
3. **Dynamic recovery** (`dynamic_delimiter_recovery.rs`) - Multiple strategies for runtime delimiters
4. **Tree-sitter adapter** (`tree_sitter_adapter.rs`) - Ensures 100% AST compatibility

See `docs/EDGE_CASES.md` for comprehensive documentation.

## Code Quality Standards

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