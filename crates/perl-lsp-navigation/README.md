# perl-lsp-navigation

Perl navigation provider crate for symbol lookup and cross-file navigation.
It covers workspace symbols, type hierarchy, type definition, single-file
references, and document links.

## Use this crate when

Use `perl-lsp-navigation` if you need the navigation logic itself. Use
`perl-lsp-workspace-symbols` when you only need symbol indexing and search, and
use `perl-lsp-providers` when you want the umbrella re-export surface.

## Key exports

- `WorkspaceSymbolsProvider` / `WorkspaceSymbol` - search indexed symbols with
  ranking and container names
- `TypeHierarchyProvider` / `TypeHierarchyItem` - infer parent and child types
  from Perl inheritance relationships
- `TypeDefinitionProvider` - find the type behind a variable, method call, or
  constructor expression
- `find_references_single_file` - same-file reference discovery
- `compute_links` - document links for `use` and `require`

## Example

```rust,ignore
use perl_lsp_navigation::{WorkspaceSymbolsProvider, find_references_single_file};

let mut provider = WorkspaceSymbolsProvider::new();
provider.index_document("file:///lib/Foo.pm", &ast, source);
let symbols = provider.search("logger", &source_map);
let refs = find_references_single_file(&ast, byte_offset);
```

## Stack role

This crate is the navigation layer used by `perl-lsp` request handlers. It sits
on top of parser output and semantic analysis, and below the editor protocol
surface.
