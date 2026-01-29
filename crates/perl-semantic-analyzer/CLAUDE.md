# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-semantic-analyzer` is a **Tier 3 core analysis crate** providing semantic analysis and symbol extraction for Perl.

**Purpose**: Semantic analysis and symbol extraction for Perl — performs scope analysis, type inference, and dead code detection.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-semantic-analyzer        # Build this crate
cargo test -p perl-semantic-analyzer         # Run tests
cargo clippy -p perl-semantic-analyzer       # Lint
cargo doc -p perl-semantic-analyzer --open   # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST and parsing
- `perl-workspace-index` - Cross-file references
- `perl-symbol-types` - Symbol taxonomy

### Main Modules

| File | Size | Purpose |
|------|------|---------|
| `analysis/semantic.rs` | 77KB | Core semantic analysis |
| `analysis/scope_analyzer.rs` | 50KB | Scope and variable analysis |
| `analysis/declaration.rs` | 43KB | Symbol declaration tracking |
| `analysis/symbol.rs` | 34KB | Symbol table management |
| `analysis/type_inference.rs` | 41KB | Type inference logic |
| `analysis/dead_code_detector.rs` | - | Dead code analysis |
| `analysis/index.rs` | - | Index structure |

### Analysis Passes

1. **Scope Analysis** — Build lexical scope tree
2. **Declaration Tracking** — Record variable/subroutine declarations
3. **Reference Resolution** — Link references to declarations
4. **Type Inference** — Infer variable types where possible
5. **Dead Code Detection** — Find unreachable/unused code

### Key Types

| Type | Purpose |
|------|---------|
| `SemanticAnalyzer` | Main analysis entry point |
| `Scope` | Lexical scope representation |
| `Symbol` | Declared symbol with metadata |
| `Reference` | Reference to a symbol |
| `TypeInfo` | Inferred type information |

## Usage

```rust
use perl_semantic_analyzer::{SemanticAnalyzer, AnalysisResult};
use perl_parser_core::Parser;

let ast = Parser::parse(source)?;
let result = SemanticAnalyzer::analyze(&ast)?;

// Access symbols
for symbol in result.symbols() {
    println!("Symbol: {} at {:?}", symbol.name, symbol.span);
}

// Access references
for reference in result.references() {
    if let Some(decl) = reference.declaration() {
        println!("Reference to {}", decl.name);
    }
}
```

### Scope Analysis

```rust
// Find all variables in scope at a position
let scope = result.scope_at(position);
for var in scope.visible_variables() {
    println!("In scope: {}", var.name);
}
```

## Important Notes

- Large source files (~50-77KB modules) reflect complexity of Perl semantics
- Analysis is incremental where possible
- Cross-file references require `perl-workspace-index`
- Type inference is best-effort (Perl is dynamically typed)
