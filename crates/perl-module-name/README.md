# perl-module-name

Normalize Perl module names without touching paths or files.

This crate is the naming primitive in the module pipeline. It handles
separator normalization and rename-friendly variants, but it does not resolve
filesystem locations or parse whole statements.

## Pipeline

- `perl-module-token-core` and `perl-module-boundary` isolate module-shaped
  tokens.
- `perl-module-name` normalizes those tokens to canonical `::` form.
- `perl-module-path`, `perl-module-reference`, and `perl-module-rename`
  consume the normalized names.

## Key API

- `normalize_package_separator`
- `legacy_package_separator`
- `module_variant_pairs`

## Example

```rust
use perl_module_name::{legacy_package_separator, normalize_package_separator};

assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
```
