# perl-ts-logos-lexer

Logos-based lexer and recursive descent parsers for Perl source code. Part of the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

This crate provides multiple tokenizer and parser implementations built on the
[Logos](https://crates.io/crates/logos) lexer generator, with context-aware slash
disambiguation (regex vs division) and regex/quote-like operator parsing.

## Key Components

- **`simple_token`** -- Simplified Logos token enum covering Perl keywords, operators, variables, and literals
- **`context_lexer_simple`** / **`context_lexer_v2`** -- Context-aware lexers that disambiguate `/` as regex or division based on preceding tokens
- **`regex_parser`** -- Dedicated parser for regex and quote-like operators (`m//`, `s///`, `qr//`, `tr///`, `q//`, `qq//`, `qw//`, `qx//`)
- **`simple_parser`** / **`simple_parser_v2`** -- Recursive descent parsers producing a simple AST (`token_ast::AstNode`)
- **`logos_lexer`** (feature `logos-tokens`) -- Full-featured Logos token enum with keyword callbacks and context-aware `PerlLexer`
- **`token_parser`** (feature `logos-tokens`) -- Chumsky-based parser consuming `logos_lexer` tokens into `perl-parser-pest` AST nodes

## Usage

```rust
use perl_ts_logos_lexer::simple_token::Token;
use perl_ts_logos_lexer::context_lexer_simple::ContextLexer;

let mut lexer = ContextLexer::new("my $x = 10 / 2;");
while let Some(token) = lexer.next() {
    println!("{:?}", token);
}
```

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT License](http://opensource.org/licenses/MIT), at your option.
