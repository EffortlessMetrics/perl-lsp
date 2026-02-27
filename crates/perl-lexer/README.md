# perl-lexer

Context-aware Perl lexer with mode-based tokenization for the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

Handles Perl's inherently context-sensitive grammar by tracking lexer mode
(`ExpectTerm` vs `ExpectOperator`) to disambiguate `/` (division vs regex),
`%` (modulo vs hash sigil), heredocs, quote-like operators, and more. Provides
checkpointing for incremental parsing and budget limits to guard against
pathological input.

## Usage

```rust
use perl_lexer::{PerlLexer, TokenType};

let mut lexer = PerlLexer::new("my $x = 42;");
while let Some(token) = lexer.next_token() {
    if matches!(token.token_type, TokenType::EOF) { break; }
    println!("{:?}: {}", token.token_type, token.text);
}
```

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
