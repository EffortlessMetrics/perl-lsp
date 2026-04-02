# perl-module-token-core

Low-level module token span parsing and boundary predicates.

This crate is the lexical foundation for the module pipeline. It knows how to
scan a token span, recognize canonical and legacy separators, and check whether
the token stands alone on a line.

## Pipeline

- `perl-module-token-core` parses spans.
- `perl-module-boundary` filters standalone matches.
- `perl-module-token` uses the result for replacement.
- `perl-module-import`, `perl-module-reference`, and `perl-module-rename`
  consume it higher up the stack.

## Key API

- `ModuleTokenSpan`
- `parse_module_token`
- `has_standalone_module_token_boundaries`
- `is_module_token_char`
- `is_module_identifier_char`

## Example

```rust
use perl_module_token_core::{ModuleTokenSpan, has_standalone_module_token_boundaries, parse_module_token};

assert_eq!(parse_module_token("use Foo::Bar;", 4), Some(ModuleTokenSpan { start: 4, end: 12 }));
assert!(has_standalone_module_token_boundaries("use Foo::Bar;", 4, 12));
```
