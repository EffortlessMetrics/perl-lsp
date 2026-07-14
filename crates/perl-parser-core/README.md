# perl-parser-core

Core parsing engine for Perl source.

Use this crate when you need the parser, AST, position mapping, or trivia
preservation layers without the higher-level workspace or LSP facades.

## Where it fits

`perl-parser-core` is the low-level parsing layer used by `perl-parser`,
`perl-semantic-analyzer`, `perl-workspace`, and `perl-refactoring`. It
turns source text into AST nodes and exposes the building blocks that higher
layers compose.

## Key entry points

- `Parser` - recursive-descent parser
- `ParseError`, `ParseResult`, `ParseOutput`
- `Node`, `NodeKind`, `SourceLocation`
- `TokenStream`, `Token`, `TokenKind`
- `incremental::IncrementalState` and `incremental::IncrementalMetrics`
- `PositionMapper` and `LineEnding`
- `Trivia`, `TriviaPreservingParser`, `format_with_trivia`

## Example

```rust
use perl_parser_core::Parser;

let mut parser = Parser::new("my $x = 42; sub hello { print $x; }");
let ast = parser.parse()?;
assert!(!ast.to_sexp().is_empty());
```

## Typical use

Use `perl-parser-core` when you are building parser-adjacent tooling or adding
new syntax behavior. If you want the higher-level analysis and refactoring
re-exports in one crate, use `perl-parser`.

The `incremental` module provides checkpoint-bounded token replay with explicit
fallback metrics. It reuses parser tokens outside the edit window and rebuilds the
AST from the assembled stream; it does not claim AST subtree reuse.
