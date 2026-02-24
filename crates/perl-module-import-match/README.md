# perl-module-import-match

Import-line module match predicates for deterministic rename workflows.

## Scope

- Determine whether a single source line should be rewritten for a target module rename
- Reuse import-head classification from `perl-module-import`
- Reuse boundary-aware token matching from `perl-module-token`

## API

- `line_references_module_import(line, module_name)`
