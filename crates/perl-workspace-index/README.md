# perl-workspace-index

Workspace indexing and cross-file lookup for Perl tooling.

Use this crate when you need to track open documents, build a symbol index, or
answer workspace-wide queries such as references, symbols, and rename targets.

## Where it fits

`perl-workspace-index` sits above `perl-parser-core` and below the editor and
refactoring layers. It owns document storage, incremental updates, and the
workspace symbol index that the rest of the stack queries.

## Key entry points

- `workspace::document_store::DocumentStore`
- `workspace::workspace_index::WorkspaceIndex`
- `workspace::workspace_rename::WorkspaceRename`
- `Parser`, `Node`, `NodeKind`, and `SourceLocation` re-exports from `perl-parser-core`

## Typical use

Use `perl-workspace-index` when you already have parsed documents and need to
keep the workspace view current as files change. If you only need syntax trees,
depend on `perl-parser-core` instead.
