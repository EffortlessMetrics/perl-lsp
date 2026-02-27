# perl-module-rename

Deterministic module-import line edit planning for file rename workflows.

## Scope

- Detect rename-impacted Perl module import lines (`use`, `require`, `use parent`, `use base`)
- Plan line-based edits for old module to new module transitions
- Support canonical (`Foo::Bar`) and legacy (`Foo'Bar`) package separators
- Delegate import-line match policy to `perl-module-import-match`
- Delegate token variant/boundary logic to `perl-module-token`

## API

- `plan_module_rename_edits(source, old_module, new_module)`
- `apply_module_rename_edits(source, edits)`
- `ModuleLineEdit`
