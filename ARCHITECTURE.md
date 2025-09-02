# Architecture Guide

This document provides a comprehensive overview of the tree-sitter-perl project architecture, including three parser implementations and a full LSP server.

## 🏗️ System Overview

The tree-sitter-perl project provides **multiple parser implementations** and **IDE integration**:

1. **v1: C-based Parser**: Original tree-sitter implementation (~95% coverage)
2. **v2: Pest Parser**: Pure Rust with PEG grammar (~99.995% coverage)
3. **v3: Native Parser**: Hand-written lexer+parser (~100% coverage) ⭐
4. **LSP Server**: Full Language Server Protocol implementation
5. **Tree-sitter Output**: All parsers produce compatible S-expressions
6. **Performance**: v3 achieves 4-19x speedup over v1 (1-150 µs)

## 📐 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                     tree-sitter-perl Project                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────┐  │
│  │ v1: C Parser │  │ v2: Pest    │  │ v3: Native   │  │  LSP   │  │
│  │   (Legacy)   │  │   Parser     │  │Parser ⭐     │  │ Server │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └───┬────┘  │
│         │                  │                  │              │       │
│         ▼                  ▼                  ▼              ▼       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              Common S-Expression Output Format                │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    v3: Native Parser Detail                   │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐      │  │
│  │  │ perl-lexer  │→ │ perl-parser │→ │ Tree-sitter AST   │      │  │
│  │  │ (Tokenizer) │  │ (RD Parser) │  │ (S-expressions)   │      │  │
│  │  └─────────────┘  └─────────────┘  └───────────────────┘      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    LSP Server Architecture                    │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐      │  │
│  │  │  JSON-RPC   │  │  Document   │  │ Language Services │      │  │
│  │  │  Handler    │  │  Manager    │  │ (Diagnostics,     │      │  │
│  │  │             │  │             │  │  Symbols, etc.)   │      │  │
│  │  └─────────────┘  └─────────────┘  └───────────────────┘      │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## 🔧 Core Components

### 1. v3: Native Parser (Recommended)

#### perl-lexer (`/crates/perl-lexer/`)
**Purpose**: Context-aware tokenization with mode tracking

**Key Features**:
- Mode-based lexing (ExpectTerm, ExpectOperator)
- Handles slash disambiguation (/ as division vs regex)
- Zero dependencies
- Checkpoint/restore for backtracking

#### perl-parser (`/crates/perl-parser/`)
**Purpose**: Recursive descent parser with operator precedence

**Key Features**:
- Consumes tokens from perl-lexer
- Pratt parsing for operators
- 100% edge case coverage
- Tree-sitter compatible AST
- **Enhanced Position Tracking** (v0.8.7+): O(log n) LSP-compliant UTF-16 position mapping

**Position Tracking Architecture** (**Diataxis: Explanation**):
- **PositionTracker**: Production-ready position mapping with LineStartsCache integration
- **ParserContext**: Enhanced token stream processing with accurate position tracking
- **UTF-16 Compliance**: Proper character counting for multi-byte Unicode characters and emoji
- **Multi-line Support**: Accurate position tracking for tokens spanning multiple lines
- **Performance**: Binary search-based position lookups for real-time LSP editing

### 2. v2: Pest Grammar (`/crates/tree-sitter-perl-rs/src/grammar.pest`)

**Purpose**: PEG grammar defining Perl 5 syntax

**Key Features**:
- **Comprehensive Coverage**: All Perl constructs including edge cases
- **Operator Precedence**: Correctly handles 100+ Perl operators
- **Context Sensitivity**: Special handling for `/`, heredocs, etc.
- **Performance**: Optimized rule ordering for common patterns

**Example Rules**:
```pest
program = { SOI ~ statements? ~ EOI }
statement = { 
    simple_assignment     // Fast path
    | sub_declaration
    | if_statement
    | expression_statement
}
```

### 2. AST Builder (`src/pure_rust_parser.rs`)

**Purpose**: Converts Pest parse trees to strongly-typed AST nodes

**Key Components**:
- **AstNode Enum**: Comprehensive node types for all Perl constructs
- **build_node()**: Recursive AST construction from Pest pairs
- **Memory Efficiency**: Uses `Arc<str>` for string storage
- **Position Tracking**: Preserves source locations for all nodes

**AST Node Example**:
```rust
pub enum AstNode {
    Program(Vec<AstNode>),
    SubDeclaration {
        name: Arc<str>,
        prototype: Option<Arc<str>>,
        body: Box<AstNode>,
    },
    // ... 50+ node types
}
```

### 3. S-Expression Generator

**Purpose**: Outputs tree-sitter compatible format

**Features**:
- **Compatibility**: Matches tree-sitter's S-expression format exactly
- **Error Nodes**: Graceful handling of unparseable constructs
- **Position Info**: Includes byte ranges for all nodes
- **Streaming**: Can output large ASTs efficiently

### 4. Edge Case Handling System

**Purpose**: Handles Perl's most complex parsing challenges

**Components**:

#### Heredoc Handler (`heredoc_parser.rs`)
- Multi-phase parsing for stateful heredocs
- Supports all variants (quoted, interpolated, indented)
- 99% coverage of real-world patterns

#### Phase-Aware Parser (`phase_aware_parser.rs`)
- Tracks BEGIN/CHECK/INIT/END blocks
- Handles compile-time vs runtime distinctions
- Preserves execution order semantics

#### Dynamic Recovery (`dynamic_delimiter_recovery.rs`)
- Detects runtime-determined delimiters
- Multiple recovery strategies
- Clear diagnostics for unparseable cases

### 5. LSP Server (`/crates/perl-parser/src/lsp_server.rs`)

**Purpose**: Language Server Protocol implementation for IDE integration

**Architecture**:

```
LSP Client (Editor) ←→ JSON-RPC ←→ LSP Server
                                        ↓
                                 Document Manager
                                        ↓
                                 Parser (v3) → AST
                                        ↓
                                 Language Services
```

**Key Components**:

#### JSON-RPC Handler
- Processes LSP requests/responses
- Manages client-server communication
- Handles lifecycle (initialize, shutdown)

#### Document Manager
- Tracks open documents
- Caches parsed ASTs
- Manages document versions
- **Enhanced Position Tracking** (v0.8.7+): LSP-compliant UTF-16 position mapping with O(log n) performance

#### Language Services
- **DiagnosticsProvider**: Syntax error detection
- **DocumentSymbolProvider**: Outline generation
- **DefinitionProvider**: Go to definition
- **ReferencesProvider**: Find all references
- **SignatureHelpProvider**: Parameter hints
- **SemanticTokensProvider**: Enhanced highlighting

## 🔍 Parser Pipeline

### 1. Tokenization
- Pest handles tokenization via grammar rules
- Zero-copy design using string slices
- Unicode-aware with proper boundaries

### 2. Parsing
- Recursive descent with packrat optimization
- Left recursion eliminated via precedence climbing
- Error recovery at statement boundaries

### 3. AST Construction
- Bottom-up construction from parse tree
- Type-safe node creation
- Position information preserved

### 4. Output Generation
- Tree-sitter S-expression format
- Optional debug output
- Streaming for large files

## 🚀 Performance Optimizations

### Grammar Optimizations
- **Fast Paths**: Common patterns parsed first
- **Atomic Rules**: Prevent backtracking where possible
- **Rule Ordering**: Most likely matches first

### Memory Optimizations
- **Arc<str>**: Shared string storage
- **Zero-Copy**: Parse directly from input
- **Lazy Allocation**: Build only required nodes

### Runtime Optimizations
- **Incremental Parsing**: Future enhancement
- **Parallel Parsing**: Parse independent blocks concurrently
- **Caching**: Reuse common subpatterns

## 🧪 Testing Strategy

### Comprehensive Test Suite (16+ files)
- **Feature Tests**: `comprehensive_feature_tests.rs` - All Perl constructs
- **Heredoc Tests**: `comprehensive_heredoc_tests.rs`, `unicode_heredoc_tests.rs`
- **Edge Cases**: `edge_case_tests.rs` - Complex parsing scenarios
- **Integration**: `integration_tests.rs` - Full pipeline validation
- **Parser Tests**: `pure_rust_parser_tests.rs` - Unit tests
- **Special Context**: Multiple specialized test files

### Coverage Areas
- ✅ Most Perl 5 syntax (~95% coverage)
- ✅ Unicode support (identifiers, strings)
- ✅ Modern Perl features (signatures, try/catch)
- ✅ Statement modifiers and postfix operators
- ✅ Complex interpolation and heredocs

### Performance Validation
- Criterion benchmarks in `/benches/`
- ~180 µs/KB parsing speed
- Memory efficiency validation
- Comparison with C parser baseline

## 🔒 Error Handling

### Parse Errors
- Clear error messages with location
- Recovery at statement boundaries
- Partial AST generation

### Edge Case Errors
- Diagnostic information in separate channel
- Graceful degradation
- Clear indication of limitations

## 🚦 Future Enhancements

### Planned Features
1. **Incremental Parsing**: Update AST for file changes
2. **Query Support**: Tree-sitter query language
3. **LSP Integration**: Language server protocol
4. **WASM Target**: Browser-based parsing

### Architecture Evolution
- Maintain backward compatibility
- Preserve S-expression format
- Enhance performance iteratively

## 📚 References

- [Pest Parser](https://pest.rs/): PEG parser generator
- [Tree-sitter](https://tree-sitter.github.io/): Parsing framework
- [Perl Language Reference](https://perldoc.perl.org/): Official Perl documentation

---

This architecture provides a solid foundation for a modern, maintainable Perl parser that integrates seamlessly with existing tooling while being purely Rust-based.