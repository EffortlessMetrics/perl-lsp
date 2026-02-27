# perl-module-token-core

Shared low-level primitives for Perl module token parsing and boundary checks.

## Scope

- Parse module-token spans from byte offsets using canonical (`::`) and legacy (`'`)
  separators.
- Provide boundary checks for standalone token matching behavior.

## API

- `parse_module_token(text, start)`
- `ModuleTokenSpan`
- `has_standalone_module_token_boundaries(line, start, end)`
- `is_module_identifier_char`
- `is_module_token_char`
