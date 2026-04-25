# perl-incremental-parsing

Compatibility shim crate for incremental parsing APIs.

## Ownership model

Incremental parsing implementation now lives in `perl-parser` and is re-exported here.
This keeps a single source of truth for correctness and performance fixes while
preserving existing `perl_incremental_parsing` import paths.

## Migration

For new code, prefer `perl-parser` directly:

```rust
use perl_parser::incremental;
```

Existing imports continue to work:

```rust
use perl_incremental_parsing::incremental;
```

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE) at your option.
