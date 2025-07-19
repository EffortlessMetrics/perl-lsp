# Architecture Guide

This document provides a comprehensive overview of the Production-Ready Pure Rust Perl Parser architecture.

## 🏗️ System Overview

The tree-sitter-perl project is a **Production-Ready Pure Rust Parser** achieving 99.9% Perl 5 syntax coverage:

1. **Pest Parser**: Grammar-driven parsing with zero C dependencies
2. **Tree-sitter Output**: 100% compatible S-expression format for IDE integration
3. **99.9% Coverage**: Handles virtually all real-world Perl code
4. **Performance**: ~180 µs/KB parsing speed with efficient memory usage
5. **Full Unicode Support**: Including identifiers and strings
6. **Comprehensive Testing**: 16+ test files with edge case coverage

## 📐 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                 Pure Rust Perl Parser                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Pest Grammar  │  │   AST Builder   │  │  S-Expression   │  │
│  │ (grammar.pest)  │  │ (PureRustPerl   │  │   Generator     │  │
│  │                 │  │    Parser)      │  │  (to_sexp)      │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
│           │                     │                     │          │
│           ▼                     ▼                     ▼          │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    Parse Pipeline                           │  │
│  │  Input → Tokenize → Parse → Build AST → Output S-exp       │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                 Edge Case System                            │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐      │  │
│  │  │   Heredoc    │  │   Phase     │  │   Dynamic       │      │  │
│  │  │   Handler    │  │   Aware     │  │   Recovery      │      │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘      │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## 🔧 Core Components

### 1. Pest Grammar (`src/grammar.pest`)

**Purpose**: Complete PEG grammar defining Perl 5 syntax

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
- ✅ All Perl 5 syntax (99.9% coverage)
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