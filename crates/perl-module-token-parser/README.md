# perl-module-token-parser

Single-line Perl module-token parsing for import and reference workflows.

## When to use this crate

Use `perl-module-token-parser` when you need to parse a module token from a
cursor offset and feed that span into import, reference, or rename logic.

## Quick example

```rust
use perl_module_token_parser::parse_module_token;

assert_eq!(
    parse_module_token("use Foo::Bar;", 4),
    Some(perl_module_token_parser::ModuleTokenSpan { start: 4, end: 12 }),
);
```

## Public API

- `parse_module_token`
- `ModuleTokenSpan`

## Workspace role

Parsing helper used by import/reference/token rewriting crates.

## License

MIT OR Apache-2.0
