# Perl Parser

A Rust-based parser for Perl 5, designed to parse Perl code and produce tree-sitter compatible S-expressions.

## Overview

This parser is built using:
- **perl-lexer**: A custom lexer for tokenizing Perl code
- **perl-parser**: A recursive descent parser with operator precedence

## Features

### ✅ Implemented (94.5% edge case coverage, up from 82.8%)

- **Variables**: All sigils ($, @, %, &, *), special variables ($_, $!, etc.)
- **Declarations**: my, our, local, state
- **Literals**: numbers, strings (with interpolation detection), arrays, hashes
- **Operators**: arithmetic, comparison, logical, regex match (=~, !~)
- **Control Flow**: if/elsif/else, unless, while, until, for, foreach
- **Functions**: sub declarations, anonymous subs, method calls
- **OOP**: bless, object construction, method calls
- **Packages**: package declarations, use/no statements
- **Regex**: pattern matching with =~ and !~
- **Arrays/Hashes**: element access, dereferencing, method chains
- **Deep dereference chains**: Complex chains like `$hash->{key}->[0]->{sub}`
- **Double quoted string interpolation**: `qq{hello $world}` with variable detection
- **Postfix code dereference**: `$ref->&*` syntax
- **Keywords as identifiers**: Reserved words in method names and expressions
- **Phase Blocks**: BEGIN, END, CHECK, INIT, UNITCHECK
- **Other**: qw() word lists, string interpolation, comments

### 🚧 Not Yet Implemented (7 remaining edge cases)

1. **Labels** - `LABEL: for (@list) { }` - requires proper lookahead
2. **Subroutine attributes** - `sub bar : lvalue { }`
3. **Variable attributes** - `my $x :shared`
4. **Format declarations** - `format STDOUT =`
5. **Default in given/when** - `default { }` blocks
6. **Class declarations** - `class Foo { }` (Perl 5.38+)
7. **Method declarations** - `method bar { }` (Perl 5.38+)

## Usage

```rust
use perl_parser::Parser;

let code = r#"
my $x = 42;
if ($x > 0) {
    print "positive";
}
"#;

let mut parser = Parser::new(code);
match parser.parse() {
    Ok(ast) => {
        println!("AST: {}", ast.to_sexp());
    }
    Err(e) => {
        println!("Parse error: {}", e);
    }
}
```

## Examples

Run the examples to see the parser in action:

```bash
# Test variable parsing
cargo run --example test_variables

# Test control flow
cargo run --example test_control_flow

# Test OOP features
cargo run --example test_bless

# Test regex matching
cargo run --example test_regex

# Test all features
cargo run --example test_comprehensive
```

## Output Format

The parser produces S-expressions compatible with tree-sitter:

```perl
my $x = 42;
```

Produces:
```
(program (my_declaration (variable $ x) (number 42)))
```

## Architecture

The parser uses a two-stage approach:
1. **Lexing**: The perl-lexer crate tokenizes the input
2. **Parsing**: The perl-parser uses recursive descent with precedence climbing

This separation allows for better error recovery and easier maintenance.

## Performance

The parser is designed for correctness over speed, but still achieves good performance:
- Typical parsing speed: ~1-5ms for small to medium files
- Memory efficient: Uses string references where possible

## Contributing

This parser is part of the tree-sitter-perl project. Contributions are welcome!

## License

Same as the parent tree-sitter-perl project.