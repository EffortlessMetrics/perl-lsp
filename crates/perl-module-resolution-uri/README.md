# perl-module-resolution-uri

Deterministic Perl module-to-URI resolution with workspace-safe search order.

## When to use this crate

Use `perl-module-resolution-uri` when you need to resolve a Perl module name
to a `file://` URI with a deterministic precedence order and a timeout budget.
It is the URI-facing sibling of `perl-module-resolution-path`.

## Quick example

```rust
use perl_module_resolution_uri::{resolve_module_uri, ModuleUriResolution};
use std::time::Duration;

let result = resolve_module_uri(
    "Foo::Bar",
    &[],
    &["file:///workspace".to_string()],
    &["lib".to_string()],
    false,
    &[],
    Duration::from_millis(100),
);

assert!(matches!(result, ModuleUriResolution::NotFound | ModuleUriResolution::Resolved(_)));
```

## Public API

- `resolve_module_uri`
- `ModuleUriResolution`

## Workspace role

URI-resolution helper for workspace-aware tools and editor integrations.

## License

MIT OR Apache-2.0
