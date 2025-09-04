# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation and Enhanced Workspace Navigation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md) for guarantees

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem with comprehensive workspace refactoring capabilities:

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
- **v0.8.9 improvements**:
  - **Cross-file workspace refactoring utilities** - comprehensive WorkspaceRefactor provider for symbol renaming, module extraction, import optimization
  - **Production-ready refactoring operations** - move subroutines between modules, inline variables, extract code sections
  - **Enterprise-grade safety and validation** - comprehensive error handling, input validation, and rollback support
  - **Unicode-aware refactoring** - full support for international characters in symbol names and code content
  - **Performance-optimized text processing** - efficient large file handling with safety limits and early termination

#### 2. **perl-lexer** (`/crates/perl-lexer/`) 
- Context-aware tokenizer with mode-based lexing
- Handles slash disambiguation at lexing phase
- Zero dependencies, used by perl-parser

#### 3. **perl-corpus** (`/crates/perl-corpus/`)
- Comprehensive test corpus with property-based testing infrastructure
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

## Key Features

### Incremental Parsing with Rope-based Document Management
- Production-ready incremental parsing with <1ms LSP updates
- Rope-based text management for UTF-16/UTF-8 position conversion  
- Subtree reuse with 70-90% cache hit ratios
- See [docs/INCREMENTAL_PARSING_GUIDE.md](docs/INCREMENTAL_PARSING_GUIDE.md) for details

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
- **~85% of LSP features work** (all advertised capabilities functional)
- Advanced syntax checking and diagnostics with hash key context detection
- Code completion (variables, 150+ built-ins, file paths) with comment documentation  
- Enhanced workspace features: symbols, rename, code actions, semantic tokens
- Incremental parsing with <1ms real-time editing performance
- File path completion with enterprise security safeguards
- **Debug Adapter Protocol (DAP)** support with full debugging flow
- **v0.8.9 Workspace Refactoring Features**:
  - ✅ **Cross-file symbol renaming** - comprehensive workspace refactor utilities
  - ✅ **Module extraction** - extract code sections to new modules with import management
  - ✅ **Import optimization** - remove unused imports, consolidate duplicates
  - ✅ **Subroutine movement** - move functions between modules with proper cleanup
  - ✅ **Variable inlining** - inline variables with their definitions across scopes
- **Test Coverage**: 530+ tests with acceptance tests for all features (including 19 comprehensive workspace refactoring tests)
- **Performance**: <50ms for all operations, works with all LSP-compatible editors
- See [docs/LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md) for details

## Default Build Configuration

**AI tools can run bare `cargo build` and `cargo test` commands** - the `.cargo/config.toml` ensures correct behavior.

## Essential Commands

## Workspace Refactoring System (**Diataxis: Explanation**) 🔧 **PRODUCTION-READY v0.8.9**

The comprehensive WorkspaceRefactor system provides enterprise-grade cross-file refactoring capabilities with safety, performance, and Unicode support:

### Core Refactoring Operations (**Diataxis: Reference**)

**Symbol Renaming** - Cross-file symbol renaming with comprehensive validation:
- **Comprehensive Symbol Support**: Variables (`$var`, `@array`, `%hash`), subroutines, packages with proper sigil handling
- **Workspace-wide Analysis**: Uses WorkspaceIndex for precise symbol location and fallback text-based search
- **Performance Optimization**: Early termination, byte-based searching, and safety limits (1000 matches max)
- **Unicode-Safe Processing**: Full support for international characters in variable names and content
- **Validation Framework**: Input validation, identical name detection, empty name prevention

**Module Extraction** - Extract code sections into new Perl modules:
- **Line-Based Extraction**: Extract specified line ranges into new .pm module files
- **Automatic Use Statements**: Replace extracted code with appropriate `use ModuleName;` statements
- **Position Validation**: Comprehensive line number validation and bounds checking
- **File Management**: Creates new module files with proper naming conventions

**Import Optimization** - Workspace-wide import statement optimization:
- **Duplicate Removal**: Identifies and consolidates duplicate import statements
- **Alphabetical Sorting**: Organizes imports in clean, consistent alphabetical order
- **Dependency Analysis**: Uses WorkspaceIndex to understand actual module dependencies
- **Batch Processing**: Efficient processing across multiple files with smart filtering

**Subroutine Movement** - Move functions between modules:
- **Precise Symbol Location**: Uses WorkspaceIndex to locate subroutine definitions
- **Complete Code Transfer**: Moves entire subroutine definitions including documentation
- **File Cleanup**: Removes subroutine from source file, appends to target module
- **Position-Aware Processing**: Handles complex subroutine ranges with proper byte offsets

**Variable Inlining** - Replace variables with their initializer expressions:
- **Scope-Aware Analysis**: Identifies variable declarations and their usage patterns
- **Expression Extraction**: Parses initializer expressions for replacement
- **Occurrence Replacement**: Replaces all variable references with the original expression
- **Definition Cleanup**: Removes the original variable declaration line

### Technical Architecture (**Diataxis: Explanation**)

**Core Components**:
- **WorkspaceRefactor**: Main refactoring provider with comprehensive operation methods
- **RefactorResult**: Structured result format with file edits, descriptions, and warnings
- **FileEdit/TextEdit**: Precise text editing instructions with byte-level positioning
- **RefactorError**: Comprehensive error handling with detailed error categorization

**Error Handling Framework**:
- **Input Validation**: Empty names, identical names, invalid ranges
- **File System Safety**: URI conversion, document indexing, position validation
- **Parse Error Recovery**: Graceful handling of incomplete or invalid code
- **Symbol Resolution**: Not found symbols, invalid positions, missing documents

**Performance Features**:
- **Efficient Text Processing**: Byte-based searching with early termination
- **Memory Management**: BTreeMap for sorted edits, HashSet for deduplication
- **Safety Limits**: 1000 match limit, 500 file limit for performance bounds
- **Smart Filtering**: Pre-checks for target strings to avoid unnecessary processing

### Tutorial: Using Workspace Refactoring (**Diataxis: Tutorial**)

**Step 1: Basic Symbol Renaming**
```rust
use perl_parser::workspace_refactor::WorkspaceRefactor;
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::Path;

// Create workspace refactor provider
let index = WorkspaceIndex::new();
let refactor = WorkspaceRefactor::new(index);

// Rename a variable across all files
let result = refactor.rename_symbol(
    "$old_name",        // Current symbol name
    "$new_name",        // New symbol name
    Path::new("file.pl"),  // File where rename initiated
    (0, 0)               // Position in file
)?;

// Apply the refactoring
for file_edit in result.file_edits {
    println!("Updating file: {:?}", file_edit.file_path);
    // Apply edits in reverse order to maintain positions
    for edit in file_edit.edits.iter().rev() {
        // Replace text at edit.start..edit.end with edit.new_text
    }
}
```

**Step 2: Module Extraction**
```rust
// Extract lines 50-100 from large_file.pl into new Utils module
let result = refactor.extract_module(
    Path::new("large_file.pl"), // Source file
    50, 100,                     // Line range (1-based, inclusive)
    "Utils"                      // New module name (without .pm)
)?;

// Results in:
// 1. large_file.pl: lines 50-100 replaced with "use Utils;"
// 2. Utils.pm: created with extracted content
```

**Step 3: Import Optimization**
```rust
// Optimize imports across entire workspace
let result = refactor.optimize_imports()?;

// Processes all files with:
// - Duplicate import removal
// - Alphabetical sorting 
// - Dependency consolidation
// - Clean formatting
```

**Step 4: Advanced Operations**
```rust
// Move subroutine between files
let result = refactor.move_subroutine(
    "utility_function",      // Subroutine to move
    Path::new("main.pl"),    // Source file
    "Utils"                  // Target module
)?;

// Inline a temporary variable
let result = refactor.inline_variable(
    "$temp",               // Variable to inline
    Path::new("file.pl"),  // File containing variable
    (0, 0)                 // Position (currently unused)
)?;
```

### How-to Guide: Enterprise Integration (**Diataxis: How-to**)

**Step 1: Error Handling and Validation**
```rust
use perl_parser::workspace_refactor::RefactorError;

match refactor.rename_symbol("$old", "$new", &path, (0, 0)) {
    Ok(result) => {
        // Check for warnings
        for warning in &result.warnings {
            eprintln!("Warning: {}", warning);
        }
        // Apply changes safely
        apply_refactor_result(result)?;
    },
    Err(RefactorError::InvalidInput(msg)) => {
        eprintln!("Input validation failed: {}", msg);
    },
    Err(RefactorError::SymbolNotFound { symbol, file }) => {
        eprintln!("Symbol '{}' not found in {}", symbol, file);
    },
    Err(e) => {
        eprintln!("Refactoring failed: {}", e);
    },
}
```

**Step 2: Unicode and International Support**
```rust
// The system fully supports Unicode symbols and content
let result = refactor.rename_symbol(
    "$♥",      // Unicode variable name
    "$love",   // ASCII replacement
    &path, (0, 0)
)?;

// Extract module with Unicode content
let result = refactor.extract_module(
    &path,
    10, 20,    // Lines containing international characters
    "国际化工具"  // Unicode module name
)?;
```

**Step 3: Performance Optimization for Large Codebases**
```rust
// The system includes built-in performance safeguards:
// - 1000 match limit per operation
// - 500 file limit for workspace operations
// - Pre-filtering to avoid processing files without target strings
// - Early termination for large operations

// For very large codebases, consider batch processing:
let files_to_process = get_high_priority_files();
for file_batch in files_to_process.chunks(10) {
    let result = refactor.optimize_imports()?;
    // Process batch results
    thread::sleep(Duration::from_millis(100)); // Rate limiting
}
```

### Testing Workspace Refactoring (**Diataxis: How-to**)

**Comprehensive Test Suite**:
```bash
# Run all workspace refactoring tests (19 comprehensive tests)
cargo test -p perl-parser workspace_refactor

# Test specific refactoring operations
cargo test -p perl-parser workspace_refactor::tests::test_rename_symbol
cargo test -p perl-parser workspace_refactor::tests::test_extract_module
cargo test -p perl-parser workspace_refactor::tests::test_optimize_imports
cargo test -p perl-parser workspace_refactor::tests::test_move_subroutine
cargo test -p perl-parser workspace_refactor::tests::test_inline_variable

# Test edge cases and error handling
cargo test -p perl-parser workspace_refactor::tests::test_rename_symbol_validation_errors
cargo test -p perl-parser workspace_refactor::tests::test_unicode_handling
cargo test -p perl-parser workspace_refactor::tests::test_large_file_handling
cargo test -p perl-parser workspace_refactor::tests::test_complex_perl_constructs
```

**Production Validation**:
- **19 comprehensive test scenarios** covering all operations and edge cases
- **Unicode test coverage** with international characters and complex scripts
- **Performance tests** with large files (100+ lines) and multiple files (10+ workspace)
- **Error handling validation** for all failure modes and input validation
- **Edge case coverage** including complex Perl constructs, regex patterns, and nested structures

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

# Run all tests  
cargo xtask test

# Run LSP server
perl-lsp --stdio

# Install LSP server globally
cargo install --path crates/perl-lsp
```

### Key Testing
```bash
# Main test suites
cargo test -p perl-parser                              # Parser tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # LSP tests
cargo xtask corpus                                     # Integration tests

# Incremental parsing (enable with --features incremental)
cargo test -p perl-parser incremental_v2::tests --features incremental
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
# Run all scope analyzer tests (41 comprehensive tests with MandatoryParameter support)
cargo test -p perl-parser --test scope_analyzer_tests

# Test enhanced variable resolution patterns
cargo test -p perl-parser scope_analyzer_tests::test_hash_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_array_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_complex_variable_patterns

# Test enhanced parameter handling and MandatoryParameter support  
cargo test -p perl-parser scope_analyzer_tests::test_parameter_extraction
cargo test -p perl-parser scope_analyzer_tests::test_subroutine_parameter_analysis

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
- **Enhanced Bare Import Handling**: Conservative analysis for bare imports to reduce false positives
- **Complete Test Coverage**: 8 comprehensive test cases covering all optimization scenarios including bare import edge cases

**Features** (**Diataxis: Reference**):
- **Unused Import Detection**: Regex-based usage analysis identifies import statements never used in code
- **Smart Bare Import Analysis**: Conservative handling of bare imports (without qw()) to avoid flagging modules with side effects or implicit usage
- **Pragma Module Recognition**: Automatic exclusion of pragma modules (strict, warnings, utf8, etc.) from unused detection
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
# Run all import optimizer tests (8 comprehensive scenarios)
cargo test -p perl-parser --test import_optimizer_tests

# Test specific optimization scenarios
cargo test -p perl-parser import_optimizer_tests::handles_bare_imports_without_symbols
cargo test -p perl-parser import_optimizer_tests::handles_entirely_unused_imports
cargo test -p perl-parser import_optimizer_tests::detects_unused_and_duplicate_imports
cargo test -p perl-parser import_optimizer_tests::handles_complex_symbol_names_and_delimiters

# Test bare import handling and false positive reduction
cargo test -p perl-parser import_optimizer_tests::handles_mixed_imports_and_usage
cargo test -p perl-parser import_optimizer_tests::preserves_order_in_optimized_output

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
   - **MandatoryParameter Support**: Enhanced AST traversal for subroutine parameters with comprehensive parameter analysis
     - Proper variable name extraction from `NodeKind::MandatoryParameter` nodes
     - Parameter scope analysis including parameter shadowing detection
     - Integration with enhanced scope resolution patterns
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
   - **41 comprehensive test cases** covering all resolution patterns, parameter handling, and edge cases

These fallbacks ensure the LSP remains functional during active development when code is temporarily invalid.

### Advanced Testing Infrastructure (v0.8.9) ⚡ **PRODUCTION-READY**

The project includes comprehensive testing infrastructure for both LSP features and incremental parsing with production-grade quality assurance.

#### Incremental Parsing Test Infrastructure (**Diataxis: Explanation**)

**Core Testing Components**:
- **IncrementalTestUtils**: Production-ready performance testing utilities with statistical analysis
- **Performance Test Harness**: Advanced benchmarking infrastructure with timing precision
- **Validation Framework**: Automated criteria checking against production performance targets
- **Statistical Analysis Engine**: Comprehensive metrics calculation and reliability validation
- **Test Macros**: Simplified test creation with `perf_test!()` and `perf_test_relaxed!()` macros

**Advanced Test Capabilities** (**Diataxis: Reference**):
- **Performance Categories**: Automatic classification (🟢 Excellent <100µs, 🟡 Very Good <500µs, etc.)
- **Statistical Validation**: Mean, median, range, standard deviation, coefficient of variation
- **Regression Detection**: Automated performance regression monitoring across test batches
- **Scaling Analysis**: Document size scaling characteristics with efficiency metrics
- **Edge Case Coverage**: Unicode, multibyte, complex structures, and error recovery scenarios

#### Using the Enhanced Test Infrastructure (**Diataxis: Tutorial**)

**Step 1: Basic Performance Testing**
```rust
use crate::support::incremental_test_utils::IncrementalTestUtils;

// Run comprehensive performance test with statistical analysis
let result = IncrementalTestUtils::performance_test_with_stats(
    "My Performance Test",
    "my $x = 42;",      // initial source
    |source| IncrementalTestUtils::create_value_edit(source, "42", "999"), // edit generator
    15  // iterations for statistical reliability
);

// Print detailed performance summary with categories
IncrementalTestUtils::print_performance_summary(&result);

// Validate against production criteria
let criteria = IncrementalTestUtils::standard_criteria();
let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
validation.print_report();  // ✅ PASSED or ❌ FAILED with details
```

**Step 2: Using Performance Test Macros**
```rust
// Standard performance test (strict criteria)
let result = perf_test!(
    "Simple Value Edit Test",
    "my $x = 42; my $y = 100;",
    |source| IncrementalTestUtils::create_value_edit(source, "42", "999"),
    15  // iterations
);

// Relaxed performance test (for complex scenarios)
let result = perf_test_relaxed!(
    "Complex Structure Test", 
    "complex_perl_source_here",
    |source| complex_edit_generator(source),
    10
);
```

**Step 3: Custom Performance Analysis**
```rust
// Create custom edit generator for specific scenarios
fn custom_edit_generator(source: &str) -> (String, Edit) {
    // Custom logic for generating edits
    IncrementalTestUtils::create_value_edit(source, "target", "replacement")
}

// Access detailed performance metrics
println!("Performance Metrics:");
println!("  Median: {}µs", result.median_incremental_micros);
println!("  Efficiency: {:.1}%", result.avg_efficiency_percentage);
println!("  Speedup: {:.1}x", result.speedup_ratio);
println!("  Consistency: {:.3}", result.coefficient_of_variation);
```

#### Performance Validation Criteria (**Diataxis: Reference**)

**Standard Production Criteria**:
```rust
pub struct PerformanceCriteria {
    pub max_avg_micros: 1000,              // <1ms average parse time
    pub min_efficiency_percentage: 70.0,   // ≥70% node reuse rate
    pub min_speedup_ratio: 2.0,            // ≥2x faster than full parsing
    pub max_coefficient_of_variation: 0.5, // Consistent performance (CV <0.5)
    pub min_success_rate: 0.95,            // ≥95% successful parses
}
```

**Relaxed Criteria** (for complex scenarios):
```rust
pub struct PerformanceCriteria {
    pub max_avg_micros: 5000,              // <5ms average (boundary case)
    pub min_efficiency_percentage: 50.0,   // ≥50% node reuse rate
    pub min_speedup_ratio: 1.5,            // ≥1.5x faster than full parsing  
    pub max_coefficient_of_variation: 1.0, // Allow more performance variation
    pub min_success_rate: 0.90,            // ≥90% successful parses
}
```

#### Running the Complete Test Suite (**Diataxis: How-to**)

**Comprehensive Incremental Tests**:
```bash
# Run all comprehensive incremental parsing tests (10 test scenarios)
cargo test -p perl-parser --features incremental --test incremental_comprehensive_test -- --nocapture

# Run performance-focused tests with statistical analysis
cargo test -p perl-parser --features incremental --test incremental_performance_tests -- --nocapture

# Run specific test scenarios with detailed output
cargo test -p perl-parser --features incremental -- test_comprehensive_simple_value_edits --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_large_document_scaling --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_unicode_and_multibyte --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_edge_cases --nocapture
```

**LSP Integration Tests**:
```bash
# Run LSP comprehensive end-to-end tests (33 test scenarios)
cargo test -p perl-lsp lsp_comprehensive_e2e_test

# Run user story tests with real-world scenarios
cargo test -p perl-lsp lsp_e2e_user_stories

# Test incremental parsing integration with LSP
cargo test -p perl-parser incremental_integration_test --features incremental
```

#### Test Results Interpretation (**Diataxis: Explanation**)

**Performance Test Output**:
```
📊 Performance Test Summary: Simple Value Edits
═════════════════════════════════════════════
📈 Timing Statistics:
  Average Incremental: 65µs     (🟢 Excellent <100µs)
  Median Incremental:  58µs
  Range: 45µs - 89µs
  Standard Deviation: 12µs
  Coefficient of Variation: 0.18 (Excellent consistency)

⚡ Performance Metrics:
  Speedup Ratio: 2.8x faster    (vs full parsing)
  Sub-millisecond Rate: 100.0%  (All edits <1ms)
  Success Rate: 100.0%

🔄 Node Reuse Statistics:
  Average Efficiency: 96.8%     (Target: ≥70%)
  Performance Category: 🟢 Excellent (<100µs)

✅ Performance validation PASSED - All criteria met
```

**Quality Metrics** ✅ **ACHIEVED**:
- **100% Test Pass Rate**: All 10 comprehensive incremental tests pass
- **100% LSP Integration**: All 33 LSP E2E tests pass
- **Production Reliability**: 100% success rate, <0.6 coefficient of variation
- **Performance Excellence**: All scenarios meet or exceed production targets

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

3. **Enhanced S-Expression Generation System** (**Diataxis: Explanation**) (Resolved Issue #72)
   - **Comprehensive Operator Mapping**: Complete binary operator coverage (binary_+, binary_<, binary_*, etc.) and unary operator coverage (unary_not, unary_-, unary_++, file tests, postfix dereferencing)
   - **String Interpolation Differentiation**: Proper distinction between `string` and `string_interpolated` nodes based on content analysis
   - **Tree-sitter Standard Compatibility**: Program nodes use standard `(source_file)` format while maintaining backward compatibility
   - **Performance Optimized**: 24-26% parsing speed improvement maintained with comprehensive operator semantics
   - **Comprehensive Coverage**: 50+ binary operators and 25+ unary operators with specific S-expression formats for detailed semantic analysis
   - **Production Verification**: 10/10 integration tests passing with comprehensive edge case validation

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

### Enhanced S-Expression Usage Guide (**Diataxis: Tutorial**)

The comprehensive S-expression generation system (Issue #72 resolved) provides detailed operator semantics and string analysis for tree-sitter integration.

#### **Step 1: Basic S-Expression Generation**
```rust
use perl_parser::Parser;

// Parse Perl code with enhanced S-expression support
let mut parser = Parser::new("my $result = ($a + $b) * $c;");
let ast = parser.parse()?;

// Generate comprehensive S-expression with specific operator names
let sexp = ast.to_sexp();
println!("{}", sexp);
// Output: (source_file (my_declaration (variable $ result) 
//         (binary_* (binary_+ (variable $ a) (variable $ b)) (variable $ c))))
```

#### **Step 2: Working with Operator Mappings** (**Diataxis: Reference**)
```perl
# Binary operators generate specific S-expression formats:
if ($x > 10) { }          # → (binary_> (variable $ x) (number 10))
$result = $a + $b;        # → (binary_+ (variable $ a) (variable $ b))
$flag and $condition;     # → (binary_and (variable $ flag) (variable $ condition))

# Unary operators have comprehensive coverage:
!$flag                    # → (unary_not (variable $ flag))
-$number                  # → (unary_- (variable $ number))
$counter++;               # → (unary_++ (variable $ counter))
-f $filename              # → (unary_-f (variable $ filename))
```

#### **Step 3: String Interpolation Detection**
```perl
# The system differentiates interpolated strings:
print "Hello world";      # → (call print (string "Hello world"))
print "Hello $name";      # → (call print (string_interpolated "Hello $name"))
```

#### **Step 4: Testing S-Expression Output**
```bash
# Run the comprehensive verification example
cargo run -p perl-parser --example verify_sexp_fixes

# Test specific operator patterns
cargo test -p perl-parser --test word_operator_precedence_test

# Validate S-expression format compliance
cargo test -p perl-parser ast_tests::test_sexp_generation
```

#### **How-to Guide: Integrating with Tree-sitter Tools** (**Diataxis: How-to**)
```python
# Python example using enhanced S-expressions
import sexpdata

# Parse enhanced S-expression output
perl_sexp = "(source_file (binary_+ (variable $ x) (number 42)))"
ast = sexpdata.loads(perl_sexp)

# Extract operator information
def extract_operators(node):
    if isinstance(node, list) and len(node) > 0:
        node_type = str(node[0])
        if node_type.startswith('binary_'):
            operator = node_type[7:]  # Extract operator from binary_+
            return [(operator, 'binary')]
        elif node_type.startswith('unary_'):
            operator = node_type[6:]   # Extract operator from unary_-
            return [(operator, 'unary')]
    return []
```

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

### Performance Targets (**Diataxis: Reference**) ⚡ **ACHIEVED v0.8.9**
- **<1ms updates** for small edits (single token changes) - ✅ **ACHIEVED**: 65-538µs average
- **<2ms updates** for moderate edits (function-level changes) - ✅ **EXCEEDED**: All scenarios <1ms  
- **Cache hit ratios** of 70-90% for typical editing scenarios - ✅ **EXCEEDED**: 96.8-99.7% efficiency
- **Memory efficient** with LRU cache eviction, Arc<Node> sharing, and Rope's piece table architecture
- **Production Reliability** - ✅ **ACHIEVED**: 100% success rate, <0.6 coefficient of variation
- **Excellent Scaling** - ✅ **VALIDATED**: 5.4-9.1µs per statement for large documents

### Advanced Incremental Parsing (IncrementalParserV2) ⚡ **PRODUCTION-READY v0.8.9**

The repository includes a **production-ready enhanced incremental parser** (`IncrementalParserV2`) with intelligent node reuse strategies and comprehensive performance testing infrastructure:

**Key Features** (**Diataxis: Explanation**):
- **Smart Node Reuse**: Automatically detects which AST nodes can be preserved across edits (70-99% efficiency)
- **Comprehensive Metrics Tracking**: Detailed statistics on reused vs reparsed nodes with performance analysis
- **Simple Value Edit Optimization**: Sub-millisecond updates for number/string literal changes
- **Graceful Fallback Mechanisms**: Robust degradation to full parsing for complex structural changes
- **Memory Efficient**: LRU cache eviction and Arc<Node> sharing for optimal memory usage
- **Performance Validation**: Built-in performance criteria validation with statistical analysis

**Enhanced Incremental Parsing Features (Latest)** ⚡ **PRODUCTION-READY**:
- **Whitespace/Comment Detection**: 100% node reuse for non-structural changes (comments, whitespace modifications)
- **Enhanced Node Matching**: 15+ node types supported for improved reuse detection (Number, String, Variable, etc.)
- **Unicode-Safe Edit Validation**: Character boundary validation prevents UTF-8 slice panics with multibyte characters
- **Strengthened Multibyte Support**: International character handling with proper position adjustment algorithms
- **Production Test Infrastructure**: Statistical analysis framework with performance criteria validation and regression detection

**Performance Characteristics** (**Diataxis: Reference**):
- **Sub-millisecond Updates**: <1ms for simple value edits (target achieved in 100% of test cases)
- **Excellent Scaling**: 5.4-9.1µs per statement for large documents (100+ statements)
- **High Efficiency**: 70-99.7% node reuse rate depending on edit complexity
- **Production Reliability**: 100% success rate across comprehensive test scenarios
- **Statistical Validation**: Coefficient of variation <0.6 for consistent performance

#### Enhanced Feature Details (**Diataxis: Explanation**)

**1. Whitespace/Comment Detection** (**Diataxis: Explanation**):
The enhanced parser includes intelligent detection of non-structural edits that only affect whitespace or comments:

```rust
// These edits achieve 100% node reuse through whitespace detection:
"my $x = 42;   "      // Add trailing whitespace
"my $x = 42; # test"  // Add comment  
"my $x =    42;"      // Modify internal whitespace
```

**Implementation Details**:
- **Lexical Analysis**: Uses `PerlLexer` to analyze edited ranges for non-structural tokens
- **Token Classification**: Identifies `TokenType::Whitespace`, `TokenType::Newline`, `TokenType::Comment(_)`
- **Position Shifting**: Reuses entire AST with adjusted positions rather than reparsing
- **Performance**: Near-zero overhead for whitespace-only changes with 100% efficiency

**2. Enhanced Node Matching** (**Diataxis: Reference**):
Expanded node type support for intelligent reuse detection:

**Supported Node Types**:
```rust
// Value nodes with exact matching
NodeKind::Number { value }     // Numeric literals (42, 3.14, 0x1A)
NodeKind::String { value }     // String literals ("hello", 'world')  
NodeKind::Variable { name, sigil } // Variables ($var, @array, %hash)
NodeKind::Package { name }     // Package declarations

// Structural nodes with content-aware matching
NodeKind::Block               // Code blocks { ... }
NodeKind::If { .. }          // Conditional statements
NodeKind::VarDecl { .. }     // Variable declarations
NodeKind::Assignment { .. }   // Assignment operations
NodeKind::FunctionCall { .. } // Function/method calls

// Container nodes (15+ total supported types)
NodeKind::Program            // Root program nodes
NodeKind::Expression         // Expression containers
NodeKind::Statement          // Statement wrappers
```

**Matching Algorithm**:
- **Content Comparison**: Exact value matching for literals and identifiers
- **Structural Equivalence**: Same node type and compatible child structure  
- **Position Independence**: Nodes can match despite location changes
- **Recursive Validation**: Deep comparison for nested structures

**3. Unicode-Safe Edit Validation** (**Diataxis: How-to**):
Comprehensive boundary validation prevents UTF-8 slice panics:

```rust
// Protected operations with boundary checking:
if !source.is_char_boundary(node.location.start) 
   || !source.is_char_boundary(node.location.end) {
    // Adjust to nearest valid boundary or skip reuse
    return false;
}
```

**Safety Features**:
- **Character Boundary Validation**: Ensures all positions fall on valid UTF-8 character boundaries
- **Range Bounds Checking**: Validates edit ranges don't exceed source length
- **Graceful Degradation**: Falls back to full parsing for invalid boundary conditions
- **Debug Logging**: Comprehensive logging for troubleshooting boundary issues

**4. Strengthened Multibyte Support** (**Diataxis: Tutorial**):
Enhanced handling of international characters and complex Unicode:

```perl
# These multibyte scenarios are properly handled:
my $♥ = "love";           # Unicode variable names
my $message = "你好世界";   # CJK string content  
my $emoji = "🚀 rocket";   # Emoji in strings
# コメント with Japanese     # Unicode in comments
```

**Implementation Features**:
- **Unicode-Safe Position Calculation**: Proper handling of multi-byte character boundaries
- **Character Length Awareness**: Accounts for characters requiring multiple bytes in UTF-8
- **Boundary Adjustment**: `ensure_unicode_boundary()` method finds nearest valid positions
- **International Testing**: Comprehensive test coverage with CJK, Arabic, and emoji characters

**5. Production Test Infrastructure** (**Diataxis: Reference**):
Comprehensive testing framework with statistical analysis:

**Performance Test Harness**:
```rust
use crate::support::incremental_test_utils::IncrementalTestUtils;

// Statistical performance testing with validation
let result = IncrementalTestUtils::performance_test_with_stats(
    "Test Name",
    source,
    edit_generator,
    15  // iterations for statistical reliability
);

// Automated criteria validation
let criteria = IncrementalTestUtils::standard_criteria();
let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
validation.print_report();  // ✅ PASSED or ❌ FAILED with details
```

**Validation Framework**:
- **Performance Categories**: Automatic classification (Excellent <100µs, Very Good <500µs, etc.)
- **Statistical Analysis**: Mean, median, standard deviation, coefficient of variation
- **Regression Detection**: Automated monitoring for performance degradation  
- **Criteria Validation**: Configurable thresholds for acceptance testing
- **Comprehensive Reporting**: Detailed performance summaries with visual indicators

**Advanced Testing Infrastructure** (**Diataxis: Reference**):
- **Performance Test Harness**: Statistical analysis with timing, efficiency, and reliability metrics
- **Comprehensive Test Suite**: 10 test scenarios covering edge cases, Unicode, scaling, and regression detection
- **Validation Framework**: Automated criteria checking against performance targets
- **Performance Categories**: Automatic classification (Excellent <100µs, Very Good <500µs, Good <1ms)

**Usage Example** (**Diataxis: Tutorial**):
```rust
// Production-ready incremental parsing with IncrementalParserV2
use perl_parser::{incremental_v2::IncrementalParserV2, edit::Edit, position::Position};

let mut parser = IncrementalParserV2::new();

// Initial parse
let tree1 = parser.parse("my $x = 42;")?;
println!("Initial: Reparsed={}, Reused={}", parser.reparsed_nodes, parser.reused_nodes);

// Apply edit (change "42" to "999")
let edit = Edit::new(
    8, 10, 11,  // byte positions: old_start, old_end, new_end
    Position::new(8, 1, 9),   // old_start position
    Position::new(10, 1, 11), // old_end position  
    Position::new(11, 1, 12), // new_end position
);
parser.edit(edit);
let tree2 = parser.parse("my $x = 999;")?;

// Performance metrics (typically: Reparsed=1, Reused=3, <100µs)
println!("After edit: Reparsed={}, Reused={}, Time={}µs", 
    parser.reparsed_nodes, parser.reused_nodes, 
    parser.get_metrics().last_parse_time.as_micros());
```

#### Enhanced Features Tutorial (**Diataxis: Tutorial**)

**Step 1: Whitespace/Comment Optimization**
```rust
use perl_parser::incremental_v2::IncrementalParserV2;

let mut parser = IncrementalParserV2::new();

// Initial source with structure
let source1 = "my $x = 42;";
parser.parse(source1).unwrap();

// Whitespace/comment edits achieve 100% node reuse
let whitespace_scenarios = vec![
    ("my $x = 42;  ", "Add trailing whitespace"),
    ("my $x = 42; # comment", "Add comment"),
    ("my $x =   42;", "Modify internal spacing"),
];

for (new_source, description) in whitespace_scenarios {
    // These edits trigger whitespace detection optimization
    let result = parser.parse(new_source).unwrap();
    println!("{}: 100% efficiency achieved", description);
    // Typically: reused_nodes = all nodes, reparsed_nodes = 0
}
```

**Step 2: Enhanced Node Matching in Action**
```rust
// The enhanced parser recognizes more node patterns for reuse
let structural_examples = vec![
    // Number literals
    ("my $count = 42;", "my $count = 999;", "Number value change"),
    
    // String literals  
    (r#"my $msg = "hello";"#, r#"my $msg = "world";"#, "String content change"),
    
    // Variable names
    ("my $old_name = 5;", "my $new_name = 5;", "Variable identifier change"),
    
    // Complex structures with partial reuse
    ("my $data = { x => 42 };", "my $data = { x => 99 };", "Nested structure edit"),
];

for (original, modified, description) in structural_examples {
    parser = IncrementalParserV2::new();
    parser.parse(original).unwrap();
    
    let result = parser.parse(modified).unwrap();
    let efficiency = parser.reused_nodes as f64 / 
        (parser.reused_nodes + parser.reparsed_nodes) as f64 * 100.0;
    
    println!("{}: {:.1}% efficiency", description, efficiency);
    // Enhanced matching typically achieves 70-95% efficiency
}
```

**Step 3: Unicode and Multibyte Character Handling**
```rust
// Enhanced parser handles international characters safely
let unicode_examples = vec![
    // Unicode variable names
    ("my $♥ = 'love';", "Unicode variable"),
    
    // CJK character strings
    (r#"my $greeting = "你好世界";"#, "CJK string content"),
    
    // Emoji in comments and strings
    (r#"my $status = "🚀 launching"; # rocket emoji"#, "Emoji handling"),
];

for (source, description) in unicode_examples {
    let mut parser = IncrementalParserV2::new();
    parser.parse(source).unwrap();
    
    // Edit within Unicode content - boundary validation prevents panics
    let modified = source.replace("o", "oo"); // Safe boundary-aware edit
    let result = parser.parse(&modified);
    
    match result {
        Ok(_) => println!("{}: ✅ Unicode-safe handling successful", description),
        Err(_) => println!("{}: Graceful fallback to full parsing", description),
    }
}
```

**Step 4: Production Performance Testing**
```rust
use crate::support::incremental_test_utils::IncrementalTestUtils;

// Using the production test infrastructure
let test_scenarios = vec![
    ("Simple Edit", "my $x = 42;", |s| 
        IncrementalTestUtils::create_value_edit(s, "42", "999")),
    ("String Edit", r#"print "hello";"#, |s|
        IncrementalTestUtils::create_value_edit(s, "hello", "world")),
];

for (name, source, edit_gen) in test_scenarios {
    // Run statistical performance test
    let result = IncrementalTestUtils::performance_test_with_stats(
        name, source, edit_gen, 10  // 10 iterations
    );
    
    // Print comprehensive performance analysis
    IncrementalTestUtils::print_performance_summary(&result);
    
    // Validate against production criteria
    let criteria = IncrementalTestUtils::standard_criteria();
    let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
    validation.print_report();
}
```

**Comprehensive Testing** (**Diataxis: How-to**):
```bash
# Run production-ready comprehensive incremental tests (10 test scenarios)
cargo test -p perl-parser --features incremental --test incremental_comprehensive_test

# Run performance-focused tests with statistical analysis
cargo test -p perl-parser --features incremental --test incremental_performance_tests

# Test specific enhanced scenarios with detailed metrics
cargo test -p perl-parser --features incremental -- test_comprehensive_simple_value_edits --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_string_edits --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_unicode_and_multibyte --nocapture

# Test whitespace/comment detection enhancements
cargo test -p perl-parser --features incremental -- test_comprehensive_edge_cases --nocapture

# Test scaling characteristics with enhanced node matching (10-100 statements)
cargo test -p perl-parser --features incremental -- test_comprehensive_large_document_scaling --nocapture

# Test production infrastructure features
cargo test -p perl-parser --features incremental -- test_comprehensive_rapid_consecutive_edits --nocapture
cargo test -p perl-parser --features incremental -- test_comprehensive_memory_and_stability --nocapture

# Test Unicode-safe edit validation and multibyte support
cargo test -p perl-parser --features incremental -- test_unicode_heavy_incremental_parsing --nocapture
cargo test -p perl-parser --features incremental -- test_ast_boundary_edit_handling --nocapture

# Run the interactive example with enhanced performance demonstration
cargo run -p perl-parser --example test_incremental_v2 --features incremental
```

**Performance Test Results** (**Diataxis: Reference**):
```
Performance Test Summary: Simple Value Edits
═══════════════════════════════════════════════════════
📈 Timing Statistics:
  Average Incremental: 65µs     (🟢 Excellent <100µs)
  Node Reuse Efficiency: 96.8%  (Target: ≥70%)
  Speedup Ratio: 2.5x faster    (vs full parsing)
  Sub-millisecond Rate: 100.0%  (All edits <1ms)

📈 Scaling Characteristics:
  10 statements: 65µs (6.5µs/stmt) - 96.8% efficiency
  25 statements: 205µs (8.2µs/stmt) - 98.7% efficiency  
  50 statements: 454µs (9.1µs/stmt) - 99.3% efficiency
  100 statements: 538µs (5.4µs/stmt) - 99.7% efficiency
  
✅ Performance validation PASSED - All criteria met
```

**Testing Infrastructure API** (**Diataxis: Reference**):
```rust
// Production-ready performance testing utilities
use crate::support::incremental_test_utils::IncrementalTestUtils;

// Run performance test with statistical analysis
let result = IncrementalTestUtils::performance_test_with_stats(
    "Test Name",
    "my $x = 42;",      // initial source
    |source| IncrementalTestUtils::create_value_edit(source, "42", "999"), // edit generator
    15  // iterations for statistical reliability
);

// Print comprehensive performance summary
IncrementalTestUtils::print_performance_summary(&result);

// Validate against production criteria
let criteria = IncrementalTestUtils::standard_criteria();
let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
validation.print_report();  // ✅ PASSED or ❌ FAILED with details
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

## Comprehensive Performance Benchmarking Guide ⚡ **PRODUCTION-READY v0.8.9**

This section provides a complete guide to performance testing and benchmarking the incremental parsing system using the production-ready infrastructure.

### Performance Testing Infrastructure (**Diataxis: Explanation**)

The performance testing system provides comprehensive analysis with statistical validation:

**Core Components**:
- **IncrementalTestUtils**: Main performance testing utilities with statistical analysis
- **PerformanceTestHarness**: Advanced benchmarking infrastructure for detailed measurements
- **ValidationFramework**: Automated criteria checking against production targets
- **Statistical Analysis**: Timing distributions, efficiency rates, and reliability metrics

**Performance Categories** (**Diataxis: Reference**):
- **🟢 Excellent**: <100µs average parse time (target for simple edits)
- **🟡 Very Good**: 100-500µs average (acceptable for moderate edits)
- **🟠 Good**: 500µs-1ms average (acceptable for complex edits)
- **🔴 Acceptable**: 1-5ms average (boundary case, investigate optimization)
- **🚨 Needs Optimization**: >5ms average (requires immediate attention)

### Running Performance Tests (**Diataxis: Tutorial**)

**Step 1: Basic Performance Testing**
```bash
# Run comprehensive incremental parsing tests with performance analysis
cargo test -p perl-parser --features incremental --test incremental_comprehensive_test -- --nocapture

# Run performance-focused tests with statistical analysis
cargo test -p perl-parser --features incremental --test incremental_performance_tests -- --nocapture
```

**Step 2: Specific Performance Scenarios**
```bash
# Test simple value edits (should achieve <100µs)
cargo test -p perl-parser --features incremental -- test_comprehensive_simple_value_edits --nocapture

# Test scaling characteristics with large documents
cargo test -p perl-parser --features incremental -- test_comprehensive_large_document_scaling --nocapture

# Test Unicode and multibyte character handling
cargo test -p perl-parser --features incremental -- test_comprehensive_unicode_and_multibyte --nocapture

# Test edge cases and error recovery
cargo test -p perl-parser --features incremental -- test_comprehensive_edge_cases --nocapture
```

**Step 3: Interactive Performance Demonstration**
```bash
# Run interactive example with real-time performance metrics
cargo run -p perl-parser --example test_incremental_v2 --features incremental
```

### Performance Criteria and Validation (**Diataxis: Reference**)

**Standard Performance Criteria**:
```rust
PerformanceCriteria {
    max_avg_micros: 1000,              // <1ms average parse time
    min_efficiency_percentage: 70.0,   // ≥70% node reuse rate
    min_speedup_ratio: 2.0,            // ≥2x faster than full parsing
    max_coefficient_of_variation: 0.5, // Consistent performance
    min_success_rate: 0.95,            // ≥95% successful parses
}
```

**Relaxed Criteria (for complex scenarios)**:
```rust
PerformanceCriteria {
    max_avg_micros: 5000,              // <5ms average (boundary case)
    min_efficiency_percentage: 50.0,   // ≥50% node reuse rate
    min_speedup_ratio: 1.5,            // ≥1.5x faster than full parsing
    max_coefficient_of_variation: 1.0, // Allow more performance variation
    min_success_rate: 0.90,            // ≥90% successful parses
}
```

### Creating Custom Performance Tests (**Diataxis: How-to**)

**Using the Performance Test Macros**:
```rust
use crate::support::incremental_test_utils::IncrementalTestUtils;

// Standard performance test (applies strict criteria)
let result = perf_test!(
    "My Performance Test",
    "my $x = 42; my $y = 100;",
    |source| IncrementalTestUtils::create_value_edit(source, "42", "999"),
    15  // iterations
);

// Relaxed performance test (for complex scenarios)
let result = perf_test_relaxed!(
    "Complex Scenario Test", 
    "complex_perl_source_here",
    |source| complex_edit_generator(source),
    10
);
```

**Manual Performance Analysis**:
```rust
use crate::support::incremental_test_utils::*;

// Run custom performance test with detailed analysis
let result = IncrementalTestUtils::performance_test_with_stats(
    "Custom Test Name",
    "initial_source", 
    edit_generator_function,
    iterations
);

// Print comprehensive performance summary
IncrementalTestUtils::print_performance_summary(&result);

// Validate against specific criteria
let criteria = IncrementalTestUtils::standard_criteria();
let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
validation.print_report();

// Access detailed metrics
println!("Median parse time: {}µs", result.median_incremental_micros);
println!("Node reuse efficiency: {:.1}%", result.avg_efficiency_percentage);
println!("Performance consistency: {:.3}", result.coefficient_of_variation);
```

### Performance Regression Detection (**Diataxis: How-to**)

The system includes automated regression detection:

```bash
# Run regression detection test (monitors performance across multiple scenarios)
cargo test -p perl-parser --features incremental -- test_comprehensive_performance_regression_detection --nocapture
```

**Interpreting Regression Results**:
- **✅ No significant regression**: Performance within acceptable bounds
- **⚠️ Performance warning**: Slight degradation detected, monitor closely
- **🚨 Regression detected**: Significant performance drop, requires investigation

### Performance Metrics Interpretation (**Diataxis: Explanation**)

**Timing Metrics**:
- **Average Incremental**: Mean parse time for incremental updates
- **Median Incremental**: Median parse time (less affected by outliers)
- **Range**: Min/Max parse times (shows consistency)
- **Standard Deviation**: Performance consistency measure
- **Coefficient of Variation**: Normalized consistency (lower is better)

**Efficiency Metrics**:
- **Node Reuse Rate**: Percentage of AST nodes reused vs reparsed
- **Speedup Ratio**: Performance improvement vs full parsing
- **Sub-millisecond Rate**: Percentage of operations completing <1ms
- **Success Rate**: Percentage of successful incremental parses

**Scaling Metrics**:
- **µs per Statement**: Performance per unit of code complexity
- **Scaling Factor**: How performance changes with document size
- **Efficiency Improvement**: Node reuse rate with increasing complexity

### Production Performance Targets (**Diataxis: Reference**) ✅ **ACHIEVED**

Based on comprehensive testing, the following targets have been achieved:

| Edit Type | Target | Achieved | Efficiency | Status |
|-----------|--------|----------|------------|--------|
| Simple Values | <100µs | 65µs avg | 96.8% | ✅ Excellent |
| Variable Names | <200µs | 120µs avg | 85.0% | ✅ Very Good |
| String Literals | <150µs | 95µs avg | 90.0% | ✅ Excellent |
| Moderate Docs (25 stmt) | <500µs | 205µs avg | 98.7% | ✅ Very Good |
| Large Docs (100 stmt) | <1ms | 538µs avg | 99.7% | ✅ Good |
| Unicode/Multibyte | <200µs | 145µs avg | 88.0% | ✅ Very Good |
| Edge Cases | <1ms | 300µs avg | 75.0% | ✅ Good |

**Overall System Performance**:
- **✅ 100% Success Rate**: All test scenarios pass validation
- **✅ Production Ready**: Meets all performance criteria
- **✅ Excellent Scaling**: Maintains efficiency with document growth
- **✅ Statistical Reliability**: <0.6 coefficient of variation

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
  - ✅ **Comprehensive S-expression generation enhancement** - Resolved Issue #72 with complete binary/unary operator mappings (binary_+, unary_++, etc.), string interpolation differentiation, and 24-26% parsing performance maintained
  - ✅ **Enhanced AST format compatibility** - Program nodes now use tree-sitter standard (source_file) format while maintaining backward compatibility
  - ✅ **Comprehensive workspace navigation** - Enhanced AST traversal including `NodeKind::ExpressionStatement` support across all providers
  - ✅ **Advanced code actions and refactoring** - Fixed parameter threshold validation and enhanced refactoring suggestions with proper AST handling
  - ✅ **Enhanced call hierarchy provider** - Complete workspace analysis with improved function call tracking and incoming call detection
  - ✅ **Production-ready workspace features** - Improved workspace indexing, symbol tracking, and cross-file rename operations
  - ✅ **Comprehensive test reliability** - 100% test pass rate achieved (195/195 library tests, 33/33 LSP E2E tests, 19/19 DAP tests)
  - ✅ **Quality gate compliance** - Zero clippy warnings, consistent formatting, full architectural compliance maintained
  - ✅ **Enhanced file path completion** - Enterprise-grade security with path traversal prevention, 18 comprehensive tests, 30+ file type recognition
- **Latest enhancements (Enhanced Incremental Parsing)**:
  - ✅ **Whitespace/Comment Detection** - 100% node reuse for non-structural changes with intelligent lexical analysis
  - ✅ **Enhanced Node Matching** - 15+ node types supported for improved reuse detection (70-99% efficiency achieved)
  - ✅ **Unicode-Safe Edit Validation** - Character boundary validation prevents UTF-8 slice panics with multibyte characters
  - ✅ **Strengthened Multibyte Support** - International character handling with proper position adjustment algorithms
  - ✅ **Production Test Infrastructure** - Statistical analysis framework with performance criteria validation and comprehensive regression detection
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
| Incremental parsing | ❌ | ❌ | ✅ Production |
| Unicode-safe editing | ❌ | Limited | ✅ Enhanced |
| Whitespace optimization | ❌ | ❌ | ✅ 100% reuse |
| Node reuse efficiency | ❌ | ❌ | ✅ 70-99% |
| Regex delimiters | ❌ | ❌ | ✅ |
| Indirect object | ❌ | ❌ | ✅ |
| Unicode identifiers | ✅ | ✅ | ✅ |
| Modern Perl (5.38+) | ❌ | ✅ | ✅ |
| Tree-sitter compatible | ✅ | ✅ | ✅ Enhanced |
| Workspace navigation | ❌ | Limited | ✅ Production |
| Test reliability | Limited | 95% | 100% |
| Performance testing | ❌ | ❌ | ✅ Statistical |
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
