# perl-module-resolution-path

Resolve a Perl module name to a workspace-safe filesystem path candidate.

This crate is the path-first leg of module resolution. It searches include
paths under a workspace root, validates the candidate stays inside that root,
and falls back to `root/lib/<module>.pm` when needed.

## Pipeline

- `perl-module-path` converts names to path strings.
- `perl-module-resolution-path` applies workspace-safe search order.
- `perl-module-resolution-uri` takes the same module name and returns a
  `file://` URI instead of a path.

## Key API

- `resolve_module_path`

## Example

```rust,ignore
use perl_module_resolution_path::resolve_module_path;
use std::path::Path;

let root = Path::new("/workspace");
let path = resolve_module_path(root, "Foo::Bar", &["lib".to_string()]);

assert!(path.is_some());
```
