# perl-semantic-analyzer

Semantic analysis, symbol extraction, and type inference for Perl source code. Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

## Features

- **Symbol extraction** -- `SymbolExtractor` builds a `SymbolTable` of definitions, references, and scopes from a parsed AST.
- **Semantic tokens** -- `SemanticAnalyzer` classifies tokens (`SemanticTokenType`, `SemanticTokenModifier`) for LSP syntax highlighting and hover info.
- **Scope analysis** -- `ScopeAnalyzer` detects unused variables, shadowing, undeclared variables, and other scope issues.
- **Type inference** -- `TypeInferenceEngine` infers `PerlType` for variables and expressions with a scoped `TypeEnvironment`.
- **Dead code detection** -- `DeadCodeDetector` identifies unused subroutines, variables, imports, and unreachable code (non-WASM only).
- **Declaration provider** -- `DeclarationProvider` resolves go-to-declaration with `LocationLink` results and parent-map traversal.
- **Workspace index** -- local `WorkspaceIndex` for cross-file symbol lookup by name, URI, or query.

## Dependencies

Builds on `perl-parser-core` (AST/parsing), `perl-workspace-index` (cross-file references), and `perl-symbol-types` (symbol taxonomy).

## Usage

```rust
use perl_semantic_analyzer::{Parser, analysis::symbol::SymbolExtractor};

let mut parser = Parser::new("sub hello { my $x = 1; }");
let ast = parser.parse()?;
let table = SymbolExtractor::new().extract(&ast);
```

## License

Licensed under MIT OR Apache-2.0 at your option.
