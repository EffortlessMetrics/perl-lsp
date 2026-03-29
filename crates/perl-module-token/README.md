# perl-module-token

Boundary-safe Perl module token replacement helpers.

## When to use this crate

Use `perl-module-token` when you need to detect or rewrite standalone module
tokens without matching partial names. It is a small orchestration layer over
the naming and boundary crates.

## Quick example

```rust
use perl_module_token::{contains_module_token, replace_module_token};

assert!(contains_module_token("use Foo::Bar;", "Foo::Bar"));
let (updated, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "New::Path");
assert!(changed);
assert!(updated.contains("New::Path"));
```

## Public API

- `module_variant_pairs`
- `contains_module_token`
- `replace_module_token`

## Workspace role

Token rewrite helper used by module rename and matching workflows.

## License

MIT OR Apache-2.0
