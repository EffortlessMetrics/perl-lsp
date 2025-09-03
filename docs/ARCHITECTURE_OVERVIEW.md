# Architecture Overview

## Crate Structure

### Production Crates
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

### Internal/Unpublished
- **`/tree-sitter-perl/`**: Original C implementation (benchmarking only)
- **`/crates/tree-sitter-perl-rs/`**: Internal test harness
- **`/xtask/`**: Development automation
- **`/docs/`**: Architecture documentation

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

### 3. S-Expression Output
- `to_sexp()` method produces tree-sitter format
- Compatible with existing tree-sitter tools
- Preserves all position information
- Error nodes for unparseable constructs

### 4. Edge Case Handling
- Comprehensive heredoc support (93% edge case test coverage)
- Phase-aware parsing for BEGIN/END blocks
- Dynamic delimiter detection and recovery
- Clear diagnostics for unparseable constructs

### 5. Testing Strategy
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
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions
- **Legacy**: `/crates/perl-parser-pest/` - maintenance only (contains outdated Rope usage)

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