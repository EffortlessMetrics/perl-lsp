# perl-module-token

Boundary-safe Perl module token replacement and variant helpers.

## Scope

- Generate canonical + legacy module-name rename variant pairs
- Detect standalone module tokens on a single source line
- Replace standalone module tokens without partial-name false positives

## API

- `module_variant_pairs(old_module, new_module)`
- `contains_module_token(line, module_name)`
- `replace_module_token(line, from, to)`
