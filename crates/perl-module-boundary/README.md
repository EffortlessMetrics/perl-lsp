# perl-module-boundary

Standalone boundary checks for Perl module tokens on a single source line.

## When to use this crate

Use `perl-module-boundary` when you need to find whole module names in a line
without matching partial identifiers. It is the low-level guard used by import
matching and rename workflows.

## Quick example

```rust
use perl_module_boundary::{contains_standalone_module_token, find_standalone_module_token_ranges};

assert!(contains_standalone_module_token("use Foo::Bar;", "Foo::Bar"));
assert!(!contains_standalone_module_token("use Foo::Barista;", "Foo::Bar"));
assert_eq!(
    find_standalone_module_token_ranges("use Foo::Bar;", "Foo::Bar").collect::<Vec<_>>().len(),
    1,
);
```

## Public API

- `contains_standalone_module_token`
- `find_standalone_module_token_ranges`
- `ModuleTokenRange`

## Workspace role

Small utility crate used by the module-import and module-rename families.

## License

MIT OR Apache-2.0
