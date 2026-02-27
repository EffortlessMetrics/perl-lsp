# perl-module-token

Boundary-safe Perl module token replacement and variant helpers.

## Scope

- Generate canonical + legacy module-name rename variant pairs
- Delegate standalone token boundary matching to `perl-module-boundary`
- Replace standalone module tokens using boundary-safe ranges

## API

- `module_variant_pairs(old_module, new_module)`
- `contains_module_token(line, module_name)`
- `replace_module_token(line, from, to)`
