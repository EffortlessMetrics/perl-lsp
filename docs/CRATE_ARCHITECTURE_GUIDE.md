# Crate Architecture Guide (v0.8.9 GA)

## Published Crates (Workspace Members)

### `/crates/perl-parser/` - Main Parser Library ⭐ **MAIN CRATE**
- **Purpose**: Core recursive descent parser with production-grade features
- **Key Features**:
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

- **Key Files**:
  - `src/parser.rs`: Recursive descent parser
  - `src/ast.rs`: AST definitions and enhanced workspace navigation
  - `src/textdoc.rs`: Core document management with `ropey::Rope`
  - `src/position_mapper.rs`: UTF-16/UTF-8 position conversion
  - `src/incremental_integration.rs`: LSP integration bridge
  - `src/incremental_handler_v2.rs`: Document change processing

### `/crates/perl-lsp/` - Standalone LSP Server ⭐ **LSP BINARY** (v0.8.9)
- **Purpose**: Clean LSP server implementation separated from parser logic
- **Key Features**:
  - Standalone Language Server binary with production-grade CLI
  - Clean separation from parser logic for improved maintainability
  - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors
- **Key Files**:
  - `src/main.rs`: Clean LSP server implementation
  - `bin/perl-lsp.rs`: LSP server binary entry point

### `/crates/perl-lexer/` - Context-Aware Tokenizer
- **Purpose**: Context-aware tokenizer with mode-based lexing
- **Key Features**:
  - Context-aware tokenizer with mode-based lexing
  - Handles slash disambiguation and Unicode identifiers
  - **Unicode Handling**: Robust support for Unicode characters in all contexts
  - **Heredoc Safety**: Proper bounds checking for Unicode + heredoc syntax
- **Key Files**:
  - `src/lib.rs`: Lexer API with Unicode support
  - `src/token.rs`: Token definitions
  - `src/mode.rs`: Lexer modes (ExpectTerm, ExpectOperator)
  - `src/unicode.rs`: Unicode identifier support

### `/crates/perl-corpus/` - Test Corpus
- **Purpose**: Comprehensive test corpus with property-based testing infrastructure
- **Key Files**:
  - `src/lib.rs`: Corpus API
  - `tests/`: Perl test files

### `/crates/perl-parser-pest/` - Legacy Pest Parser ⚠️ **LEGACY**
- **Purpose**: Pest-based parser (v2 implementation), marked as legacy
- **Status**: Published but marked as legacy, use `perl-parser` instead

## Benchmark Framework (v0.8.9) ⭐ **NEW**

### `/crates/tree-sitter-perl-rs/src/bin/benchmark_parsers.rs`
- **Purpose**: Comprehensive Rust benchmark runner
- **Features**:
  - Statistical analysis with confidence intervals
  - JSON output compatible with comparison tools
  - Memory usage tracking and performance categorization
  - Configurable iterations and warmup cycles

### `/tree-sitter-perl/test/benchmark.js`
- **Purpose**: C implementation benchmark harness  
- **Features**:
  - Node.js-based benchmarking for C parser
  - Standardized JSON output format compatible with comparison framework
  - Environment variable configuration support

### `/scripts/generate_comparison.py`
- **Purpose**: Statistical comparison generator
- **Features**:
  - Cross-language performance analysis (C vs Rust)
  - Configurable regression thresholds (5% parse time, 20% memory defaults)
  - Performance gates with statistical significance testing
  - Markdown and JSON report generation with confidence intervals

### `/scripts/setup_benchmark.sh`
- **Purpose**: Automated benchmark environment setup
- **Features**:
  - Dependency installation for Python analysis framework
  - Environment validation and configuration
  - Complete setup automation for cross-language benchmarking

### `/scripts/test_comparison.py`
- **Purpose**: Comprehensive benchmark framework test suite
- **Features**:
  - 12 test cases covering statistical analysis, configuration, and error handling
  - Validates regression detection and performance gate functionality
  - Unit tests for comparison metrics and threshold validation

## Excluded Crates (System Dependencies)

### `/crates/perl-parser-pest/` - Legacy Pest Parser
- **Status**: Published as `perl-parser-pest` on crates.io (marked legacy)
- **Exclusion Reason**: Requires bindgen for C interop

### `/tree-sitter-perl/` - Original C Implementation
- **Exclusion Reason**: libclang dependency

### `/tree-sitter-perl-c/` - C Parser Bindings
- **Exclusion Reason**: libclang-dev dependency

### `/crates/tree-sitter-perl-rs/` - Internal Test Harness
- **Exclusion Reason**: bindgen dependency

### `/xtask/` - Development Automation
- **Exclusion Reason**: Circular dependency with excluded crates

## Key Components

### Pest Parser Architecture
- PEG grammar in `grammar.pest` defines all Perl syntax
- Recursive descent parsing with packrat optimization
- Zero-copy parsing with `&str` slices
- Feature flag: `pure-rust` enables the Pest parser

### AST Generation
- Strongly typed AST nodes in `pure_rust_parser.rs`
- Arc<str> for efficient string storage
- Tree-sitter compatible node types
- Position tracking for all nodes

### S-Expression Output
- `to_sexp()` method produces tree-sitter format
- Compatible with existing tree-sitter tools
- Preserves all position information
- Error nodes for unparseable constructs

### Enhanced Position Tracking (v0.8.7+)
- **O(log n) Position Mapping**: Efficient binary search-based position lookups using LineStartsCache
- **LSP-Compliant UTF-16 Support**: Accurate character counting for multi-byte Unicode characters and emoji
- **Multi-line Token Support**: Proper position tracking for tokens spanning multiple lines (strings, comments, heredocs)
- **Line Ending Agnostic**: Handles CRLF, LF, and CR line endings consistently across platforms
- **Production-Ready Integration**: Seamless integration with parser context and LSP server for real-time editing
- **Comprehensive Testing**: 8 specialized test cases covering Unicode, CRLF, multiline strings, and edge cases

### Edge Case Handling
- Comprehensive heredoc support (93% edge case test coverage)
- Phase-aware parsing for BEGIN/END blocks
- Dynamic delimiter detection and recovery
- Clear diagnostics for unparseable constructs

### Testing Strategy
- Grammar tests for each Perl construct
- Edge case tests with property testing
- Performance benchmarks
- Integration tests for S-expression output
- Position tracking validation tests
- Encoding-aware lexing for mid-file encoding changes
- Tree-sitter compatible error nodes and diagnostics
- Performance optimized (<5% overhead for normal code)

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready with Rope support
2. **For IDE Integration**: Install `perl-lsp` from `perl-parser` crate - includes full Rope-based document management  
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (v0.8.9)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions
- **Legacy (Excluded)**: `/crates/perl-parser-pest/` - maintenance only, excluded from workspace
- **Build Tools (Excluded)**: `/xtask/` - build automation, excluded due to dependencies

### Rope Development Guidelines
**IMPORTANT**: All Rope improvements should target the **production perl-parser crate**, not internal test harnesses.

**Production Rope Modules** (Target for improvements):
- **`/crates/perl-parser/src/textdoc.rs`**: Core document management with `ropey::Rope`
- **`/crates/perl-parser/src/position_mapper.rs`**: UTF-16/UTF-8 position conversion
- **`/crates/perl-parser/src/incremental_integration.rs`**: LSP integration bridge
- **`/crates/perl-parser/src/incremental_handler_v2.rs`**: Document change processing

**Do NOT modify these Rope usages** (internal test code):
- **`/crates/tree-sitter-perl-rs/`**: Legacy test harnesses with outdated Rope usage
- **Internal test infrastructure**: Focus on production code, not test utilities