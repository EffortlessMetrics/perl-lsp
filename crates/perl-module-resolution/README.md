# perl-module-resolution

Module-name resolution for workspace-aware Perl tooling.

Use this crate when you need to turn `Foo::Bar` into a filesystem path or
`file://` URI while respecting workspace roots, open documents, and include
paths.

## Where it fits

This crate sits above the low-level path and URI helpers and below editor
providers. It combines the `perl-module-resolution-path` and
`perl-module-resolution-uri` layers so callers can resolve modules without
re-implementing workspace lookup rules.

## Key entry points

- `resolve_module_path(root, module_name, include_paths)` - filesystem lookup
- `resolve_module_uri(...)` - workspace-aware URI resolution
- `ModuleUriResolution` - result type for URI lookups
- `use_lib` - helpers for `use lib` and `FindBin` include-path discovery

## Typical use

Use `perl-module-resolution` when you already know the module name and need to
find the right file or URI for navigation, completion, or hover features. If
you only need path safety or URI normalization, use the lower-level crates
directly.
