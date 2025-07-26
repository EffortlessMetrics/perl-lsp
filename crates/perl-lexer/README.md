# perl-lexer

A high-performance, context-aware lexer for Perl 5.

## Features

- 🚀 **Zero-copy tokenization** - Efficient memory usage
- 🧠 **Context-aware** - Correctly handles `/` as division vs regex
- 📝 **Full Perl 5 support** - Heredocs, quotes, interpolation, attributes
- 🔧 **Mode-based lexing** - ExpectTerm vs ExpectOperator disambiguation
- 🎯 **Production ready** - Extensive test coverage

## Installation

```toml
[dependencies]
perl-lexer = "0.4.0"
```

## Usage

```rust
use perl_lexer::{PerlLexer, LexerMode};

let input = "my $x = 42;";
let mut lexer = PerlLexer::new(input);

while let Some(token) = lexer.next_token() {
    println!("{:?}", token);
}
```

## Token Types

The lexer produces a comprehensive set of tokens including:

- **Identifiers** - Variables, subroutines, packages
- **Keywords** - All Perl 5 keywords including modern ones
- **Operators** - All operators with proper precedence hints
- **Literals** - Numbers, strings (with interpolation support)
- **Delimiters** - Parentheses, brackets, braces
- **Special** - Heredocs, POD, formats, attributes

## Context-Aware Features

### Slash Disambiguation

```perl
$x / $y;     # Division operator
$x =~ /foo/; # Regex match
```

### Mode-Based Parsing

The lexer tracks parser state to correctly tokenize:

```rust
lexer.set_mode(LexerMode::ExpectTerm);    // Next token is a term
lexer.set_mode(LexerMode::ExpectOperator); // Next token is an operator
```

## Performance

- Zero allocations for most tokens
- SIMD optimizations available
- ~1-2 µs per token on modern hardware

## License

MIT OR Apache-2.0