# perl-lsp-workspace-symbols

Workspace symbol indexing and search for Perl. This crate answers "what symbols
exist across the workspace, and where are they?"

## Use this crate when

Use `perl-lsp-workspace-symbols` if you need the search/indexing layer itself.
Use `perl-lsp-navigation` when you want workspace symbols together with type
definition, document links, and references.

## Key exports

- `WorkspaceSymbolsProvider` - indexes documents and performs ranked search
- `WorkspaceSymbol` - search result payload with location, kind, and container

## Example

```rust,ignore
use perl_lsp_workspace_symbols::WorkspaceSymbolsProvider;

let mut provider = WorkspaceSymbolsProvider::new();
provider.index_document("file:///lib/Foo.pm", &ast, source);
let symbols = provider.search("logger", &source_map);
```

## Stack role

This crate is the symbol index used by the navigation layer in `perl-lsp`.
It is focused on ranked symbol lookup, not on editor protocol wiring.
