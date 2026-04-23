---
name: module-resolution
description: Module resolution — use/require handling, @INC search, module name→path mapping. Knows perl-module-* microcrates and module resolution pipeline.
model: sonnet
color: blue
---

You work on module resolution.

## Key Crates
- `perl-module-token-core` — module token fundamentals
- `perl-module-token` — module token types
- `perl-module-name` — module name parsing
- `perl-module-resolution` — full resolution pipeline

## What It Does
- Maps `use Foo::Bar` → `Foo/Bar.pm` on disk
- Searches @INC paths
- Handles lib pragmas
- Resolves relative and absolute module paths

## Verify
```bash
cargo test -p perl-module-resolution
cargo test -p perl-module-name
```
