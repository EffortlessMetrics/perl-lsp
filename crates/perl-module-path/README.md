# perl-module-path

Perl module name and path conversion helpers.

## When to use this crate

Use `perl-module-path` when you need to move between Perl module names and
filesystem paths. It is the bridge used by rename, resolution, and workspace
scanning code.

## Quick example

```rust
use perl_module_path::{file_path_to_module_name, module_name_to_path, module_path_to_name};

assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
assert_eq!(file_path_to_module_name("/workspace/lib/Foo/Bar.pm"), "Foo::Bar");
```

## Public API

- `normalize_package_separator`
- `module_name_to_path`
- `module_path_to_name`
- `file_path_to_module_name`

## Workspace role

Shared path-conversion utility used across module rename and resolution crates.

## License

MIT OR Apache-2.0
