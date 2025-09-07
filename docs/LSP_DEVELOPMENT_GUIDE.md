# LSP Development Guide

## Source Threading Architecture (v0.8.7+)

All LSP providers now support source-aware analysis for enhanced documentation extraction:

### Provider Constructor Patterns
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

### Comment Documentation Extraction

The system provides comprehensive comment documentation extraction with the following features:

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

#### Comment Documentation Examples
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

## Adding New LSP Features

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

## Testing Procedures (*Diataxis: How-to Guide* - Testing procedures)

### Dual-Scanner Corpus Validation (v0.8.9+)

For comprehensive LSP development testing, use dual-scanner corpus comparison to validate parser behavior:

```bash
# Prerequisites: Install system dependencies
sudo apt-get install libclang-dev  # Ubuntu/Debian
brew install llvm                  # macOS

# Navigate to xtask directory (excluded from main workspace)
cd xtask

# Run dual-scanner corpus comparison
cargo run corpus                   # Compare both C and Rust scanners
cargo run corpus -- --scanner both --diagnose  # Detailed analysis

# Individual scanner validation  
cargo run corpus -- --scanner c     # C scanner only (baseline)
cargo run corpus -- --scanner rust  # Rust scanner only
cargo run corpus -- --scanner v3    # V3 native parser

# Diagnostic analysis for parser differences
cargo run corpus -- --diagnose      # Analyze first failing test
cargo run corpus -- --test          # Test simple expressions
```

### Understanding Scanner Mismatch Reports (*Diataxis: Reference* - Output interpretation)

When scanner outputs differ, the system provides detailed analysis:
```
🔀 Scanner mismatches:
   expressions.txt: binary_expression_precedence

🔍 STRUCTURAL ANALYSIS:
C scanner nodes: 15
Rust scanner nodes: 14
❌ Nodes missing in Rust output:
  - precedence_node
➕ Extra nodes in Rust output:  
  - simplified_expression
```

Use this information to:
1. **Identify parsing differences** between C and Rust implementations
2. **Validate LSP behavior** across different parser backends  
3. **Track parser development** and feature parity
4. **Debug structural inconsistencies** in AST generation

## Code Actions and Refactoring

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

## Testing LSP Features

### Test Infrastructure (v0.8.6+)
The project includes a robust test infrastructure with async LSP harness, performance optimizations, and production-grade assertion helpers:

**Async LSP Harness** (`tests/support/lsp_harness.rs`):
- **Thread-safe Communication**: Uses mpsc channels for non-blocking server communication
- **Timeout Support**: Configurable timeouts for all LSP operations (default: 2s)
- **Real JSON-RPC Protocol**: Tests actual protocol compliance, not mocked responses  
- **Background Processing**: Server runs in separate thread preventing test blocking
- **Notification Handling**: Separate buffer for server notifications and diagnostics

### Performance Testing Configuration (v0.8.9+) (**Diataxis: How-to Guide** - Performance testing)

The test infrastructure now includes comprehensive performance optimizations that achieve 99.5% timeout reduction:

#### LSP_TEST_FALLBACKS Environment Variable (**NEW**)

**Purpose**: Enable fast testing mode for CI and development environments

**Configuration**:
```bash
# Enable fast testing mode (reduces test timeouts by ~75%)
export LSP_TEST_FALLBACKS=1

# Performance characteristics:
# - Request timeout: 500ms (vs 2000ms)
# - Wait for idle: 50ms (vs 2000ms)
# - Symbol polling: single 200ms attempt (vs progressive backoff)
# - Result: test_completion_detail_formatting: 60s+ → 0.26s (99.5% improvement)
```

**Usage Examples**:
```bash
# Run all LSP tests in fast mode
LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp

# Run specific performance-sensitive tests
LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp test_completion_detail_formatting
LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp test_workspace_symbol_search

# Validate workspace builds quickly
LSP_TEST_FALLBACKS=1 cargo check --workspace
```

#### Timeout Configuration Modes (**Diataxis: Reference**)

**Production Mode** (default - comprehensive testing):
```rust
// Default timeouts for thorough testing
let timeout = Duration::from_secs(2);           // Request timeout
let idle_wait = Duration::from_secs(2);         // Wait for idle
let symbol_budget = Duration::from_secs(10);    // Symbol polling
```

**Fast Mode** (LSP_TEST_FALLBACKS=1 - optimized for speed):
```rust
// Optimized timeouts for CI/development
let timeout = Duration::from_millis(500);       // 75% reduction
let idle_wait = Duration::from_millis(50);      // 97.5% reduction  
let symbol_check = Duration::from_millis(200);  // Single attempt
```

#### Performance Validation Results

**Before Optimization**:
- `test_completion_detail_formatting`: >60 seconds (often timeout)
- Workspace symbol tests: Often exceed CI limits
- Test suite runtime: 5-10 minutes

**After Optimization (v0.8.9)**:
- `test_completion_detail_formatting`: 0.26 seconds (99.5% faster)
- All tests pass with `LSP_TEST_FALLBACKS=1`: <10 seconds total
- Test suite runtime: <1 minute in fast mode
- Zero functional regressions: All tests maintain identical behavior

**Assertion Helpers** (`tests/support/mod.rs`):
- **Deep Validation**: All LSP responses are validated for proper structure
- **Meaningful Errors**: Clear error messages for debugging test failures
- **No Tautologies**: All assertions actually validate response content

### Using the Async Test Harness
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

### Test Commands
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

## Enhanced Position Tracking Development (v0.8.7+)

The enhanced position tracking system provides accurate line/column mapping for LSP compliance:

### Using PositionTracker in Parser Context
```rust
use crate::parser_context::ParserContext;

// Create parser with automatic position tracking
let ctx = ParserContext::new(source);

// Access accurate token positions
let token = ctx.current_token().unwrap();
let range = token.range();
println!("Token at line {}, column {}", range.start.line, range.start.column);
```

### Position Tracking API Reference
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

## Error Recovery and Fallback Mechanisms

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