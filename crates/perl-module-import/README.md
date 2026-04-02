# perl-module-import

Parse the leading shape of `use` and `require` lines.

This crate turns a single source line into a structured import head with stable
byte offsets and dispatch semantics. Use it when you need to know what the
statement is before you decide how to rewrite or inspect it.

## Pipeline

- `perl-module-token-core` handles token spans.
- `perl-module-import` classifies the leading statement and token.
- `perl-module-import-match` gives you a boolean line check when you do not
  need the full parse.
- `perl-module-reference` and `perl-module-rename` use the parse result to find
  or rewrite imports.

## Key API

- `LoadTiming`
- `ImportBehavior`
- `DispatchSemantics`
- `RequireForm`
- `ModuleImportKind`
- `ModuleImportHead`
- `parse_module_import_head`

## Example

```rust
use perl_module_import::{ModuleImportKind, parse_module_import_head};

let head = parse_module_import_head("use parent 'Foo::Bar';");
assert!(matches!(head.as_ref().map(|h| h.kind), Some(ModuleImportKind::UseParent)));
assert_eq!(head.as_ref().map(|h| h.token), Some("parent"));
```
