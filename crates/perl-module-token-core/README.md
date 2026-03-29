# perl-module-token-core

Shared low-level parsing and boundary primitives for Perl module tokens.

## When to use this crate

Use `perl-module-token-core` when you need the grammar-level mechanics for
module token parsing or boundary checking. It is the lowest layer in the
module-name family and is intended to be reused by higher-level crates.

## Quick example

```rust
use perl_module_token_core::{has_standalone_module_token_boundaries, parse_module_token};

assert_eq!(
    parse_module_token("use Foo::Bar;", 4),
    Some(perl_module_token_core::ModuleTokenSpan { start: 4, end: 12 }),
);
assert!(has_standalone_module_token_boundaries("use Foo::Bar;", 4, 12));
```

## Public API

- `parse_module_token`
- `ModuleTokenSpan`
- `has_standalone_module_token_boundaries`
- `is_module_identifier_char`
- `is_module_token_char`

## Workspace role

Foundational parser utility used by the module boundary and rename stack.

## License

MIT OR Apache-2.0
