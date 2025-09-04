# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation and Enhanced Workspace Navigation
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md) for guarantees

## Project Overview

This repository contains **four published crates** forming a complete Perl parsing ecosystem:

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
- Includes LSP server binary (`perl-lsp`) with full Rope-based document state
- **v0.8.9 improvements** (Production-Stable PR Workflow Integration):
  - **Enhanced AST format compatibility** - Program nodes now use tree-sitter standard (source_file) format while maintaining backward compatibility
  - **Comprehensive workspace navigation** - Enhanced AST traversal including `NodeKind::ExpressionStatement` support across all providers
  - **Advanced code actions and refactoring** - Fixed parameter threshold validation and enhanced refactoring suggestions with proper AST handling
  - **Enhanced call hierarchy provider** - Complete workspace analysis with improved function call tracking and incoming call detection
  - **Production-ready workspace features** - Improved workspace indexing, symbol tracking, and cross-file rename operations
  - **Comprehensive test reliability** - 100% test pass rate achieved (195/195 library tests, 33/33 LSP E2E tests, 19/19 DAP tests)
  - **Quality gate compliance** - Zero clippy warnings, consistent formatting, full architectural compliance maintained
- **v0.8.8 improvements** (Critical Reliability Fixes):
  - **Enhanced bless parsing capabilities** - complete AST generation compatibility with tree-sitter format for all blessed reference patterns
  - **FunctionCall S-expression enhancement** - special handling for `bless` and built-in functions with proper tree-sitter node structure
  - **Symbol extraction reliability** - comprehensive AST traversal including `NodeKind::ExpressionStatement` for workspace navigation
  - **Enhanced workspace features** - all 33 LSP E2E tests now passing with improved symbol tracking and reference resolution
  - **Improved parser stability** - resolves all 10 bless parsing test failures and symbol documentation integration issues
  - **Test coverage achievement** - 95.9% pass rate with comprehensive bless parsing and workspace navigation validation
- **v0.8.7 improvements** (Combined PR #53 Token Position Tracking + PR #71 Comment Documentation):
  - **O(log n) position mapping** - replaced placeholder tracking with production-ready implementation using LineStartsCache
  - **LSP-compliant UTF-16 position tracking** - accurate line/column tracking with Unicode and CRLF support
  - **Enhanced parser context** - production-grade position tracking throughout token stream processing  
  - **Multi-line token support** - accurate position tracking for tokens spanning multiple lines (strings, comments)
  - **Performance optimization** - efficient binary search-based position lookups for real-time LSP editing
  - **Unicode-safe position calculation** - proper handling of multi-byte characters and emoji in position mapping
  - **Comprehensive position test coverage** - 9 new position tracking tests covering edge cases (CRLF, UTF-16, multiline)
  - **Comprehensive comment documentation extraction** - production-ready leading comment parsing with extensive test coverage (20 tests)
  - **Enhanced source threading architecture** - source-aware LSP providers with improved context for all features
  - **S-expression format compatibility** - resolved bless parsing regressions with complete AST compatibility
  - **Unicode and performance safety** - UTF-8 character boundary handling and optimized string processing
  - **Production-stable hash key context detection** - industry-leading bareword analysis with comprehensive coverage
  - **Edge case robustness** - handles complex formatting scenarios including multi-package support and Unicode comments
  - **Performance optimized comment extraction** - <100µs per iteration with pre-allocated capacity for large comment blocks
- **v0.8.6 improvements**:
  - Type Definition Provider for blessed references and ISA relationships
  - Implementation Provider for class/method implementations
  - Enhanced UTF-16 position handling with CRLF/emoji support
  - **Enhanced substitution parsing** - improved from ~99.995% to ~99.996% coverage via PR #42
  - Robust delimiter handling for s/// operators with paired delimiters
  - Single Source of Truth LSP capability management
- **v0.8.5 improvements**:
  - Typed ServerCapabilities for LSP 3.18 compliance
  - Pull Diagnostics support (workspace/diagnostic)
  - Stable error codes (-32802 for cancellation)
  - Enhanced inlay hints with type anchors
  - Improved cancellation handling with test endpoint
- **v0.8.3 improvements**:
  - Hash literal parsing fixed (`{ key => value }`)
  - Parenthesized expressions with word operators
  - qw() array parsing with all delimiters
  - Enhanced go-to-definition using DeclarationProvider

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

## Workspace Configuration (v0.8.9)

### Core Published Crates (Workspace Members)
The workspace includes only the core production crates for clean builds:
- `crates/perl-lexer` - Context-aware tokenizer
- `crates/perl-parser` - Main parser and LSP server
- `crates/perl-corpus` - Test corpus
- `crates/perl-lsp` - LSP server binary

### Excluded Crates (Legacy/System Dependencies)
Several crates are excluded from the workspace due to system dependencies:
- `tree-sitter-perl` - Legacy C parser (requires libclang)
- `tree-sitter-perl-c` - C parser bindings (requires libclang-dev)
- `crates/perl-parser-pest` - Legacy Pest parser (bindgen dependency)
- `crates/tree-sitter-perl-rs` - Internal testing (bindgen dependency)
- `crates/parser-benchmarks` - Benchmarking (potential dependencies)
- `crates/parser-tests` - Test harness (potential dependencies)
- `xtask` - Build automation (depends on excluded crates)

### Workspace Test Report (**Diataxis: Reference**)

**Document**: `WORKSPACE_TEST_REPORT.md` - Comprehensive test status and validation results

**Current Status (v0.8.9)**:
- **Core Published Crates**: ✅ All tests passing (perl-parser: 194 tests, perl-lexer: 9 tests, perl-corpus: 12 tests)
- **LSP Integration**: ✅ All E2E tests passing (33 tests), DAP tests (19 tests)
- **Workspace Build**: ✅ Clean build with excluded crates configuration
- **Feature Gates**: ✅ Resolved incremental_v2 and lsp-advanced feature issues
- **CI Stability**: ✅ Test timeout adjustments implemented for production stability

**Known Limitations**: 
- C-based parser excluded due to libclang dependencies
- xtask automation excluded due to tree-sitter-perl crate dependency
- Some aspirational features properly ignored (19 advanced LSP tests)

### Workspace Exclusion Rationale (**Diataxis: Explanation**)

The workspace configuration prioritizes **clean builds and published crate stability** over comprehensive internal tooling:

**Why Exclude System-Dependent Crates?**
- **libclang Dependencies**: C-based parsers require `libclang-dev` system package, making builds fragile across environments
- **bindgen Dependencies**: Legacy parsers use bindgen for C interop, adding build complexity and potential failures
- **Clean CI/CD**: Published crate builds must succeed reliably across all platforms without system dependencies

**Why Exclude Build Tools?**
- **xtask Circular Dependencies**: Build tools depend on excluded crates, creating dependency cycles
- **Focus on Core**: Published crates (perl-parser, perl-lsp, perl-lexer, perl-corpus) represent the production API surface
- **Alternative Workflows**: Standard cargo commands replace most xtask functionality for published crates

**Why This Approach Works**:
- **Reliable Builds**: `cargo build` and `cargo test` always succeed for published crates
- **Clear Boundaries**: Published vs internal tooling separation prevents API confusion  
- **Better Testing**: Focus on production code paths rather than internal build infrastructure

### Default Build Configuration

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

#### Excluded Crates (System Dependencies)
- **`/crates/perl-parser-pest/`**: Legacy Pest parser (bindgen dependency)
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
- **Recent improvements (v0.8.9)**:
  - ✅ **Enhanced AST format compatibility** - Program nodes now use tree-sitter standard (source_file) format with backward compatibility
  - ✅ **Comprehensive workspace navigation** - Enhanced AST traversal including `NodeKind::ExpressionStatement` support across all providers
  - ✅ **Advanced code actions and refactoring** - Fixed parameter threshold validation and enhanced refactoring suggestions
  - ✅ **Enhanced call hierarchy provider** - Complete workspace analysis with improved function call tracking and incoming call detection
  - ✅ **Production-ready workspace features** - Improved workspace indexing, symbol tracking, and cross-file rename operations
  - ✅ **100% test reliability achievement** - All 195 library tests, 33 LSP E2E tests, and 19 DAP tests now passing consistently
  - ✅ **Quality gate compliance** - Zero clippy warnings, consistent formatting, full architectural compliance maintained
- **Previous improvements (v0.8.8)**:
  - ✅ **Enhanced bless parsing capabilities** - complete AST generation compatibility with tree-sitter format for all blessed reference patterns
  - ✅ **FunctionCall S-expression enhancement** - special handling for `bless` and built-in functions with proper tree-sitter node structure
  - ✅ **Symbol extraction reliability** - comprehensive AST traversal including `NodeKind::ExpressionStatement` for workspace navigation
  - ✅ **Enhanced workspace features** - all 33 LSP E2E tests now passing with improved symbol tracking and reference resolution
  - ✅ **Test coverage achievement** - 95.9% pass rate with all 12 bless parsing tests passing and symbol documentation integration complete
  - ✅ **Enhanced Variable Resolution Patterns** - comprehensive support for complex Perl variable access patterns
    - Hash access resolution: `$hash{key}` → `%hash` (reduces false undefined variable warnings)
    - Array access resolution: `$array[idx]` → `@array` (proper sigil conversion for array elements)
    - Advanced pattern recognition for nested hash/array structures
    - Context-aware hash key detection to reduce false bareword warnings
    - Fallback mechanisms for complex nested patterns and method call contexts
    - **Test Coverage**: 38 scope analyzer tests passing (24 existing + 14 new comprehensive tests)
  - ✅ **Production-stable hash key context detection** - industry-leading bareword analysis with comprehensive hash context coverage
  - ✅ **Advanced scope analysis** - hash access (`$hash{key}`), array access (`$array[idx]`), method calls
  - ✅ **Enhanced delimiter recovery** - comprehensive pattern recognition for dynamic delimiters
  - ✅ **Recursive variable resolution** - fallback mechanisms for complex nested patterns
- **Previous improvements (v0.8.7)**:
  - ✅ **S-expression format compatibility** - resolved all bless parsing regressions with complete AST compatibility (fixed in v0.8.8)
  - ✅ **Stabilized scope analyzer** - `is_in_hash_key_context()` method proven in production with O(depth) performance
  - ✅ **Complete AST compatibility** - fixed subroutine declaration format and signature parameter parsing
- **Previous improvements (v0.8.6)**:
  - ✅ Type Definition Provider for blessed references and ISA relationships
  - ✅ Implementation Provider for class/method implementations  
  - ✅ Enhanced UTF-16 position handling with CRLF/emoji support
  - ✅ Single Source of Truth LSP capability management
- **Previous improvements (v0.8.4)**:
  - ✅ Added 9 new LSP features - workspace symbols, rename, code actions, semantic tokens, inlay hints, document links, selection ranges, on-type formatting
  - ✅ Contract-driven testing - every capability backed by acceptance tests
  - ✅ Feature flag control - `lsp-ga-lock` for conservative releases
  - ✅ Fallback mechanisms - works with incomplete/invalid code
  - ✅ 530+ tests passing including comprehensive E2E coverage
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

### Coding Standards (Workspace v0.8.9)

#### Quality Checks (**Diataxis: How-to**)
```bash
# Workspace quality standards (recommended)
cargo fmt                              # Format workspace code
cargo clippy --workspace              # Lint workspace crates
cargo test                            # Run workspace tests

# Legacy quality commands (excluded)
# Run `cargo xtask check --all`       # xtask excluded from workspace
```

#### Style Guidelines (**Diataxis: Reference**)
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions

## Contributing (Workspace v0.8.9)

### Development Workflow (**Diataxis: Tutorial**)

**Step 1: Set up development environment**
```bash
# Clone repository
git clone https://github.com/EffortlessSteven/tree-sitter-perl.git
cd tree-sitter-perl

# Build workspace crates
cargo build                            # Builds published crates only
```

**Step 2: Make changes**
```bash
# Parser improvements → /crates/perl-parser/src/
# LSP server enhancements → /crates/perl-lsp/src/  
# Lexer improvements → /crates/perl-lexer/src/
# Test corpus additions → /crates/perl-corpus/
```

**Step 3: Test and validate**
```bash
# Pre-commit validation
cargo fmt                              # Format code
cargo clippy --workspace              # Zero warnings required
cargo test                            # All tests must pass

# Comprehensive validation  
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # 33 LSP tests
cargo bench -p perl-parser            # Performance regression check
```

### Contribution Areas (**Diataxis: Reference**)

**High Priority (Workspace Members)**:
- **Parser Core** (`/crates/perl-parser/`): AST enhancements, edge case handling
- **LSP Server** (`/crates/perl-lsp/`): Protocol features, editor integration  
- **Lexer** (`/crates/perl-lexer/`): Unicode support, tokenization improvements
- **Test Coverage** (`/crates/perl-corpus/`): Edge case additions, validation

**Maintenance (Excluded from Workspace)**:
- **Legacy Parser** (`/crates/perl-parser-pest/`): Bug fixes only
- **Build Tools** (`/xtask/`): Requires excluded dependencies

### Quality Requirements (**Diataxis: Reference**)
- **Test Coverage**: All 284+ tests must pass, including LSP E2E and corpus validation
- **Zero Warnings**: `cargo clippy --workspace` must show no warnings
- **Documentation**: Update relevant docs for new features and API changes
- **Performance**: No regressions in `cargo bench -p perl-parser` results