# perl-semantic-analyzer

Semantic analysis, symbol extraction, and type inference for Perl source code. Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## When to use this crate

Use `perl-semantic-analyzer` when parsing alone is not enough and you need
symbol- or scope-aware understanding of Perl code.

Typical use cases:

- extracting definitions and references from an AST
- computing semantic tokens or scope diagnostics
- inferring types or detecting dead code
- preparing symbol data for navigation, rename, or workspace indexing

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

## Workspace role

`perl-semantic-analyzer` sits between parsing and editor/runtime features. It
is the crate to reach for when you need semantic meaning rather than just
syntactic structure.

## License

Licensed under MIT OR Apache-2.0 at your option.
