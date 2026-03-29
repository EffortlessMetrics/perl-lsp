# perl-lsp-navigation

Navigation providers for Perl source.

## When to use this crate

Use `perl-lsp-navigation` when you want the navigation slice of Perl LSP
features without pulling in the full server binary.

It is a good fit for Rust tooling that needs:

- definition and reference helpers
- workspace symbol search
- type hierarchy or type-definition support
- document-link extraction from Perl imports

## Public API

- `WorkspaceSymbolsProvider` and `WorkspaceSymbol`: index parsed documents and search symbols.
- `TypeHierarchyProvider`, `TypeHierarchyItem`, and `TypeHierarchySymbolKind`: build and resolve Perl type hierarchies.
- `TypeDefinitionProvider`: go-to-type-definition support for variables, method calls, constructors, and `bless` expressions.
- `find_references_single_file`: same-file reference lookup by byte offset.
- `compute_links`: extracts document links from `use` and `require` statements.

## Example

```rust,ignore
use perl_lsp_navigation::{TypeHierarchyProvider, WorkspaceSymbolsProvider};

let type_hierarchy = TypeHierarchyProvider::new(workspace_index);
let workspace_symbols = WorkspaceSymbolsProvider::new(workspace_index);
```

## Workspace role

Internal feature crate consumed by `perl-lsp` navigation request handlers. It
is mostly a workspace building block rather than a standalone end-user crate.

## Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | Enables LSP-specific type-definition behavior |
| `slow_tests` | Enables slow or expensive integration tests |

## License

MIT OR Apache-2.0
