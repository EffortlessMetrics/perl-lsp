# perl-module-name

Canonical and legacy Perl module-name separator helpers.

## When to use this crate

Use `perl-module-name` when you need to normalize or project Perl package
separators. It is the shared naming layer for module-path, token, and rename
workflows.

## Quick example

```rust
use perl_module_name::{legacy_package_separator, module_variant_pairs, normalize_package_separator};

assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
assert_eq!(module_variant_pairs("Foo::Bar", "New::Path").len(), 2);
```

## Public API

- `normalize_package_separator`
- `legacy_package_separator`
- `module_variant_pairs`

## Workspace role

Shared naming utility used by module resolution and rename crates.

## License

MIT OR Apache-2.0
