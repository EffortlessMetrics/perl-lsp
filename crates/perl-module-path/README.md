# perl-module-path

Convert Perl module names to filesystem paths and back.

This crate is the string-to-path bridge in the module pipeline. It knows about
`Foo::Bar` and `Foo/Bar.pm`, but it does not search the filesystem or validate
workspace boundaries.

## Pipeline

- `perl-module-name` normalizes module separators.
- `perl-module-path` converts canonical names to path strings and back.
- `perl-module-resolution-path` and `perl-module-resolution-uri` use those
  strings when they search a workspace.

## Key API

- `normalize_package_separator`
- `module_name_to_path`
- `module_path_to_name`
- `file_path_to_module_name`

## Example

```rust
use perl_module_path::{module_name_to_path, module_path_to_name};

assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
```
