# perl-module-import

Single-line Perl import head parsing and classification.

## Scope

- Parse leading `use` and `require` statements
- Extract the first import token and its byte range
- Classify `use parent` / `use base` as distinct import kinds

## API

- `parse_module_import_head(line)`
- `ModuleImportHead`
- `ModuleImportKind`
