# Wave 2 Semantic Spine Plan

**Status**: Planning and execution guide for Wave 2.
**Intent**: Deliver the most reviewable and tested complete slice that makes semantic facts real **without** migrating user-facing providers yet.

---

## Why Wave 2 exists

The core problem is not missing parser output; it is **overlapping semantic truth represented in different shapes** across `perl-symbol`, `perl-workspace`, and `perl-semantic-analyzer`.

Wave 2 establishes one shared semantic spine:

- canonical anchors
- canonical occurrences
- stable entities
- typed edges
- provenance
- confidence

This wave is intentionally substrate-first. No completion/diagnostics/rename cutover yet.

---

## Wave 2 target state

```text
perl-symbol / exporter / workspace
  → emit canonical facts

perl-workspace
  → stores fact shards
  → builds typed definition/reference indexes
  → keeps old public APIs working
  → can compare old vs new query answers
```

---

## Scope guardrails (strict)

### In scope

1. `SymbolDecl -> EntityFact` adapter.
2. `SymbolRef -> OccurrenceFact` adapter.
3. `ExportInfo -> ExportSet` adapter.
4. `FileFactShard` write-through storage in workspace index.
5. Deterministic `DefinitionCandidate` multimap behind compatibility APIs.
6. Typed `ReferenceEdge` global index behind compatibility APIs.
7. Shadow-compare receipts for definition/reference/count-usage queries.
8. Semantic scorecard v1 for fact coverage and fixture coverage.

### Out of scope

- completion migration
- undefined-symbol diagnostic migration
- rename/safe-delete migration
- full package graph
- Moose/Moo generated member support
- external `@INC`/CPAN index
- on-disk semantic persistence
- full type/value-shape inference

---

## Box plan (8 parallel slices)

| Box | Deliverable | Provider behavior change? |
|---:|---|---|
| 1 | `SymbolDecl -> EntityFact` adapter | No |
| 2 | `SymbolRef -> OccurrenceFact` adapter | No |
| 3 | `ExportInfo -> ExportSet` adapter | No |
| 4 | `FileFactShard` write-through store | No |
| 5 | `DefinitionCandidate` multimap behind compatibility APIs | No / shadow-only optional |
| 6 | typed `ReferenceEdge` global index behind compatibility APIs | No / shadow-only optional |
| 7 | semantic query shadow-compare receipts | No |
| 8 | semantic scorecard v1 | No |

---

## Merge order

Do not merge all boxes blindly in numeric order.

1. Boxes 1–3 (exact adapters)
2. Box 8 (scorecard v1), if it consumes adapter outputs cleanly
3. Box 4 (file shard write-through)
4. Boxes 5–6 (candidate/ref indexes)
5. Box 7 (shadow receipts)

If Box 4 is ready and Boxes 5–6 are both clean, merge Box 4 first, then update/merge 5 and 6 one at a time.

---

## Definition of done for Wave 2

Wave 2 is complete when all of the following are true:

- Symbol declarations can become canonical entities.
- Symbol references can become canonical occurrences.
- Export analysis can become canonical export sets.
- Workspace can store per-file fact shards.
- Workspace can represent multiple definition candidates deterministically.
- Workspace can preserve typed references globally.
- Old/new query answers can be compared deterministically.
- Semantic scorecard can report fact and fixture coverage.

And all this is true while **existing provider behavior remains stable**.

---

## Practical review posture

Default first-pass review can prioritize broad, fast feedback and fix-forward loops.
Escalate to deep semantic review only when correctness is ambiguous (especially around dynamic boundaries and policy decisions).

---

## Path after Wave 2 (overall initiative)

Wave 2 creates the substrate. Subsequent waves should migrate user-facing behavior in risk-aware order.

### Wave 3 — imports and visibility

Build:

- `ImportSpec`
- import/export index
- `visible_symbols_at(uri, offset)`

Why first: this resolves high-value contradictions between completion, diagnostics, and refactors around imported symbols.

### Wave 4 — low-risk query consumer migration

Migrate first:

- goto-definition
- find-references
- count-usages

Why before rename/diagnostics: navigation regressions are easier to validate and roll back safely.

### Wave 5 — completion and undefined diagnostics

Move completion to visible-symbol/query-backed candidate sources first, then undefined-symbol diagnostics with confidence-aware behavior.

### Wave 6 — package graph and generated members

Support inheritance/role/generated-member semantics (`use parent`, `@ISA`, framework-generated accessors) as first-class graph edges.

### Wave 7 — value-shape-lite receiver modeling

Add lightweight receiver shape tracking (not full type inference) to improve method candidate ranking and degrade gracefully when unknown.

### Wave 8 — rename/safe-delete planning

Introduce refactor plans only after graph and visibility are mature, with dynamic-boundary-aware blocking/warnings.

### Wave 9 — incremental invalidation and release proof

Add semantic fingerprints/invalidation strategy and release scorecards (accuracy + latency + refactor safety) on representative real workspaces.

---

## One-line sequencing

```text
Wave 2: facts emitted, stored, indexed, compared
Wave 3: import/export visibility queryable
Wave 4: definition/reference query consumers migrate
Wave 5: completion + undefined diagnostics migrate
Wave 6: package graph + generated members
Wave 7: value-shape-lite + method candidates
Wave 8: rename/safe-delete plans
Wave 9: incremental invalidation + release proof
```

---

## Non-negotiables for the initiative

- Do not move rename early.
- Do not do full type inference before visibility/graph fundamentals.
- Do not let provider layers re-implement import/export semantics.
- Do not model dynamic Perl boundaries as exact.
- Do not cut over providers without shadow comparisons and scorecard evidence.
