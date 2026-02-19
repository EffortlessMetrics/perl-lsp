# perl-parser-pest

Legacy Pest-based Perl parser (v2) for the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

A pure Rust Perl parser built on the [Pest](https://pest.rs/) PEG parser generator. Parses Perl 5 source into a typed `AstNode` enum and can emit tree-sitter-compatible S-expressions. Maintained as a learning tool, compatibility reference, and benchmark baseline -- not for production use (see `perl-parser` v3 instead).

## Public API

| Type | Description |
|------|-------------|
| `PureRustPerlParser` | Main parser: `parse()` returns `AstNode`, `to_sexp()` formats output |
| `PerlParser` | Pest-derived grammar entry point (exposes `Rule` enum) |
| `AstNode` | Typed AST covering declarations, control flow, expressions, literals, and modern Perl features |
| `PrattParser` | Operator-precedence parser for Perl's expression grammar |
| `SexpFormatter` | Configurable S-expression output (positions, compact mode) |
| `ParseError` / `ParseResult` | Error types with position-aware diagnostics |

## Usage

```rust
use perl_parser_pest::PureRustPerlParser;

let mut parser = PureRustPerlParser::new();
let ast = parser.parse("my $x = 42;").expect("parse failed");
println!("{}", parser.to_sexp(&ast));
```

## License

MIT OR Apache-2.0
