# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation and Enhanced Workspace Navigation
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md) for guarantees

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem:

### Published Crates (v0.8.9 GA)

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
- **LSP server implementation** - core LSP server logic used by perl-lsp binary

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

#### 4. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
- **Standalone Language Server binary** - production-ready LSP server for Perl
- **Clean separation** from parser logic for improved maintainability
- **Production-grade command-line interface** with comprehensive options
- **Enhanced modularity** - focused solely on LSP protocol implementation
- **Easy installation** - dedicated crate for LSP binary distribution
- **Editor integration** - works with VSCode, Neovim, Emacs, and all LSP-compatible editors
- **Advanced CLI features** - health checks, feature discovery, version reporting
- **Built on perl-parser** - leverages all parser capabilities through clean API

#### 5. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
- Pest-based parser (v2 implementation)
- ~99.995% Perl 5 coverage
- Marked as legacy - use perl-parser instead
- Kept for migration/comparison

## Incremental Parsing with Rope-based Document Management (v0.8.7) 🚀

The native parser includes **production-ready incremental parsing** with **Rope-based document management** for efficient real-time LSP editing:

### Architecture (**Diataxis: Explanation**)
- **IncrementalDocument**: High-performance document state with subtree caching and Rope integration
- **Rope-based Text Management**: Efficient UTF-16/UTF-8 position conversion using `ropey` crate
- **Subtree Reuse**: Container nodes reuse unchanged AST subtrees from cache  
- **Metrics Tracking**: Detailed performance metrics (reused vs reparsed nodes)
- **Content-based Caching**: Hash-based subtree matching for common patterns
- **Position-based Caching**: Range-based subtree matching with precise Rope position tracking

### Rope Integration (**Diataxis: Reference**)
The perl-parser crate includes comprehensive Rope support for document management:

**Core Rope Modules**:
- **`textdoc.rs`**: UTF-16 aware text document handling with `ropey::Rope`
- **`position_mapper.rs`**: Centralized position mapping (CRLF/LF/CR line endings, UTF-16 code units, byte offsets)
- **`incremental_integration.rs`**: Bridge between LSP server and incremental parsing with Rope
- **`incremental_handler_v2.rs`**: Enhanced incremental document updates using Rope

**Position Conversion Features**:
```rust
// UTF-16/UTF-8 position conversion
use crate::textdoc::{Doc, PosEnc, lsp_pos_to_byte, byte_to_lsp_pos};
use ropey::Rope;

// Create document with Rope
let mut doc = Doc { rope: Rope::from_str(content), version };

// Convert LSP positions (UTF-16) to byte offsets 
let byte_offset = lsp_pos_to_byte(&doc.rope, pos, PosEnc::Utf16);

// Convert byte offsets to LSP positions
let lsp_pos = byte_to_lsp_pos(&doc.rope, byte_offset, PosEnc::Utf16);
```

**Line Ending Support**:
- **CRLF handling**: Proper Windows line ending support
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR
- **UTF-16 emoji support**: Correct positioning with Unicode characters requiring surrogate pairs

### Performance Targets (**Diataxis: Reference**)
- **<1ms updates** for small edits (single token changes) with Rope optimization
- **<2ms updates** for moderate edits (function-level changes) with subtree reuse
- **Cache hit ratios** of 70-90% for typical editing scenarios
- **Memory efficient** with LRU cache eviction, Arc<Node> sharing, and Rope's piece table architecture

### Incremental Parsing API (**Diataxis: Tutorial**)
```rust
// Create incremental document with Rope support
let mut doc = IncrementalDocument::new(source)?;

// Apply single edit (automatically uses Rope for position tracking)
let edit = IncrementalEdit::new(start_byte, end_byte, new_text);
doc.apply_edit(edit)?;

// Apply multiple edits in batch (Rope handles position adjustments)
let mut edits = IncrementalEditSet::new();
edits.add(edit1);
edits.add(edit2);
doc.apply_edits(&edits)?;

// Performance metrics with Rope-optimized parsing
println!("Parse time: {:.2}ms", doc.metrics.last_parse_time_ms);
println!("Nodes reused: {}", doc.metrics.nodes_reused);
println!("Nodes reparsed: {}", doc.metrics.nodes_reparsed);
```

### LSP Integration (**Diataxis: How-to**)
- **Document Management**: LSP server uses Rope for all document state (`textdoc::Doc`)
- **Position Conversion**: Automatic UTF-16 ↔ UTF-8 conversion via `position_mapper::PositionMapper`
- **Incremental Updates**: Enable via `PERL_LSP_INCREMENTAL=1` environment variable
- **Change Application**: Efficient change processing using `textdoc::apply_changes()`
- **Fallback Mechanisms**: Graceful degradation to full parsing when incremental parsing fails
- **Testing**: Comprehensive integration tests with async LSP harness and Rope-based position validation

### Development Guidelines (**Diataxis: How-to**)
**Where to Make Rope Improvements**:
- **Production Code**: `/crates/perl-parser/src/` - All Rope enhancements should target this crate
- **Key Modules**: `textdoc.rs`, `position_mapper.rs`, `incremental_*.rs` modules
- **NOT Internal Test Harnesses**: Avoid modifying `/crates/tree-sitter-perl-rs/` or other internal test code

**Rope Testing Commands**:
```bash
# Test Rope-based position mapping
cargo test -p perl-parser position_mapper

# Test incremental parsing with Rope integration  
cargo test -p perl-parser incremental_integration_test

# Test UTF-16 position conversion with multibyte characters
cargo test -p perl-parser multibyte_edit_test

# Test LSP document changes with Rope
cargo test -p perl-lsp lsp_comprehensive_e2e_test
```

## LSP Crate Separation Architecture (v0.8.9) 🏗️ **ARCHITECTURAL ENHANCEMENT**

### Rationale and Benefits (**Diataxis: Explanation**)

The comprehensive LSP crate separation in v0.8.9 represents a major architectural improvement that provides clear separation of concerns between parsing logic and LSP protocol implementation:

**Architectural Principles**:
- **Single Responsibility**: Each crate has a focused, well-defined purpose
- **Clean Interfaces**: Clear API boundaries between parser and LSP functionality
- **Independent Versioning**: LSP server can evolve independently from parser core
- **Reduced Coupling**: LSP protocol changes don't impact parser internals
- **Enhanced Testability**: Isolated testing of LSP features and parser logic

**Production Benefits**:
- **Improved Maintainability**: Easier to understand, modify, and extend each component
- **Better Distribution**: Users can install only what they need (parser library vs LSP binary)
- **Enhanced Modularity**: Clear separation enables better code organization
- **Reduced Build Times**: Selective compilation of components reduces build overhead
- **Cleaner Dependencies**: Each crate manages only its necessary dependencies

### Crate Responsibilities (**Diataxis: Reference**)

**perl-parser crate** (`/crates/perl-parser/`):
- **Core parser implementation** - AST generation, syntax analysis
- **LSP provider logic** - completion, hover, diagnostics, etc.
- **Text processing utilities** - Rope integration, position mapping
- **Incremental parsing** - document state management, cache handling
- **Library API** - stable interface for external consumers

**perl-lsp crate** (`/crates/perl-lsp/`):
- **LSP protocol implementation** - JSON-RPC communication, request handling
- **Command-line interface** - argument parsing, logging, health checks
- **Server lifecycle management** - initialization, shutdown, error handling
- **Editor integration** - protocol compliance, feature advertisement
- **Binary distribution** - production-ready executable for end users

### Migration Guide (**Diataxis: How-to**)

**For End Users**:
```bash
# Old approach (deprecated)
cargo install perl-parser --features lsp

# New approach (recommended)
cargo install perl-lsp
```

**For Library Consumers**:
```rust
// Parser functionality remains in perl-parser
use perl_parser::{Parser, LspServer, CompletionProvider};

// LSP binary logic is now in perl-lsp crate
// (most users don't need to import this directly)
```

**For Contributors**:
- **Parser improvements** → `/crates/perl-parser/src/`
- **LSP protocol features** → `/crates/perl-parser/src/` (provider logic)
- **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
- **Integration tests** → `/crates/perl-lsp/tests/` (E2E LSP tests)

### Quality Improvements (**Diataxis: Reference**)

The crate separation delivered immediate quality benefits:
- **Zero clippy warnings** across both crates
- **Consistent formatting** with shared workspace lints
- **Enhanced test coverage** with dedicated LSP integration tests
- **Improved error handling** with focused error types per crate
- **Better documentation** with crate-specific examples and guides

## Enhanced Workspace Navigation and PR Workflow Integration (v0.8.9) ⭐ **PRODUCTION READY**

### Comprehensive Workspace Features (**Diataxis: Explanation**)

The v0.8.9 release introduces production-stable workspace navigation with comprehensive AST traversal enhancements and PR workflow integration capabilities:

**Enhanced AST Traversal Patterns**:
- **ExpressionStatement Support**: All LSP providers now properly traverse `NodeKind::ExpressionStatement` nodes for complete symbol coverage
- **Tree-sitter Standard AST Format**: Program nodes now use standard (source_file) format while maintaining backward compatibility  
- **Comprehensive Node Coverage**: Enhanced workspace indexing covers all Perl syntax constructs across the entire codebase
- **Production-Stable Symbol Tracking**: Improved symbol resolution with enhanced cross-file reference tracking

**Advanced Code Actions and Refactoring** (**Diataxis: Reference**):
- **Parameter Threshold Validation**: Fixed refactoring suggestions with proper parameter counting and threshold enforcement
- **Enhanced Refactoring Engine**: Improved AST traversal for comprehensive code transformation suggestions
- **Smart Refactoring Detection**: Advanced pattern recognition for extract method, variable, and other refactoring opportunities
- **Production-Grade Error Handling**: Robust validation and fallback mechanisms for complex refactoring scenarios

**Call Hierarchy and Workspace Analysis** (**Diataxis: How-to**):
- **Enhanced Call Hierarchy Provider**: Complete workspace analysis with improved function call tracking and incoming call detection
- **Comprehensive Function Discovery**: Enhanced recursive traversal for complete subroutine and method identification across all AST node types
- **Cross-File Call Analysis**: Improved workspace-wide call relationship tracking with accurate reference resolution
- **Advanced Symbol Navigation**: Enhanced go-to-definition and find-references with comprehensive workspace indexing

### Tutorial: Using Enhanced Workspace Features (**Diataxis: Tutorial**)

**Step 1: Workspace Symbol Search**
```perl
# The LSP now finds symbols across all contexts:
sub main_function {     # Found via workspace/symbol search
    my $var = 42;       # Local scope tracking enhanced
}

{
    sub nested_function { }  # Now discovered via ExpressionStatement traversal
}
```

**Step 2: Enhanced Cross-File Navigation**
```perl
# File: lib/Utils.pm
our $GLOBAL_CONFIG = {};   # Workspace-wide rename support

sub utility_function {     # Enhanced call hierarchy tracking
    # Function implementation
}

# File: bin/app.pl  
use lib 'lib';
use Utils;
$Utils::GLOBAL_CONFIG = {};  # Cross-file reference resolution
Utils::utility_function();  # Enhanced call hierarchy navigation
```

**Step 3: Advanced Code Actions and Refactoring**
```perl
# Before refactoring suggestions enhancement:
my $result = calculate_complex_value($a, $b, $c, $d, $e);  # Complex parameter list

# Enhanced code actions now suggest:
# 1. Extract method for parameter grouping
# 2. Parameter object pattern
# 3. Method chaining opportunities
```

### How-to Guide: Leveraging Workspace Integration (**Diataxis: How-to**)

**Enable Enhanced Workspace Features**:
```bash
# LSP server automatically uses enhanced workspace indexing
perl-lsp --stdio

# For development and debugging:
PERL_LSP_DEBUG=1 perl-lsp --stdio --log
```

**Testing Enhanced Features**:
```bash
# Test comprehensive workspace symbol detection
cargo test -p perl-parser workspace_index_comprehensive_symbol_traversal

# Test enhanced call hierarchy provider
cargo test -p perl-parser call_hierarchy_enhanced_expression_statement_support  

# Test improved code actions
cargo test -p perl-parser code_actions_enhanced_parameter_threshold_validation

# Test cross-file workspace features
cargo test -p perl-parser workspace_rename_cross_file_symbol_resolution
```

**Performance and Quality Metrics**:
- **100% Test Coverage**: All 195 library tests, 33 LSP E2E tests, and 19 DAP tests passing
- **Zero Quality Issues**: No clippy warnings, consistent code formatting maintained
- **Enhanced Symbol Resolution**: Improved accuracy in cross-file symbol tracking and reference resolution
- **Production-Ready Reliability**: Comprehensive validation across all supported Perl constructs

### Reference: Enhanced API Documentation (**Diataxis: Reference**)

**Enhanced Workspace Indexing**:
```rust
// Enhanced workspace index with ExpressionStatement support
impl WorkspaceIndex {
    /// Traverse all AST nodes including ExpressionStatement patterns
    pub fn index_symbols_comprehensive(&mut self, ast: &Node, file_path: &str);
    
    /// Enhanced symbol resolution with cross-file reference tracking
    pub fn resolve_symbol_enhanced(&self, symbol: &str) -> Vec<SymbolReference>;
}

// Enhanced code actions with parameter validation
impl CodeActionsEnhanced {
    /// Validate refactoring parameters with proper threshold checking
    pub fn validate_refactoring_parameters(&self, node: &Node) -> RefactoringValidation;
    
    /// Generate refactoring suggestions with enhanced AST analysis
    pub fn suggest_refactorings_enhanced(&self, context: &RefactoringContext) -> Vec<CodeAction>;
}

// Enhanced call hierarchy with comprehensive traversal
impl CallHierarchyProvider {
    /// Track function calls across all node types including ExpressionStatement
    pub fn find_calls_comprehensive(&self, function: &str) -> CallHierarchy;
    
    /// Enhanced incoming call detection with workspace-wide analysis
    pub fn find_incoming_calls_enhanced(&self, target: &str) -> Vec<CallReference>;
}
```

**Quality Gate Integration**:
- **Architectural Compliance**: Full compliance with Rust 2024 edition and MSRV 1.89+ requirements
- **Performance Validation**: No performance regressions detected in enhanced workspace operations
- **Memory Safety**: All enhanced features maintain memory safety and thread safety guarantees
- **Production Crate Compatibility**: Enhanced features fully compatible with published crate ecosystem

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
  - ✅ **Import optimization** - unused import detection, duplicate consolidation (NEW)
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

### Tutorial: Using the Standalone perl-lsp Binary (**Diataxis: Tutorial**)

The separated perl-lsp crate provides a production-ready LSP server with comprehensive CLI features:

#### **Step 1: Installation**
```bash
# Method 1: Install from crates.io (recommended)
cargo install perl-lsp

# Method 2: Install from source
git clone https://github.com/EffortlessSteven/tree-sitter-perl-rs
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp

# Method 3: Build without installing
cargo build -p perl-lsp --release
# Binary will be at target/release/perl-lsp
```

#### **Step 2: Basic Usage**
```bash
# Run LSP server (default stdio mode for editors)
perl-lsp --stdio

# Run with debug logging (helpful for troubleshooting)
perl-lsp --stdio --log

# Quick health check (verify installation)
perl-lsp --health
# Output: ok 0.8.9

# Show version and build information
perl-lsp --version
# Output: perl-lsp 0.8.9
#         Git tag: v0.8.9
#         Perl Language Server using perl-parser v3
```

#### **Step 3: Editor Integration**

**VSCode Integration**:
1. Install a generic LSP extension or Perl-specific extension
2. Configure the language server:
```json
{
    "perl.languageServer": {
        "command": "perl-lsp",
        "args": ["--stdio"],
        "rootPatterns": ["Makefile.PL", "Build.PL", "cpanfile", ".git"]
    }
}
```

**Neovim Integration** (with nvim-lspconfig):
```lua
require'lspconfig'.perl_lsp = {
  default_config = {
    cmd = {'perl-lsp', '--stdio'},
    filetypes = {'perl'},
    root_dir = require'lspconfig'.util.find_git_ancestor,
  },
}
require'lspconfig'.perl_lsp.setup{}
```

#### **Step 4: Advanced Features**

**Feature Discovery**:
```bash
# Export complete feature catalog as JSON
perl-lsp --features-json
# Outputs comprehensive capability matrix for integration
```

**Production Debugging**:
```bash
# Run with detailed logging for troubleshooting
perl-lsp --stdio --log 2> perl-lsp.log
# Monitor perl-lsp.log for diagnostic information
```

**Health Monitoring**:
```bash
# Automated health checks in scripts
if perl-lsp --health > /dev/null 2>&1; then
    echo "perl-lsp is ready"
else
    echo "perl-lsp installation issue"
    exit 1
fi
```

#### **Step 5: Workspace Features**

The perl-lsp binary automatically provides:
- **Cross-file navigation** - Go to definition across modules
- **Workspace-wide symbol search** - Find functions/variables project-wide
- **Incremental parsing** - Real-time syntax checking as you type
- **File path completion** - Auto-complete file paths in strings
- **Enterprise security** - Path traversal prevention, safe file operations

## Default Build Configuration

The project includes `.cargo/config.toml` which automatically configures:
- Optimized dev builds (`opt-level = 1`) for parser testing
- Full release optimization (`lto = true`) for benchmarks  
- Automatic backtraces (`RUST_BACKTRACE=1`)
- Sparse registry protocol for faster updates

**AI tools can run bare `cargo build` and `cargo test` commands** - the configuration ensures correct behavior.

## Key Commands

### Build Commands

#### LSP Server (perl-lsp crate)
```bash
# Quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Homebrew (macOS)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# Build perl-lsp binary from dedicated crate
cargo build -p perl-lsp --release

# Install perl-lsp binary globally from separated crate
cargo install --path crates/perl-lsp

# Install perl-lsp from crates.io (production binary)
cargo install perl-lsp

# Run the LSP server with enhanced CLI options
perl-lsp --stdio              # For editor integration (default)
perl-lsp --stdio --log        # With debug logging
perl-lsp --health             # Quick health check
perl-lsp --version            # Show version and build info
perl-lsp --features-json      # Export feature catalog as JSON
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
cargo install perl-lsp                     # Standalone LSP server binary
cargo add perl-parser                      # Core parser library
cargo add perl-lexer                       # Tokenizer library  
cargo add perl-corpus --dev                # Testing corpus

# Build all production crates from source
cargo build -p perl-lsp --release          # Standalone LSP binary
cargo build -p perl-parser --release       # Core parser library
cargo build -p perl-lexer --release        # Tokenizer
cargo build -p perl-corpus --release       # Test corpus
cargo build -p perl-parser-pest --release  # Legacy (maintenance only)
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

# Test import optimizer (from library usage)
# Example: Analyze a Perl file for import issues
```rust
use perl_parser::import_optimizer::ImportOptimizer;
use std::path::Path;

let optimizer = ImportOptimizer::new();
let analysis = optimizer.analyze_file(Path::new("script.pl"))?;
let optimized_imports = optimizer.generate_optimized_imports(&analysis);
println!("{}", optimized_imports);
```
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

# Run scope analyzer tests specifically
cargo test -p perl-parser --test scope_analyzer_tests

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

### Adding New LSP Features (Updated for Crate Separation)

With the new perl-lsp crate separation, implementing new LSP features follows a clean architectural pattern:

#### **Provider Implementation** (`/crates/perl-parser/src/`)
1. **Add feature provider** (e.g., `completion.rs`, `code_actions.rs`, `import_optimizer.rs`)
   - Implement provider struct with core logic
   - **Use source-aware constructors** for enhanced documentation support
   - Focus on language analysis, not protocol details
   - Add to `lib.rs` exports for public API

#### **LSP Server Integration** (`/crates/perl-parser/src/lsp_server.rs`)
2. **Add request handler** 
   - Add handler method (e.g., `handle_completion`)
   - **Thread source text** through provider constructors using `doc.content`
   - Wire up in main request dispatcher
   - Handle request/response formatting and protocol compliance

#### **Binary Interface** (`/crates/perl-lsp/src/`)
3. **CLI enhancement (if needed)**
   - Update command-line options for new features
   - Add feature discovery support (`--features-json`)
   - Ensure proper error handling and logging

#### **Testing Strategy**
4. **Multi-level testing**
   - **Unit tests**: Provider logic in `/crates/perl-parser/tests/`
   - **LSP integration tests**: Protocol compliance in `/crates/perl-lsp/tests/`  
   - **Symbol documentation tests**: Comment extraction features
   - **E2E user story tests**: Real-world scenarios with async LSP harness

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

3. **Import Optimization** (`import_optimizer.rs`) (**Diataxis: Explanation**)

The import optimization system provides comprehensive analysis and optimization of Perl import statements with enterprise-grade reliability and performance.

**Architecture** (**Diataxis: Explanation**):
- **ImportOptimizer**: Core analysis engine with regex-based usage detection
- **ImportAnalysis**: Structured analysis results with unused, duplicate, and missing import tracking
- **OptimizedImportGeneration**: Alphabetical sorting and clean formatting with duplicate consolidation
- **Complete Test Coverage**: 9 comprehensive test cases covering all optimization scenarios

**Features** (**Diataxis: Reference**):
- **Unused Import Detection**: Regex-based usage analysis identifies import statements never used in code
- **Duplicate Import Consolidation**: Merges multiple import lines from same module into single optimized statements  
- **Missing Import Detection**: Identifies Module::symbol references requiring additional imports (planned)
- **Optimized Import Generation**: Alphabetical sorting and clean formatting of import statements
- **Performance Optimized**: Fast analysis suitable for real-time LSP code actions

**Import Optimizer API** (**Diataxis: Reference**):
```rust
// Core ImportOptimizer methods
impl ImportOptimizer {
    /// Create new optimizer instance
    pub fn new() -> Self;
    
    /// Analyze Perl file for import optimization opportunities
    pub fn analyze_file(&self, path: &Path) -> Result<ImportAnalysis, ImportOptimizerError>;
    
    /// Generate optimized import statements from analysis
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String;
}

// ImportAnalysis structure
pub struct ImportAnalysis {
    pub unused_imports: Vec<UnusedImport>,
    pub duplicate_imports: Vec<DuplicateImport>,
    pub missing_imports: Vec<MissingImport>, // planned
}
```

**Tutorial: Using Import Optimization** (**Diataxis: Tutorial**):
```rust
use perl_parser::import_optimizer::ImportOptimizer;
use std::path::Path;

// Step 1: Create optimizer
let optimizer = ImportOptimizer::new();

// Step 2: Analyze a Perl file for import issues
let analysis = optimizer.analyze_file(Path::new("script.pl"))?;

// Step 3: Check for unused imports
for unused in &analysis.unused_imports {
    println!("Module {} has unused symbols: {:?}", unused.module, unused.symbols);
}

// Step 4: Check for duplicate imports
for duplicate in &analysis.duplicate_imports {
    println!("Module {} imported {} times", duplicate.module, duplicate.count);
}

// Step 5: Generate optimized imports
let optimized = optimizer.generate_optimized_imports(&analysis);
println!("Optimized imports:\n{}", optimized);
```

**How-to Guide: Testing Import Optimization** (**Diataxis: How-to**):
```bash
# Run all import optimizer tests
cargo test -p perl-parser --test import_optimizer_tests

# Test specific optimization scenarios
cargo test -p perl-parser import_optimizer_tests::test_unused_import_detection
cargo test -p perl-parser import_optimizer_tests::test_duplicate_consolidation
cargo test -p perl-parser import_optimizer_tests::test_optimized_generation

# Integration testing with LSP code actions
cargo test -p perl-parser --test lsp_code_actions_tests -- import_optimization
```

**Adding New Refactorings**:
```rust
// In code_actions_enhanced.rs
fn your_refactoring(&self, node: &Node) -> Option<CodeAction> {
    // 1. Check if refactoring applies
    // 2. Generate new code
    // 3. Return CodeAction with TextEdits
}
```

### Testing LSP Features (Separated Crate Architecture)

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

### Crate Structure (v0.8.7 GA)

#### Production Crates
- **`/crates/perl-parser/`**: Main parser and LSP server
  - `src/parser.rs`: Recursive descent parser
  - `src/lsp_server.rs`: LSP implementation
  - `src/ast.rs`: AST definitions
  - `bin/perl-lsp.rs`: LSP server binary
  - Published as `perl-parser` on crates.io

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
1. **For Perl Parsing Libraries**: Use `perl-parser` - fastest, most complete, production-ready with Rope support
2. **For IDE/Editor Integration**: Install `perl-lsp` - dedicated LSP server binary with clean CLI interface
3. **For Application Development**: Add `perl-parser` as dependency, install `perl-lsp` binary for development
4. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite  
5. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations (**Diataxis: Reference**)
- **Core Parser**: `/crates/perl-parser/` - parser logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone LSP server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - test case additions, corpus validation
- **Legacy**: `/crates/perl-parser-pest/` - maintenance only (contains outdated Rope usage)

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

### Position Tracking Development (**Diataxis: How-to**) (v0.8.7+)

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

## Pure Rust Parser Details

### Grammar Extension
To extend the Pest grammar:
1. Edit `src/grammar.pest`
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update the `build_node` method to handle new rules
4. Add tests for new constructs

### Current Grammar Coverage (~100%)
- ✅ Variables (scalar, array, hash) with all declaration types (my, our, local, state)
- ✅ Literals (numbers, strings with interpolation, identifiers, lists)
- ✅ All operators with proper precedence including smart match (~~)
- ✅ Control flow (if/elsif/else, unless, while, until, for, foreach, given/when/default)
- ✅ Subroutines (named and anonymous) with signatures and prototypes
  - Enhanced anonymous subroutine handling with automatic expression statement wrapping
  - Maintains backward compatibility with existing named subroutine parsing
  - Preserves AST structure integrity for downstream consumers
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

## Incremental Parsing with Rope-based Document Management (v0.8.7) 🚀

The native parser includes **production-ready incremental parsing** with **Rope-based document management** for efficient real-time LSP editing:

### Architecture (**Diataxis: Explanation**)
- **IncrementalDocument**: High-performance document state with subtree caching and Rope integration
- **Rope-based Text Management**: Efficient UTF-16/UTF-8 position conversion using `ropey` crate
- **Subtree Reuse**: Container nodes reuse unchanged AST subtrees from cache  
- **Metrics Tracking**: Detailed performance metrics (reused vs reparsed nodes)
- **Content-based Caching**: Hash-based subtree matching for common patterns
- **Position-based Caching**: Range-based subtree matching with precise Rope position tracking

### Rope Integration (**Diataxis: Reference**)
The perl-parser crate includes comprehensive Rope support for document management:

**Core Rope Modules**:
- **`textdoc.rs`**: UTF-16 aware text document handling with `ropey::Rope`
- **`position_mapper.rs`**: Centralized position mapping (CRLF/LF/CR line endings, UTF-16 code units, byte offsets)
- **`incremental_integration.rs`**: Bridge between LSP server and incremental parsing with Rope
- **`incremental_handler_v2.rs`**: Enhanced incremental document updates using Rope

**Position Conversion Features**:
```rust
// UTF-16/UTF-8 position conversion
use crate::textdoc::{Doc, PosEnc, lsp_pos_to_byte, byte_to_lsp_pos};
use ropey::Rope;

// Create document with Rope
let mut doc = Doc { rope: Rope::from_str(content), version };

// Convert LSP positions (UTF-16) to byte offsets 
let byte_offset = lsp_pos_to_byte(&doc.rope, pos, PosEnc::Utf16);

// Convert byte offsets to LSP positions
let lsp_pos = byte_to_lsp_pos(&doc.rope, byte_offset, PosEnc::Utf16);
```

**Line Ending Support**:
- **CRLF handling**: Proper Windows line ending support
- **Mixed line endings**: Robust detection and handling of mixed CRLF/LF/CR
- **UTF-16 emoji support**: Correct positioning with Unicode characters requiring surrogate pairs

### Performance Targets (**Diataxis: Reference**)
- **<1ms updates** for small edits (single token changes) with Rope optimization
- **<2ms updates** for moderate edits (function-level changes) with subtree reuse
- **Cache hit ratios** of 70-90% for typical editing scenarios
- **Memory efficient** with LRU cache eviction, Arc<Node> sharing, and Rope's piece table architecture

### Advanced Incremental Parsing (IncrementalParserV2)
The repository includes an enhanced incremental parser (`IncrementalParserV2`) with intelligent node reuse strategies:

**Key Features**:
- **Smart Node Reuse**: Automatically detects which AST nodes can be preserved across edits
- **Metrics Tracking**: Provides detailed statistics on reused vs reparsed nodes
- **Simple Value Edit Detection**: Optimized for common scenarios like number/string changes
- **Fallback Mechanisms**: Graceful degradation to full parsing when needed

**Usage Example**:
```rust
// Basic incremental parsing with IncrementalParserV2
use perl_parser::{incremental_v2::IncrementalParserV2, edit::Edit, position::Position};

let mut parser = IncrementalParserV2::new();

// Initial parse
let tree1 = parser.parse("my $x = 42;")?;
println!("Initial: Reparsed={}, Reused={}", parser.reparsed_nodes, parser.reused_nodes);

// Apply edit (change "42" to "4242")
parser.edit(Edit::new(8, 10, 12, /* position data */));
let tree2 = parser.parse("my $x = 4242;")?;
println!("After edit: Reparsed={}, Reused={}", parser.reparsed_nodes, parser.reused_nodes);
// Expected output: Reparsed=1, Reused=3 (only the Number node needs reparsing)
```

**Testing IncrementalParserV2**:
```bash
# Test the example implementation
cargo run -p perl-parser --example test_incremental_v2 --features incremental

# Run comprehensive incremental tests
cargo test -p perl-parser --test incremental_integration_test --features incremental

# Run all incremental-related tests
cargo test -p perl-parser incremental --features incremental
```

### Incremental Parsing API (**Diataxis: Tutorial**)
```rust
// Create incremental document with Rope support
let mut doc = IncrementalDocument::new(source)?;

// Apply single edit (automatically uses Rope for position tracking)
let edit = IncrementalEdit::new(start_byte, end_byte, new_text);
doc.apply_edit(edit)?;

// Apply multiple edits in batch (Rope handles position adjustments)
let mut edits = IncrementalEditSet::new();
edits.add(edit1);
edits.add(edit2);
doc.apply_edits(&edits)?;

// Performance metrics with Rope-optimized parsing
println!("Parse time: {:.2}ms", doc.metrics.last_parse_time_ms);
println!("Nodes reused: {}", doc.metrics.nodes_reused);
println!("Nodes reparsed: {}", doc.metrics.nodes_reparsed);
```

### LSP Integration (**Diataxis: How-to**)
- **Document Management**: LSP server uses Rope for all document state (`textdoc::Doc`)
- **Position Conversion**: Automatic UTF-16 ↔ UTF-8 conversion via `position_mapper::PositionMapper`
- **Incremental Updates**: Enable via `PERL_LSP_INCREMENTAL=1` environment variable
- **Change Application**: Efficient change processing using `textdoc::apply_changes()`
- **Fallback Mechanisms**: Graceful degradation to full parsing when incremental parsing fails
- **Testing**: Comprehensive integration tests with async LSP harness and Rope-based position validation

### Development Guidelines (**Diataxis: How-to**)
**Where to Make Rope Improvements**:
- **Production Code**: `/crates/perl-parser/src/` - All Rope enhancements should target this crate
- **Key Modules**: `textdoc.rs`, `position_mapper.rs`, `incremental_*.rs` modules
- **NOT Internal Test Harnesses**: Avoid modifying `/crates/tree-sitter-perl-rs/` or other internal test code

**Rope Testing Commands**:
```bash
# Test Rope-based position mapping
cargo test -p perl-parser position_mapper

# Test incremental parsing with Rope integration  
cargo test -p perl-parser incremental_integration_test

# Test UTF-16 position conversion with multibyte characters
cargo test -p perl-parser multibyte_edit_test

# Test LSP document changes with Rope
cargo test -p perl-lsp lsp_comprehensive_e2e_test
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

## Unicode Handling (v0.8.6)

The lexer includes comprehensive Unicode support with recent robustness improvements:

### Unicode Features
- **Unicode Identifiers**: Full support for Unicode characters in variable names (`my $♥ = 'love'`)  
- **Unicode Operators**: Support for Unicode operators and symbols
- **UTF-8 Text Processing**: Proper handling of UTF-8 encoded Perl source files
- **Context-Aware Parsing**: Unicode characters properly handled in all lexer contexts

### Recent Improvements (v0.8.6)
**Fixed Unicode + Heredoc Panic** (`perl-lexer` v0.8.6):
- **Problem**: Lexer would panic on Unicode characters followed by incomplete heredoc syntax (e.g., `¡<<'`)
- **Root Cause**: Bounds checking failure during heredoc delimiter extraction with Unicode text
- **Solution**: Enhanced text construction tracking throughout heredoc parsing phases
- **Testing**: Added comprehensive Unicode test cases to prevent regression

**Troubleshooting Guide: Unicode Issues**:
```perl
# These cases are now handled correctly:
¡<<'             # Unicode + incomplete heredoc (was panic, now graceful)
my $♥ = 42;      # Unicode variable names (always worked)  
¡ << 'END'       # Unicode with spacing (always worked)
print "♥";       # Unicode in strings (always worked)
```

**Technical Details**:
- Uses `src/unicode.rs` for Unicode character classification
- Implements `is_perl_identifier_start()` and `is_perl_identifier_continue()`
- Maintains text construction state during all parsing phases
- Provides graceful error handling for malformed Unicode sequences

**Reference: Unicode Test Coverage**:
- Property-based testing with Unicode edge cases
- Regression tests for specific Unicode + heredoc combinations  
- Performance testing ensures no Unicode processing overhead

## Current Status

### v1: C-based Parser
- **Coverage**: ~95% of Perl syntax
- **Performance**: Fastest for simple parsing (~12-68 µs)
- **Status**: Legacy, kept for benchmarking

### v2: Pest-based Parser
- **Coverage**: ~99.996% of Perl syntax (improved substitution support as of PR #42)
- **Performance**: ~200-450 µs for typical files
- **Status**: Production ready, excellent for most use cases
- **Recent improvements (PR #42)**:
  - ✅ **Enhanced substitution parsing** - improved coverage from ~99.995% to ~99.996%
  - ✅ **Robust delimiter handling** for s/// operators with paired delimiters (s{pattern}{replacement})
  - ✅ **Improved quote parser** with better error handling and nested delimiter support
  - ✅ **Comprehensive test coverage** for substitution edge cases
  - ✅ Backward compatibility with fallback mechanisms
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
  - ✅ **Enhanced variable resolution patterns** - comprehensive scope analysis improvements:
    - Hash element access: `$hash{key}` → `%hash` (proper sigil conversion)
    - Array element access: `$array[idx]` → `@array` (proper sigil conversion)
    - Array/hash slices: `@hash{keys}`, `@array[indices]`
    - Complex nested patterns: `$data{user}->{name}`, `$items[0]->{field}`
    - Context-aware bareword detection in hash keys
    - **38 comprehensive scope analyzer tests** ensuring all patterns work correctly
  - ✅ **Advanced delimiter recovery** with comprehensive pattern recognition
  - ✅ **Hash key context detection** to reduce false bareword warnings
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
| Import optimization | ❌ | ❌ | ✅ |

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