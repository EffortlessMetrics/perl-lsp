# perl-module-reference

Find module references under a cursor in Perl source.

This crate is the cursor-aware layer above token parsing. It identifies the
module name under the cursor inside `use`/`require` statements and can also
look inside `use parent`/`use base` argument lists.

## Pipeline

- `perl-module-token-parser` finds the token span.
- `perl-module-reference` decides whether the cursor is on a `use`, `require`,
  `parent`, or `base` reference.
- `perl-module-rename` uses this information to plan edits.

## Key API

- `ModuleReferenceKind`
- `ModuleReference`
- `find_module_reference`
- `find_module_reference_extended`
- `extract_module_reference`
- `extract_module_reference_extended`

## Example

```rust
use perl_module_reference::extract_module_reference;

let reference = extract_module_reference("use Foo::Bar;", 9);
assert_eq!(reference, Some("Foo::Bar".to_string()));
```
