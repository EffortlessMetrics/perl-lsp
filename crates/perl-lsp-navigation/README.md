# perl-lsp-navigation

Navigation providers for Perl LSP symbol discovery and linking.

## Scope

- Implements workspace symbols, type hierarchy, and type definition providers.
- Supports reference queries and document-link extraction.
- Builds navigation results backed by workspace index and semantic context.

## Public Surface

- `TypeDefinitionProvider`.
- `TypeHierarchyProvider`, `TypeHierarchyItem`, `TypeHierarchySymbolKind`.
- `WorkspaceSymbolsProvider`, `WorkspaceSymbol`.
- Helpers: `find_references_single_file`, `compute_links`.

## Workspace Role

Internal feature crate consumed by `perl-lsp` navigation request handlers.

## License

MIT OR Apache-2.0.
