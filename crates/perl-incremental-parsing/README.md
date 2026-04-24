# perl-incremental-parsing

Compatibility shim crate for incremental parsing APIs.

## Source of truth

`perl-parser` is now the single owner of incremental parsing implementation in this
workspace (`perl_parser::incremental`).

This crate intentionally re-exports that API so existing downstream imports continue
to compile while preventing logic from drifting in two places.

## Migration

Prefer importing directly from `perl-parser` in new code:

```rust
use perl_parser::incremental::{IncrementalState, Edit, apply_edits};
```

Legacy imports from `perl-incremental-parsing` still work but are deprecated.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
