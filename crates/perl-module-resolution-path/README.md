# perl-module-resolution-path

Workspace-aware Perl module-name to filesystem-path resolution.

## When to use this crate

Use `perl-module-resolution-path` when you need a filesystem path candidate for
a Perl module name inside a workspace root. It searches include paths under the
root, applies workspace path validation, and falls back to `root/lib`.

## Quick example

```rust
use perl_module_resolution_path::resolve_module_path;
use std::path::Path;

let root = Path::new("/workspace");
let path = resolve_module_path(root, "Foo::Bar", &["lib".to_string()]);
assert!(path.is_some());
```

## Public API

- `resolve_module_path`

## Workspace role

Filesystem-path resolution helper used by higher-level module resolution crates.

## License

MIT OR Apache-2.0
