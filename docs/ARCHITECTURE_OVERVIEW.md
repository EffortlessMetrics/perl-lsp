# Architecture Overview

## Crate Structure

### Production Crates
- **`/crates/perl-lsp/`**: Standalone LSP server binary. This is what users install for IDE integration.
- **`/crates/perl-parser/`**: The core parsing library. It contains the parser itself, the AST definitions, and all the LSP feature implementations. Published as `perl-parser` on crates.io.

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

### Internal/Unpublished
- **`/tree-sitter-perl/`**: Original C implementation (benchmarking only)
- **`/crates/tree-sitter-perl-rs/`**: Tree-sitter integration with unified scanner architecture
  - Delegation pattern: C scanner wrapper delegates to Rust implementation
  - Single source of truth for all scanner functionality
  - Maintains backward compatibility while providing modern Rust performance
- **`/xtask/`**: Development automation
- **`/docs/`**: Architecture documentation

## Workspace Configuration Strategy (v0.8.9+)

### Exclusion Architecture (**Diataxis: Explanation** - Design decisions)

The workspace uses a **production-focused exclusion strategy** to ensure reliable builds:

#### Excluded Crates
- **`tree-sitter-perl-c`**: Requires libclang and system dependencies
- **Example crates with feature conflicts**: Avoid cross-crate feature dependency issues
- **Legacy tooling**: Internal development tools not part of published API

#### Architectural Benefits
1. **Platform Independence**: No C toolchain requirements
2. **CI Stability**: Consistent build behavior across platforms
3. **Production Focus**: Testing only published crate surface area
4. **Dependency Safety**: Avoid system-specific build failures

This approach prioritizes **published crate reliability** over comprehensive internal tooling, ensuring users can depend on stable builds regardless of their platform or system configuration.

See [WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md) for current workspace status.

## Key Components

### 1. Pest Parser Architecture
- PEG grammar in `grammar.pest` defines all Perl syntax
- Recursive descent parsing with packrat optimization
- Zero-copy parsing with `&str` slices
- Feature flag: `pure-rust` enables the Pest parser

### 2. AST Generation
- Strongly typed AST nodes in `pure_rust_parser.rs`
- Arc<str> for efficient string storage
- Tree-sitter compatible node types
- Position tracking for all nodes

### 3. Production-Ready Incremental Parsing (**Diataxis: Explanation**)
- **IncrementalParserV2**: Advanced incremental parser with intelligent node reuse
- **Statistical Validation**: Comprehensive performance analysis framework
  - Performance metrics: 65µs average (Excellent), 205µs moderate (Very Good), 538µs large (Good)
  - Node reuse efficiency: 99.7% peak, 96.8% average (target: ≥70%)
  - Statistical consistency: <0.6 coefficient of variation (target: <1.0)
  - Success rate: 100% with comprehensive fallback mechanisms
- **Unicode-Safe Operations**: Proper multibyte character handling with UTF-8 boundary validation
- **Memory Efficiency**: Arc<Node> sharing, LRU cache eviction, Rope-based document management
- **Test Infrastructure**: 40+ comprehensive test cases with production-grade validation
- **LSP Integration**: Real-time document updates with Rope-based position tracking

### 4. S-Expression Output
- `to_sexp()` method produces tree-sitter format
- Compatible with existing tree-sitter tools
- Preserves all position information
- Error nodes for unparseable constructs

### 5. Edge Case Handling
- Comprehensive heredoc support (93% edge case test coverage)
- Phase-aware parsing for BEGIN/END blocks
- Dynamic delimiter detection and recovery
- Clear diagnostics for unparseable constructs

### 6. Testing Strategy
- Grammar tests for each Perl construct
- Edge case tests with property testing
- **Incremental Parsing Tests**: 40+ comprehensive test cases with statistical validation
- **Performance Benchmarks**: Sub-millisecond performance validation with regression detection
- Integration tests for S-expression output
- Position tracking validation tests
- Encoding-aware lexing for mid-file encoding changes
- Tree-sitter compatible error nodes and diagnostics
- Performance optimized (<5% overhead for normal code, 65µs incremental updates)

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready with Rope support
2. **For IDE Integration**: Install `perl-lsp` from `perl-parser` crate - includes full Rope-based document management  
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **LSP Binary & CLI**: `/crates/perl-lsp/` - for changes to the command-line interface or server startup.
- **LSP Feature Logic**: `/crates/perl-parser/` - for all core LSP features (diagnostics, completion, etc.). This is where most LSP development happens.
- **Parser Core**: `/crates/perl-parser/` - for changes to the parsing engine itself.
- **Lexer**: `/crates/perl-lexer/` - for tokenization improvements.
- **Test Corpus**: `/crates/perl-corpus/` - for adding new test cases.
- **Legacy**: `/crates/perl-parser-pest/` - maintenance only.

### Rope Development Guidelines
**IMPORTANT**: All Rope improvements should target the **production perl-parser crate**, not internal test harnesses.

**Production Rope Modules** (Target for improvements):
- **`/crates/perl-parser/src/textdoc.rs`**: Core document management with `ropey::Rope`.
- **`/crates/perl-parser/src/position_mapper.rs`**: UTF-16/UTF-8 position conversion.
- **`/crates/perl-parser/src/incremental_integration.rs`**: LSP integration bridge.
- **`/crates/perl-parser/src/incremental_handler_v2.rs`**: Document change processing.

**Recent Incremental Parsing Improvements**:
- **Enhanced Module Organization**: Fixed import issues in incremental parsing comprehensive tests
- **Improved Code Consistency**: Enhanced formatting and readability across incremental parsing modules
- **Stabilized Integration**: Resolved module import dependencies for better build reliability

**Do NOT modify these Rope usages** (internal test code):
- **`/crates/tree-sitter-perl-rs/`**: Legacy test harnesses with outdated Rope usage
- **Internal test infrastructure**: Focus on production code, not test utilities

## Performance Characteristics

- Pure Rust parser: ~200-450 µs for typical files (2.5KB)
- Memory usage: Arc<str> for zero-copy string storage
- Production ready: Handles real-world Perl code
- Predictable: ~180 µs/KB parsing speed
- Legacy C parser: ~12-68 µs (kept for benchmark reference only)

## Context-Sensitive Features

The parser includes sophisticated solutions for Perl's context-sensitive features:

### Slash Disambiguation
1. **Mode-aware lexer** (`perl_lexer.rs`) - Tracks parser state to disambiguate / as division vs regex
2. **Preprocessing adapter** (`lexer_adapter.rs`) - Transforms ambiguous tokens for PEG parsing
3. **Disambiguated parser** (`disambiguated_parser.rs`) - High-level API with automatic handling

See `SLASH_DISAMBIGUATION.md` for full details.

### Heredoc Support
1. **Multi-phase parser** (`heredoc_parser.rs`) - Three-phase approach to handle stateful heredocs
2. **Full parser** (`full_parser.rs`) - Combines heredoc and slash handling
3. **Complete coverage** - Supports all heredoc variants including indented heredocs

See `HEREDOC_IMPLEMENTATION.md` for full details.

### Edge Case Handling
1. **Edge case handler** (`edge_case_handler.rs`) - Unified detection and recovery system
2. **Phase-aware parsing** (`phase_aware_parser.rs`) - Handles BEGIN/CHECK/INIT/END blocks
3. **Dynamic recovery** (`dynamic_delimiter_recovery.rs`) - Multiple strategies for runtime delimiters
4. **Tree-sitter adapter** (`tree_sitter_adapter.rs`) - Ensures 100% AST compatibility

See `docs/EDGE_CASES.md` for comprehensive documentation.

## Thread-Safety Architecture (**Diataxis: Explanation**)

### Thread-Safety Design Principles

The tree-sitter-perl architecture implements comprehensive thread-safety through immutable data structures and local state management patterns. This design enables high-performance concurrent operations while eliminating race conditions.

#### Core Thread-Safety Patterns

1. **Immutable Provider Pattern** (**Diataxis: Reference**)
   ```rust
   // Thread-safe provider with immutable data
   pub struct SemanticTokensProvider {
       source: String,  // Immutable after construction
       // No mutable shared state
   }
   
   impl SemanticTokensProvider {
       // Safe for concurrent access (&self, not &mut self)
       pub fn extract(&self, ast: &Node) -> Vec<SemanticToken> {
           let mut collector = TokenCollector::new(&self.source);
           collector.collect(ast)  // Local state only
       }
   }
   ```

2. **Local State Collector Pattern** (**Diataxis: Reference**)
   ```rust
   // Each operation creates fresh local state
   struct TokenCollector<'a> {
       source: &'a str,                               // Immutable reference
       declared_vars: HashMap<String, Vec<(u32, u32)>>, // Local state per call
   }
   
   impl<'a> TokenCollector<'a> {
       fn new(source: &'a str) -> Self {
           Self { 
               source, 
               declared_vars: HashMap::new() // Fresh state each time
           }
       }
   }
   ```

3. **Arc-Based Node Sharing** (**Diataxis: Reference**)
   ```rust
   // AST nodes use Arc for safe concurrent access
   pub struct Node {
       pub kind: Arc<NodeKind>,     // Immutable shared content
       pub span: Span,              // Value type - no sharing issues
       pub children: Vec<Arc<Node>>, // Safe to share between threads
   }
   ```

#### Performance Impact of Thread-Safety

**Semantic Tokens Performance** (v0.8.9):
- **Average execution time**: 2.826µs 
- **Performance improvement**: 35x better than 100µs target
- **Memory efficiency**: Zero persistent state between calls
- **Concurrency**: Unlimited concurrent calls with consistent results

**Memory Architecture**:
- **Zero-copy source references**: `&str` slices avoid string duplication
- **Local state isolation**: Each operation creates independent working state
- **Efficient cleanup**: Local state automatically dropped after operation
- **No locks required**: Immutable data eliminates need for synchronization

#### Thread-Safety Validation (**Diataxis: How-to**)

The architecture includes comprehensive thread-safety testing:

```rust
#[test]
fn test_concurrent_semantic_token_access() {
    let provider = SemanticTokensProvider::new(source.to_string());
    let ast = parse_code(source);
    
    // Test concurrent calls produce identical results
    let (tokens1, tokens2, tokens3) = rayon::join(
        || provider.extract(&ast),
        || provider.extract(&ast), 
        || provider.extract(&ast)
    );
    
    // Verify consistency across all concurrent calls
    assert_eq!(tokens1, tokens2);
    assert_eq!(tokens2, tokens3);
}
```

#### Integration with LSP Server (**Diataxis: How-to**)

The thread-safe design enables high-performance LSP operations:

```rust
// LSP server can safely handle concurrent requests
fn handle_semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Response> {
    let doc = self.get_document(&params.uri)?;
    
    // Thread-safe provider creation - no shared mutable state
    let provider = SemanticTokensProvider::new(doc.content.clone());
    
    // Safe concurrent access to AST and provider
    let tokens = provider.extract(&doc.ast);
    
    Ok(encode_semantic_tokens(&tokens))
}
```

#### Benefits of Thread-Safe Architecture (**Diataxis: Explanation**)

1. **Eliminated Race Conditions**: No shared mutable state prevents data races
2. **Exceptional Performance**: Local state management avoids synchronization overhead  
3. **Memory Safety**: Immutable references prevent use-after-free scenarios
4. **Scalability**: Unlimited concurrent operations without contention
5. **Consistency**: Identical results guaranteed for same inputs across threads
6. **Maintainability**: Clear ownership and lifetime semantics reduce complexity

#### Future Thread-Safety Extensions (**Diataxis: Reference**)

The thread-safe patterns established for semantic tokens provide a template for future LSP features:

- **Completion Provider**: Apply immutable provider + local collector pattern
- **Hover Provider**: Use same thread-safe AST traversal approach
- **Definition Provider**: Implement concurrent symbol resolution with local state
- **Reference Provider**: Scale to workspace-wide concurrent symbol searches

This architecture ensures all LSP features can achieve similar performance and safety characteristics as the semantic token provider.