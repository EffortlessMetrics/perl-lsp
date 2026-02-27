# perl-module-reference

Cursor-aware Perl module reference extraction for `use` / `require` statements.

## Scope

- Detect direct module references in `use Module::Name` and `require Module::Name`
- Enforce cursor-aware extraction (return only when cursor is on the module token)
- Normalize legacy separators (`Foo'Bar`) to canonical (`Foo::Bar`)
- Return stable byte ranges for located module references

## API

- `find_module_reference(text, cursor_pos)`
- `extract_module_reference(text, cursor_pos)`
- `ModuleReference`
- `ModuleReferenceKind`
