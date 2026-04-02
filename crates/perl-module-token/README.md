# perl-module-token

Standalone Perl module token helpers for line-based workflows.

Use this crate when you need a quick yes/no or replacement for a module token
on a single line, but not the full cursor-aware parse.

## Pipeline

- `perl-module-token-core` parses the token span.
- `perl-module-token` enforces standalone boundaries and performs replacement.
- `perl-module-boundary` and `perl-module-rename` build on this behavior for
  higher-level workflows.

## Key API

- `contains_module_token`
- `replace_module_token`

## Example

```rust
use perl_module_token::{contains_module_token, replace_module_token};

assert!(contains_module_token("use Foo::Bar;", "Foo::Bar"));
let (rewritten, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "New::Module");
assert_eq!(rewritten, "use New::Module;");
assert!(changed);
```
