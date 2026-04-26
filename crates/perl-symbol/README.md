# perl-symbol

Unified Perl symbol model for the `perl-lsp` ecosystem: taxonomy, cursor
extraction, search indexing, and AST surface projection — all in a single
published crate.

## Modules

| Module | Purpose |
|--------|---------|
| [`types`](src/types/mod.rs) | Symbol taxonomy: `SymbolKind`, `VarKind`, and LSP protocol mappings |
| [`cursor`](src/cursor/mod.rs) | Cursor-based symbol extraction helpers (token under cursor, UTF-16 column handling) |
| [`index`](src/index/mod.rs) | Trie + inverted-index symbol search (`SymbolIndex`) |
| [`surface`](src/surface/mod.rs) | Projection layer: derives `SymbolDecl` views from the Perl AST |

## Quick start

```rust,ignore
use perl_symbol::{SymbolKind, VarKind};

let var = SymbolKind::Variable(VarKind::Scalar);
assert_eq!(var.sigil(), Some("$"));

// LSP protocol mapping
assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12);
```

## Benchmarks

Run the `perl-symbol` benchmark suite with:

```bash
cargo bench -p perl-symbol
```

The suite currently tracks cursor extraction, UTF-16 token extraction,
symbol-range lookup, index insertion/query performance at 1k symbols, and
surface declaration projection workloads.

## History

This crate consolidates four former microcrates (`perl-symbol-types`,
`perl-symbol-cursor`, `perl-symbol-index`, `perl-symbol-surface`) into a single
published facade as part of ADR-0041 (microcrate collapse). See PR #4428 for
the Wave B collapse that produced this crate.

## License

MIT OR Apache-2.0
