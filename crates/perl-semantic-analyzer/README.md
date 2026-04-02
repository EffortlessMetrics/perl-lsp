# perl-semantic-analyzer

Semantic analysis for Perl source.

Use this crate when you already have an AST and need symbols, scopes, types,
declaration lookup, or dead-code signals before you get to editor features.

## Where it fits

`perl-semantic-analyzer` sits above `perl-parser-core` and next to
`perl-workspace-index`. It turns parsed trees into analysis artifacts that the
workspace, refactoring, and LSP layers can consume.

## Key entry points

- `analysis::symbol::SymbolExtractor`
- `analysis::scope_analyzer::ScopeAnalyzer`
- `analysis::type_inference::TypeInferenceEngine`
- `analysis::semantic::SemanticAnalyzer`
- `analysis::declaration::DeclarationProvider`
- `analysis::index::WorkspaceIndex`

## Example

```rust
use perl_parser_core::Parser;
use perl_semantic_analyzer::analysis::symbol::SymbolExtractor;

let mut parser = Parser::new("sub hello { my $x = 1; }");
let ast = parser.parse()?;
let _symbols = SymbolExtractor::new().extract(&ast);
```

## Typical use

Use `perl-semantic-analyzer` when you are building hover, declaration, type,
or diagnostics features from parsed Perl source. If you need cross-file lookup
or document caching, pair it with `perl-workspace-index`.
