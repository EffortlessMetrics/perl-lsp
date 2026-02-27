# perl-parser

Central hub crate for the Perl parser ecosystem. Provides a native recursive-descent
parser (v3) with Tree-sitter-compatible AST output, semantic analysis, workspace
indexing, refactoring, and LSP provider re-exports.

## Usage

```rust
use perl_parser::Parser;

let mut parser = Parser::new("my $x = 42;");
let ast = parser.parse().unwrap();
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

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
