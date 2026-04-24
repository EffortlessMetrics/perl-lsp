# perl-incremental-parsing

Compatibility shim for incremental parsing APIs that are now owned by `perl-parser`.

## Overview

This crate no longer maintains an independent incremental implementation.
It re-exports `perl_parser::incremental` so existing imports continue to work while
keeping one source of truth for correctness and performance fixes.

## Migration

Prefer importing from `perl-parser` directly:

```rust
use perl_parser::incremental::{IncrementalState, Edit, apply_edits};
```

This shim remains available for backward compatibility:

```rust
use perl_incremental_parsing::{IncrementalState, Edit, apply_edits};
```

## Part of the `perl-lsp` Workspace

This crate is a Tier 3 member of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
