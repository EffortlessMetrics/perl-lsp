# perl-parser-pest

Legacy Pest-based Perl parser (v2) — maintained as a learning tool and compatibility layer.

## Overview

This crate provides a pure Rust Perl parser built with the [Pest](https://pest.rs/) parser generator.
It outputs tree-sitter compatible S-expressions and requires no C dependencies.

**Note:** This is maintained as a learning tool and historical reference.
For production parsing and LSP, use `perl-parser` (v3).

## Features

- Pure Rust implementation with no C dependencies
- Pest PEG grammar for Perl 5 syntax
- Tree-sitter compatible S-expression output
- Pratt parser for operator precedence

## Usage

```rust
use perl_parser_pest::PureRustPerlParser;

let mut parser = PureRustPerlParser::new();
let code = r#"
    sub hello {
        my $name = shift;
        print "Hello, $name!\n";
    }
"#;

let ast = parser.parse(code).unwrap();
let sexp = parser.to_sexp(&ast);
println!("{}", sexp);
```

## Architecture

The parser uses a three-stage pipeline:
1. **Pest Parsing**: PEG grammar processes input into parse tree
2. **AST Building**: Type-safe AST construction with Pratt parsing for operators
3. **S-Expression Output**: Tree-sitter compatible format generation

## Why v2 (Pest)?

The v2 parser was the first pure-Rust parser attempt. It's useful for:
- Learning about PEG-based parser design
- Comparing grammar-first vs hand-written approaches
- Benchmarking against the v3 native parser

## Known Limitations

- S-expression formatter has incomplete child formatting
- Some complex Perl constructs may not parse correctly
- Performance is slower than v3 due to Pest overhead

## Related Crates

- `perl-parser` (v3): Production parser for LSP
- `perl-lexer`: Context-aware Perl lexer
- `tree-sitter-perl-rs`: Multi-parser comparison harness
