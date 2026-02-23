# perl-module-path

Perl module name/path conversion utilities.

## Scope

- Convert module names to Perl module paths: `Foo::Bar` -> `Foo/Bar.pm`
- Convert module paths/keys to module names: `Foo/Bar.pm` -> `Foo::Bar`
- Convert source file paths to module names for rename workflows
- Normalize legacy package separators: `Foo'Bar` -> `Foo::Bar`
- Normalize both `/` and `\\` separators

## API

- `normalize_package_separator(module_name)`
- `module_name_to_path(module_name)`
- `module_path_to_name(module_path)`
- `file_path_to_module_name(file_path)`
