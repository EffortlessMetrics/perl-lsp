# perl-parser-core

Core parsing engine for the [perl-parser](https://github.com/EffortlessMetrics/perl-lsp) workspace. Provides a recursive descent parser with IDE-friendly error recovery, AST construction, token stream utilities, and position mapping for LSP integration.

## When to use this crate

Use `perl-parser-core` when you want the lower-level parser engine without the
broader re-export surface of `perl-parser`.

It is the right entry point when you need:

- the recursive-descent parser itself
- AST and token-stream types
- IDE-friendly error recovery and position mapping
- a foundation for custom tooling built on parsed Perl syntax

## Public API

- **`Parser`** -- recursive descent parser with `parse()` and `parse_with_recovery()` methods
- **`RecoveryParser`** / **`ParserContext`** -- error-tolerant parsing with budget-controlled recovery
- **`Node`**, **`NodeKind`**, **`SourceLocation`** -- AST types re-exported from `perl-ast`
- **`TokenStream`**, **`Token`**, **`TokenKind`** -- buffered token stream with lookahead
- **`ParseError`**, **`ParseOutput`**, **`ParseResult`** -- error types and result wrappers
- **`PositionMapper`**, **`LineIndex`** -- UTF-8/UTF-16 position conversion for LSP
- **`Trivia`**, **`TriviaPreservingParser`** -- whitespace/comment preservation for formatters

## Usage

```rust
use perl_parser_core::Parser;

let mut parser = Parser::new("my $x = 42; sub hello { print $x; }");
let ast = parser.parse()?;
// Recovered errors available via parser.errors()
```

## Workspace role

Tier 2 crate that aggregates Tier 1 leaf crates (`perl-lexer`, `perl-token`, `perl-ast`, `perl-error`, etc.) into the parsing engine consumed by `perl-parser` and higher-level analysis crates.

## License

MIT OR Apache-2.0
