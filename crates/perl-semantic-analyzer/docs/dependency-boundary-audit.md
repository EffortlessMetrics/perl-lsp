# Dependency Boundary Audit: `perl-semantic-analyzer` → `perl-workspace`

This note captures all current `perl-workspace` coupling points in `perl-semantic-analyzer` and classifies each one to guide follow-up extractions.

## Current coupling map

1. **Crate-level re-export of workspace index module**
   - Location: `src/lib.rs`
   - Usage: `pub use perl_workspace::workspace_index;`
   - Classification: **accidental convenience**
   - Why: This forwards workspace-facing storage/query types through the analyzer crate boundary.

2. **Symbol key vocabulary used by declaration provider logic**
   - Location: `src/analysis/declaration.rs`
   - Usage: `SymKind`, `SymbolKey`
   - Classification: **shared vocabulary**
   - Why: Declaration lookup currently returns workspace key types.

3. **Semantic query facade accepts workspace index object**
   - Location: `src/analysis/semantic/query_facade.rs`
   - Usage: `WorkspaceIndex`
   - Classification: **query/store coupling**
   - Why: Query facade takes workspace-owned index state directly.

## Extraction order

1. Move `SymKind` / `SymbolKey` vocabulary to semantic-neutral shared crate surface.
2. Replace `lib.rs` re-export with analyzer-owned neutral fact interfaces.
3. Refactor `query_facade` methods to consume semantic fact collections and keep workspace query policy in `perl-workspace`.

## Non-goals for this audit

- No behavior change.
- No scorecard/fixture movement.
- No provider-facing API reshaping in this PR.
