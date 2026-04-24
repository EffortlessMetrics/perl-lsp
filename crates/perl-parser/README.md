# perl-parser

Use this crate when you want one dependency for parsing Perl plus the higher
layers that usually come next: semantic analysis, workspace indexing,
refactoring, and the LSP-oriented provider re-exports.

If you only need the parser engine itself, use `perl-parser-core` directly.

## Where it fits

`perl-parser` is the top of the parsing stack. It lets downstream tools depend
on one crate instead of wiring the parser, analysis, workspace, and refactoring
families together by hand.

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
tests that need both parsing and the layers above parsing. If you only need a
small slice of the stack, depend on the narrower crate directly.
