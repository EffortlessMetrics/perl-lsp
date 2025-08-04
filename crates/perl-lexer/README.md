# perl-lexer

A high-performance Perl lexer with context-aware tokenization.

## Features

- **Context-aware tokenization**: Correctly handles Perl's complex syntax including regex vs division disambiguation
- **Full Perl 5 support**: All operators, keywords, and constructs
- **Unicode support**: Full Unicode identifier support
- **Zero dependencies**: Core lexer has minimal dependencies
- **Fast**: Optimized for speed with SIMD support

## Usage

```rust
use perl_lexer::{Lexer, Token, TokenType};

let input = "my $x = 42;";
let lexer = Lexer::new(input);

for token in lexer {
    println!("{:?}", token);
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.