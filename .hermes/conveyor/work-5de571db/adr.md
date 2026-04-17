# ADR-057: Incremental Semantic Analysis for perl-lsp

## Status
Proposed

## Context

The semantic analysis in perl-lsp performs a full re-analysis of the entire AST on every document edit, even when only a small portion of the file changed. For large Perl files (50KB+), this causes O(n) AST traversal on every keystroke, degrading interactive editing performance.

Two prior problems have been identified:

1. **Cache invalidation on every edit**: The `SemanticAnalyzer` cache in `DocumentStore::get_or_build_analyzer()` is keyed by `(uri, content_hash)`. Every `didChange` event changes the content hash, invalidating the entire cache. The only handler currently using this cache is `hover.rs`.

2. **API gap in incremental parsing**: The `perl-incremental-parsing` crate's `IncrementalDocument::apply_edits()` returns `ParseResult<()>` and discards the `changed_ranges` data that `IncrementalState::apply_edits()` produces internally. This blocks any incremental semantic analysis that would rely on knowing which AST regions were reparsed.

## Decision

We will implement incremental semantic analysis in two phases:

### Phase 1 (API Foundation — Required First)

**Before any semantic analysis work**, extend the `perl-incremental-parsing` crate API to expose `changed_ranges`:

- Modify `IncrementalDocument::apply_edits()` to return `ParseResult<ChangedRanges>` where `ChangedRanges` contains the byte ranges of the AST that were reparsed vs. reused
- This is a prerequisite for Phase 2 — without this API, incremental semantic analysis cannot be implemented as described in the original plan

Also correct the misleading documentation:

- Remove the "≤1ms for typical changes" claim from `SemanticAnalyzer` struct documentation until incremental analysis is actually implemented and verified

### Phase 2 (Incremental Semantic Analysis)

Once `ChangedRanges` is exposed:

- Implement `SemanticAnalyzer::analyze_incremental(state: &IncrementalAnalyzerState, changed_ranges: &[Range])` that re-analyzes only AST regions affected by edits and their dependent scopes
- Store `SemanticAnalyzer` state alongside `IncrementalDocument` state in the document store
- On each `didChange`, invalidate only the semantic analysis for changed regions, reusing analysis for unchanged subtrees

### Why Not Route Through Existing Cache (Phase 1 Original Plan)

The original plan proposed routing all handlers through the existing `(uri, content_hash)`-keyed cache as a "quick win." However:

- The cache is invalidated on every `didChange` (content hash changes)
- The benefit window is only multiple LSP requests arriving before the next keystroke — a millisecond-scale window
- This provides negligible real-world performance improvement
- It risks creating a false sense of progress without addressing the actual problem

Routing through the cache remains useful for avoiding redundant analysis when multiple requests arrive on the same document version, but this is a separate concern from true incremental analysis.

## Alternatives Considered

### Alternative 1: Implement Phase 1 as originally planned (route through cache)
- **Rejected because**: The `(uri, content_hash)` cache is invalidated on every keystroke. The "quick win" benefit is negligible — only helping when multiple LSP requests arrive within milliseconds. The misleading performance claim would remain.

### Alternative 2: Cache at AST subtree level using node ranges
- **Rejected because**: Perl's dynamic scoping ( closures, `our` variables, `use`d packages) makes it complex to track which subtrees depend on which definitions. The incremental approach handles this properly once `changed_ranges` is available.

### Alternative 3: Full incremental analysis from scratch without IncrementalDocument
- **Rejected because**: The `IncrementalDocument` already tracks which AST regions were reparsed. Leveraging this is more maintainable than duplicating the change detection logic.

## Consequences

### Tradeoffs
- **Phase 1 adds API design work** before semantic analysis can begin — this is necessary to avoid building on a flawed foundation
- **Phase 2 requires storing SemanticAnalyzer state** alongside IncrementalDocument state, increasing memory usage
- **Subroutine-level invalidation must account for Perl's dynamic scoping** — scope analysis can cross subroutine boundaries via closures and `our` variables

### Benefits
- True O(k) incremental analysis where k is the size of changed regions, not O(n) for the entire AST
- No misleading performance claims — the "≤1ms" claim is removed until verified
- Clean separation between incremental parsing infrastructure and semantic analysis

### Risks
- **API changes to `IncrementalDocument`** may affect other consumers — must be backward compatible or carefully migration
- **Scope invalidation complexity** — when a symbol definition changes, all scopes that reference it must be invalidated
- **Testing complexity** — need to verify incremental analysis produces identical results to full analysis

## Dependencies

- `perl-incremental-parsing`: Must expose `ChangedRanges` from `apply_edits()`
- `perl-semantic-analyzer`: Must implement `analyze_incremental()` method
- `perl-lsp`: Must wire incremental analysis into the document store lifecycle

## Related ADRs

- ADR-050: Incremental Parsing Architecture (for context on IncrementalDocument design)