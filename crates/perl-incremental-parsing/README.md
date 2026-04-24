# perl-incremental-parsing

Compatibility shim for incremental parsing APIs.

## Overview

`perl-parser` is the source of truth for incremental parsing. This crate re-exports
`perl_parser::incremental` so existing callers can migrate gradually without
behavior changes.

## Migration

Prefer importing directly from `perl-parser`:

```rust
use perl_parser::incremental::{apply_edits, Edit, IncrementalState};
```

The legacy path remains available:

```rust
use perl_incremental_parsing::incremental::{apply_edits, Edit, IncrementalState};
```

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
