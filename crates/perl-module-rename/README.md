# perl-module-rename

Plan module rename edits for Perl source lines.

This crate is the rewrite layer. It takes an old module name and a new one,
then computes line edits for `use`/`require`, `@ISA`, and qualified calls.

## Pipeline

- `perl-module-reference` finds the relevant module reference.
- `perl-module-import-match` and `perl-module-token` identify matching lines.
- `perl-module-rename` produces the actual edit plan and applies it.

## Key API

- `ModuleLineEdit`
- `plan_module_rename_edits`
- `apply_module_rename_edits`
- `line_references_isa_assignment`
- `line_references_qualified_call`
- `replace_module_name_prefix`

## Example

```rust
use perl_module_rename::{apply_module_rename_edits, plan_module_rename_edits};

let source = "use Foo::Bar;\nFoo::Bar::init();\n";
let edits = plan_module_rename_edits(source, "Foo::Bar", "Renamed::Module");
let rewritten = apply_module_rename_edits(source, &edits);

assert_eq!(rewritten, "use Renamed::Module;\nRenamed::Module::init();\n");
```
