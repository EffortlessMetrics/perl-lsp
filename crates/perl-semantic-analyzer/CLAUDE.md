# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Name**: `perl-semantic-analyzer`
- **Version**: 0.9.0
- **Tier**: 4 (three-level internal dependencies)
- **Purpose**: Semantic analysis, symbol extraction, type inference, scope analysis, and dead code detection for Perl source code. Provides the core analysis layer consumed by LSP provider crates.

## Commands

```bash
cargo build -p perl-semantic-analyzer        # Build
cargo test -p perl-semantic-analyzer         # Run tests
cargo clippy -p perl-semantic-analyzer       # Lint
cargo doc -p perl-semantic-analyzer --open   # View documentation
```

## Architecture

### Dependencies

| Crate | Role |
|-------|------|
| `perl-parser-core` | AST nodes (`Node`, `NodeKind`), `Parser`, source locations |
| `perl-workspace-index` | Cross-file workspace index, `SymKind`, `SymbolKey` |
| `perl-symbol-types` | Shared `SymbolKind`, `VarKind` enums |
| `regex` | Pattern matching in semantic classification |
| `rustc-hash` | `FxHashMap` for fast scope/parent-map lookups |
| `serde` | Serialization for dead code analysis results |

### Modules (`src/analysis/`)

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `semantic` | `SemanticAnalyzer`, `SemanticToken`, `SemanticTokenType`, `SemanticTokenModifier`, `HoverInfo` | Semantic token classification and hover info for LSP |
| `symbol` | `SymbolExtractor`, `SymbolTable`, `Symbol`, `SymbolReference`, `Scope`, `ScopeKind`, `ScopeId` | Symbol extraction and symbol table construction from AST |
| `scope_analyzer` | `ScopeAnalyzer`, `ScopeIssue`, `IssueKind` | Scope issue detection (unused/undeclared/shadowed variables) |
| `type_inference` | `TypeInferenceEngine`, `TypeEnvironment`, `PerlType`, `ScalarType`, `TypeConstraint` | Type inference with scoped environments |
| `declaration` | `DeclarationProvider`, `LocationLink`, `ParentMap` | Go-to-declaration with parent-map AST traversal |
| `dead_code_detector` | `DeadCodeDetector`, `DeadCode`, `DeadCodeType`, `DeadCodeAnalysis`, `DeadCodeStats` | Workspace-level dead code detection (non-WASM only) |
| `index` | `WorkspaceIndex`, `SymbolDef` | Local cross-file symbol index (non-WASM only) |

### Re-exports from `lib.rs`

The crate re-exports core types from `perl-parser-core` (`Node`, `NodeKind`, `SourceLocation`, `Parser`, `ast`, `error`, etc.) and `perl-workspace-index::workspace_index`, so downstream crates can use a single import.

## Usage Examples

### Symbol Extraction

```rust
use perl_semantic_analyzer::{Parser, analysis::symbol::SymbolExtractor};

let mut parser = Parser::new("sub hello { my $x = 1; }");
let ast = parser.parse()?;
let table = SymbolExtractor::new().extract(&ast);
assert!(table.symbols.contains_key("hello"));
```

### Semantic Analysis

```rust
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;

let analyzer = SemanticAnalyzer::analyze(&ast);
let tokens = analyzer.semantic_tokens();  // For LSP highlighting
let hover = analyzer.hover_at(offset);    // For LSP hover
```

### Scope Analysis

```rust
use perl_semantic_analyzer::analysis::scope_analyzer::{ScopeAnalyzer, IssueKind};

let analyzer = ScopeAnalyzer::new();
let issues = analyzer.analyze(&ast, source, &pragma_map);
for issue in &issues {
    // IssueKind: UnusedVariable, VariableShadowing, UndeclaredVariable, etc.
}
```

### Type Inference

```rust
use perl_semantic_analyzer::analysis::type_inference::{TypeInferenceEngine, TypeEnvironment};

let mut engine = TypeInferenceEngine::new();
let result = engine.infer(&ast);
```

## Important Notes

- Modules `dead_code_detector` and `index` are gated behind `#[cfg(not(target_arch = "wasm32"))]`.
- Source files are large (40-80KB) reflecting the complexity of Perl semantics; this is intentional.
- `SymbolKind` and `VarKind` are re-exported from `perl-symbol-types` (shared across the workspace).
- The crate bans `unwrap()`, `expect()`, `panic!()`, `todo!()` in non-test code via workspace lints.
- Doctests are disabled (`doctest = false` in Cargo.toml).
