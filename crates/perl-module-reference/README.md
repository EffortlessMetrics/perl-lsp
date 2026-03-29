# perl-module-reference

Cursor-aware module-reference extraction for Perl `use` and `require` lines.

## When to use this crate

Use `perl-module-reference` when you need to answer “what module is under the
cursor?” for import lines. It returns the module token, its range, and the
reference kind so navigation and rename code can make the right decision.

## Quick example

```rust
use perl_module_reference::extract_module_reference;

assert_eq!(extract_module_reference("use Foo::Bar;", 4), Some("Foo::Bar".to_string()));
assert_eq!(extract_module_reference("require Foo::Bar;", 8), Some("Foo::Bar".to_string()));
```

## Public API

- `find_module_reference`
- `find_module_reference_extended`
- `extract_module_reference`
- `extract_module_reference_extended`
- `ModuleReference`
- `ModuleReferenceKind`

## Workspace role

Shared lookup crate used by navigation and rename workflows.

## License

MIT OR Apache-2.0
