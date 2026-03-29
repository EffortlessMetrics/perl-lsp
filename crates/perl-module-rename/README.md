# perl-module-rename

Deterministic import-line rename planning for Perl module refactors.

## When to use this crate

Use `perl-module-rename` when you need to rewrite module references during a
rename workflow. It plans the line edits for import statements, `use parent`,
`use base`, and related module-name prefixes while preserving canonical and
legacy separator forms.

## Quick example

```rust,ignore
use perl_module_rename::{apply_module_rename_edits, plan_module_rename_edits};

let edits = plan_module_rename_edits("use Foo::Bar;\n", "Foo::Bar", "New::Path");
let rewritten = apply_module_rename_edits("use Foo::Bar;\n", &edits);
assert!(rewritten.contains("New::Path"));
```

## Public API

- `plan_module_rename_edits`
- `apply_module_rename_edits`
- `ModuleLineEdit`

## Workspace role

Workspace rename helper used by module-path and import matching crates.

## License

MIT OR Apache-2.0
