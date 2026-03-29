# perl-module-import-match

Deterministic predicates for deciding whether an import line references a module.

## When to use this crate

Use `perl-module-import-match` when you need a yes/no decision for whether a
single source line should be rewritten during a module rename. It combines
import-head parsing with boundary-safe token matching so rename workflows avoid
partial false positives.

## Quick example

```rust
use perl_module_import_match::line_references_module_import;

assert!(line_references_module_import("use Foo::Bar;", "Foo::Bar"));
assert!(!line_references_module_import("use Foo::Barista;", "Foo::Bar"));
```

## Public API

- `line_references_module_import`

## Workspace role

Predicate crate used by module rename and token-rewrite workflows.

## License

MIT OR Apache-2.0
