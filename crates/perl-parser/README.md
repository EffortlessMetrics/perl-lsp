# perl-parser

High-level facade for the Perl parsing stack.

Use this crate when you want one entry point for parsing, semantic analysis,
workspace indexing, refactoring, and the LSP provider re-exports that sit on
top of them. If you only need the parser engine, use `perl-parser-core`
directly.

## Where it fits

`perl-parser` is the top of the parsing stack. It re-exports the lower-level
parser, analysis, workspace, and refactoring crates so downstream code can
depend on one crate instead of the whole family.

## Main entry points

- `Parser` plus `ast`, `position`, `error`, and `ParseResult`
- `analysis::*` from `perl-semantic-analyzer`
- `workspace::*` from `perl-workspace-index`
- `refactor::*` from `perl-refactoring`
- `completion`, `diagnostics`, `rename`, and other LSP provider re-exports
- `perl-parse` when the `cli` feature is enabled

## Example

```rust
use perl_parser::Parser;

let mut parser = Parser::new("my $x = 42;");
let ast = parser.parse()?;
assert!(!ast.to_sexp().is_empty());
```

## Typical use

Use `perl-parser` when you are building editor tooling, code transforms, or
tests that need both parsing and the higher-level analysis layers. If you only
need a small slice of the stack, depend on the lower-level crate directly.
