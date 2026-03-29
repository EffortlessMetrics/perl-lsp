# perl-module-import

Single-line `use` and `require` parsing for Perl import workflows.

## When to use this crate

Use `perl-module-import` when you need to classify an import line before a
rename, navigation, or refactoring pass. It parses the first token after
`use`/`require`, records the byte range, and distinguishes `use parent` and
`use base`.

## Quick example

```rust
use perl_module_import::{parse_module_import_head, ModuleImportKind};

let head = parse_module_import_head("use parent 'Foo::Bar';").unwrap();
assert_eq!(head.kind, ModuleImportKind::UseParent);
assert_eq!(head.token, "parent");
```

## Public API

- `parse_module_import_head`
- `ModuleImportHead`
- `ModuleImportKind`
- `RequireForm`
- `DispatchSemantics`

## Workspace role

Parsing utility used by the module-reference and module-rename families.

## License

MIT OR Apache-2.0
