# perl-lsp-workspace-symbols

Single-responsibility microcrate for Perl LSP `workspace/symbol` indexing and search.

## When to use this crate

Use `perl-lsp-workspace-symbols` when you want a focused implementation of
LSP `workspace/symbol` for Perl without pulling in the full server runtime.

It handles:

- indexing symbols from parsed Perl files
- searching workspace symbols by query
- shaping results into LSP-friendly workspace symbol payloads

## Quick example

```rust,ignore
use perl_lsp_workspace_symbols::WorkspaceSymbolsProvider;

let mut provider = WorkspaceSymbolsProvider::new();
provider.index_document("file:///test.pl", &ast, source);
let results = provider.search("hello", &source_map);
assert!(!results.is_empty());
```

## Public API

- `WorkspaceSymbolsProvider`: index and query entry point
- `WorkspaceSymbol`: workspace-symbol result type

## License

MIT OR Apache-2.0
