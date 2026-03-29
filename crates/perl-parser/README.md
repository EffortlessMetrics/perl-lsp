# perl-parser
[![Crates.io](https://img.shields.io/crates/v/perl-parser.svg)](https://crates.io/crates/perl-parser)
[![Documentation](https://docs.rs/perl-parser/badge.svg)](https://docs.rs/perl-parser)

Native Perl parser and parser-hub crate for the `perl-lsp` workspace.

## When to use this crate

Use `perl-parser` when you want one entry point that combines parsing with the
rest of the Perl analysis stack:

- parse Perl source into an AST
- plug into semantic analysis and workspace indexing
- access LSP-facing provider crates from one crate family

If you only need tokenization or the lower-level parser engine, prefer
`perl-lexer` or `perl-parser-core`.

## Usage

```rust
use perl_parser::Parser;

let mut parser = Parser::new("my $x = 42;");
let ast = parser.parse()?;
println!("{}", ast.to_sexp());
```

## Included binary

`perl-parse` (requires the `cli` feature) parses Perl files and prints the AST
in S-expression, JSON, or debug format.

## Key re-exports

| Module | Source crate | Purpose |
|--------|-------------|---------|
| `engine` | `perl-parser-core` | Recursive-descent parser, AST, error recovery |
| `analysis` | `perl-semantic-analyzer` | Scope analysis, type inference, symbol tables |
| `workspace` | `perl-workspace-index` | Cross-file symbol indexing and document store |
| `refactor` | `perl-refactoring` | Import optimizer, modernization, refactoring engine |
| `tdd` | `perl-tdd-support` | Test generation and TDD workflow |
| `completion`, `diagnostics`, `rename`, ... | `perl-lsp-*` | LSP feature providers |

## Workspace role

`perl-parser` is the broadest library entry point in the
[`perl-lsp`](https://github.com/EffortlessMetrics/perl-lsp) workspace. It is a
good choice for downstream tools that want parser access plus room to grow into
semantic or editor features later.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
