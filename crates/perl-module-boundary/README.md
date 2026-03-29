# perl-module-boundary

Detect standalone Perl module tokens at lexical boundaries.

This crate answers a narrow question: does a line contain a module token that
is truly standalone, or is it part of a larger identifier or path? That makes
it the boundary-checking layer for import, reference, and rename workflows.

## Pipeline

- `perl-module-token-core` parses the token span.
- `perl-module-boundary` filters that span down to standalone matches.
- `perl-module-import`, `perl-module-reference`, and `perl-module-rename` use
  the result to avoid partial matches.

## API

- `ModuleTokenRange`
- `ModuleTokenRangeIter`
- `find_standalone_module_token_ranges`
- `contains_standalone_module_token`

## Example

```rust
use perl_module_boundary::{contains_standalone_module_token, find_standalone_module_token_ranges};

let line = "use Foo::Bar;";
assert!(contains_standalone_module_token(line, "Foo::Bar"));
assert_eq!(find_standalone_module_token_ranges(line, "Foo::Bar").count(), 1);
```
