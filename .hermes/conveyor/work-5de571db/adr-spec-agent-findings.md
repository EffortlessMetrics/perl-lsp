# ADR/Spec Findings — work-5de571db

## What This ADR Decides
The ADR decides to restructure the incremental semantic analysis work into two phases: (1) extend the `IncrementalDocument` API to expose `changed_ranges` before any semantic analysis work begins, and (2) implement true incremental semantic analysis on top of that API. The original plan's "quick win" of routing through the existing `(uri, content_hash)`-keyed cache is rejected as providing negligible real-world benefit.

## Key Decision
**Block Phase 2 on API design**: `IncrementalDocument::apply_edits()` must return `ChangedRanges` containing byte ranges of AST that were reparsed vs. reused. Without this API, incremental semantic analysis cannot be implemented as described and the misleading "≤1ms" performance claim cannot be achieved.

## Alternatives Considered

1. **Phase 1 as originally planned (route through cache)** — Rejected because the `(uri, content_hash)` cache is invalidated on every keystroke. Only helps when multiple LSP requests arrive within milliseconds. Creates false sense of progress.

2. **Cache at AST subtree level** — Rejected because Perl's dynamic scoping makes tracking scope dependencies complex. Incremental approach handles this properly once `changed_ranges` is available.

3. **Full incremental from scratch without IncrementalDocument** — Rejected because IncrementalDocument already tracks reparsed regions; leveraging it is more maintainable.

## Consequences

**Benefits:**
- True O(k) incremental analysis where k is changed region size, not O(n) for entire AST
- No misleading performance claims — "≤1ms" removed until verified
- Clean API separation between parsing and semantic analysis

**Tradeoffs:**
- Phase 1 adds API design work before semantic analysis begins
- Phase 2 requires storing SemanticAnalyzerState alongside IncrementalDocument, increasing memory
- Scope invalidation must account for Perl's dynamic scoping across subroutine boundaries

**Risks:**
- API changes to `IncrementalDocument` may affect other consumers
- Scope invalidation complexity with closures, `our` variables, `use`d packages
- Need to verify incremental analysis produces identical results to full analysis

## Acceptance Criteria
1. `IncrementalDocument::apply_edits()` returns `ChangedRanges` (reparsed + reused byte ranges)
2. `analyze_incremental()` produces identical output to `analyze()` for all edit patterns
3. Re-analyzing single-line edit in 50KB file visits O(k) nodes, not O(n)
4. All handlers (rename, references, navigation, hover) use incremental analysis
5. No `SemanticAnalyzerState` leaks on `didClose`