# perl-module-token-rewrite

Boundary-safe Perl module token rewrite helper.

## Scope

- Replace standalone module tokens on a single line
- Reuse `perl-module-boundary` scanning for exact module-token boundaries

## API

- `replace_module_token(line, from, to)`
