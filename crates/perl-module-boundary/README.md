# perl-module-boundary

Standalone Perl module-token boundary matching for single-line scanners.

## Scope

- Match standalone module tokens without partial-name false positives
- Return byte ranges for each standalone match in a source line
- Preserve canonical (`::`) and legacy (`'`) separator boundary rules

## API

- `contains_standalone_module_token(line, module_name)`
- `find_standalone_module_token_ranges(line, module_name)`
- `ModuleTokenRange`
