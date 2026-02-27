# perl-module-name

Perl module-name separator normalization and canonical/legacy variant helpers.

## Scope

- Normalize legacy package separators: `Foo'Bar` -> `Foo::Bar`
- Project canonical names into legacy separators: `Foo::Bar` -> `Foo'Bar`
- Build stable canonical + legacy rename pairs for module-rename workflows

## API

- `normalize_package_separator(module_name)`
- `legacy_package_separator(module_name)`
- `module_variant_pairs(old_module, new_module)`
