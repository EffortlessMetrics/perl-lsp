# perl-lsp-navigation

LSP navigation providers for Perl: workspace symbols, type hierarchy, type definition, find references, and document links.

## When to use this crate

Use `perl-lsp-navigation` when you want the navigation slice of Perl LSP
features without pulling in the full server binary.

It is a good fit for Rust tooling that needs:

- definition/reference style navigation helpers
- workspace symbol search
- type hierarchy or type-definition support
- document-link extraction from Perl imports

## Public API

- **`WorkspaceSymbolsProvider`** / **`WorkspaceSymbol`** -- indexes parsed documents and searches symbols with fuzzy, prefix, and exact matching.
- **`TypeHierarchyProvider`** / **`TypeHierarchyItem`** / **`TypeHierarchySymbolKind`** -- compatibility re-exports from `perl-lsp-type-hierarchy`; prepares type hierarchy items and resolves super/subtypes via `@ISA`, `use parent`, and `use base`.
- **`TypeDefinitionProvider`** -- go-to-type-definition for variables, method calls, constructors, and `bless` expressions (requires `lsp-compat` feature).
- **`find_references_single_file`** -- finds all same-file references to a variable or subroutine by byte offset.
- **`compute_links`** -- extracts document links from `use` and `require` statements with deferred resolution.

## Workspace role

Internal feature crate consumed by `perl-lsp` navigation request handlers. Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | Enables `TypeDefinitionProvider::find_type_definition` and LSP type imports |
| `slow_tests` | Enables slow/expensive integration tests |

## License

MIT OR Apache-2.0
