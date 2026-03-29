# perl-module-import-match

Boolean module-import matching for single Perl source lines.

Use this crate when you only need a yes/no answer: does this line reference the
module import I care about? It is the fast predicate layer underneath rename
and import workflows.

## Pipeline

- `perl-module-import` parses and classifies the statement.
- `perl-module-import-match` answers the boolean match question.
- `perl-module-rename` uses both when it plans edits.

## API

- `line_references_module_import`

## Example

```rust
use perl_module_import_match::line_references_module_import;

assert!(line_references_module_import("use Foo::Bar;", "Foo::Bar"));
assert!(!line_references_module_import("use Foo::Bar;", "Other::Module"));
```
