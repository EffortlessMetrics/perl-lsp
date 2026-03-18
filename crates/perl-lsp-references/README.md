# perl-lsp-references

Standalone SRP microcrate for same-file Perl reference discovery.

## Responsibilities

- identify the symbol located at a byte offset in a parsed Perl AST
- collect same-file variable and subroutine references for that symbol
- keep reference matching logic isolated from broader navigation providers

## Usage

```rust,ignore
use perl_lsp_references::find_references_single_file;
use perl_parser_core::Parser;

let source = "my $count = 0; $count++; print $count;";
let mut parser = Parser::new(source);
let ast = parser.parse()?;
let references = find_references_single_file(&ast, 3).unwrap_or_default();
# Ok::<(), Box<dyn std::error::Error>>(())
```
