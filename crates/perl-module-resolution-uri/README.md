# perl-module-resolution-uri

Resolve a Perl module name to a `file://` URI.

This crate is the URI-facing leg of module resolution. It keeps the search
order deterministic, respects a timeout budget, and returns a compact outcome
enum instead of throwing resolution policy into caller code.

## Pipeline

- `perl-module-resolution-path` resolves to filesystem paths.
- `perl-module-resolution-uri` resolves to file URIs and tracks `NotFound` vs
  `TimedOut`.
- Higher-level workspace tools can pick the variant that fits their editor or
  transport layer.

## Key API

- `ModuleUriResolution`
- `resolve_module_uri`

## Example

```rust,ignore
use perl_module_resolution_uri::{ModuleUriResolution, resolve_module_uri};
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

match result {
    ModuleUriResolution::Resolved(uri) => assert!(uri.starts_with("file://")),
    ModuleUriResolution::NotFound | ModuleUriResolution::TimedOut => {}
}
```
