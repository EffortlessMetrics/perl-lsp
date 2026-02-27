# perl-module-resolution-uri

Perl module URI resolution microcrate.

This crate provides deterministic, timeout-aware module resolution for Perl module
names to `file://` URIs. It owns the search policy used by workspace-aware tools
when resolving imports and dependencies:

- Open document URI precedence
- Workspace folder + include path search with traversal protection
- Optional system `@INC` fallback
- Timeout budget enforcement

## Example

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
