# perl-module-token-parser

Cursor-aware module token parsing for import and reference workflows.

This crate is the bridge between raw text and token spans. It identifies the
module token under a cursor so the higher-level import, reference, and rename
crates can stay boundary-aware without reimplementing token scanning.

## Pipeline

- `perl-module-token-core` scans spans.
- `perl-module-token-parser` exposes the cursor-facing parser.
- `perl-module-reference` and `perl-module-rename` use the span to find or
  rewrite module names.

## Key API

- `ModuleTokenSpan`
- `parse_module_token`

## Example

```rust
use perl_module_token_parser::{ModuleTokenSpan, parse_module_token};

assert_eq!(parse_module_token("use Foo::Bar;", 4), Some(ModuleTokenSpan { start: 4, end: 12 }));
```
