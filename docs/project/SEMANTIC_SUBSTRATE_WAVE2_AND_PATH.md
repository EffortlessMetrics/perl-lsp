# Semantic Substrate Wave 2 Plan and Forward Path

This document captures Wave 2 execution for the semantic substrate initiative and the milestone path that follows it.

## Initiative Goal

Move from overlapping semantic truth across `perl-symbol`, `perl-workspace`, and `perl-semantic-analyzer` to one canonical semantic spine:

- anchors
- occurrences
- stable entities
- typed edges
- provenance
- confidence

The review standard remains: **most reviewable and tested complete slice**.

## Wave 2 Scope (No Provider Cutover Yet)

Wave 2 makes canonical facts real and queryable internally, while keeping current provider behavior stable.

### In Scope

1. `SymbolDecl -> EntityFact` adapter.
2. `SymbolRef -> OccurrenceFact` adapter.
3. `ExportInfo -> ExportSet` adapter.
4. `FileFactShard` write-through storage in `perl-workspace`.
5. Deterministic `DefinitionCandidate` multimap behind compatibility APIs.
6. Typed `ReferenceEdge` global index behind compatibility APIs.
7. Shadow-compare receipts for old/new query answers.
8. Semantic scorecard v1 for fact coverage and fixture coverage.

### Explicitly Out of Scope

- Completion migration.
- Undefined-symbol diagnostic migration.
- Rename/safe-delete migration.
- Full package graph and generated-members cutover.
- On-disk semantic persistence.

## Wave 2 Box Map

| Box | Purpose | Provider behavior change? |
|---:|---|---|
| 1 | `SymbolDecl -> EntityFact` adapter | No |
| 2 | `SymbolRef -> OccurrenceFact` adapter | No |
| 3 | `ExportInfo -> ExportSet` adapter | No |
| 4 | `FileFactShard` write-through store in `perl-workspace` | No |
| 5 | `DefinitionCandidate` multimap behind compatibility APIs | No / shadow-only internals |
| 6 | typed `ReferenceEdge` global index behind compatibility APIs | No / shadow-only internals |
| 7 | shadow-compare receipts for definition/reference queries | No |
| 8 | semantic scorecard v1: fact counts + fixture coverage | No |

## Wave 2 Merge Order

Do not merge strictly in numeric order. Merge by dependency and reviewability:

1. Boxes 1–3 (exact adapters).
2. Box 8 (scorecard v1), if it cleanly consumes adapter outputs.
3. Box 4 (`FileFactShard` write-through).
4. Boxes 5–6 (candidate/reference indexing internals).
5. Box 7 (shadow-compare receipts).

If Box 4 lands before Boxes 5–6, cascade-update those PRs and merge indexing changes one at a time.

## Wave 2 Success Criteria

Wave 2 is complete when the repository can truthfully state:

- Symbol declarations can be emitted as canonical entities.
- Symbol references can be emitted as canonical occurrences.
- Export analysis can be emitted as canonical export sets.
- Workspace stores per-file fact shards and reindex/removal is deterministic.
- Workspace can hold multiple deterministic definition candidates.
- Workspace preserves typed references globally.
- Old/new query answers can be shadow-compared via deterministic receipts.
- Scorecard output reports fact and fixture coverage, including unavailable rows.

## Post-Wave-2 Path (Milestones)

### Milestone 1 (Wave 3): Imports and Visibility

Build:

- `ImportSpec` extraction.
- import/export resolution index.
- `visible_symbols_at(uri, offset)` query.

First coverage should include:

- `use Foo;`
- `use Foo ();`
- `use Foo qw(a b);`
- `use Foo ':tag';`
- `require Foo; Foo->import(qw(a b));`

Primary goal: make import/export semantics queryable before provider cutover.

### Milestone 2 (Wave 4): Low-Risk Query Consumer Migration

Migrate first:

- goto-definition
- find-references
- count-usages

Use shadow-compare receipts and scorecards to prove new paths are same or improved before full cutover.

### Milestone 3 (Wave 5): Completion on Visible Symbols

Move completion onto:

- local lexical facts
- `visible_symbols_at`
- definition candidates

Keep old path as fallback under feature flag until scorecard deltas are clean.

### Milestone 4 (Wave 5+): Diagnostics on Confidence-Aware Facts

Migrate undefined-symbol and low-risk unused checks onto confidence/provenance-aware facts after completion stabilizes.

### Milestone 5 (Wave 6): Package Graph + Generated Members

Add package inheritance/role/generated-member semantics and feed completion/navigation/diagnostics.

### Milestone 6 (Wave 7): Value-Shape-Lite

Add lightweight receiver-shape inference for better method candidate quality without full type inference.

### Milestone 7 (Wave 8): Refactor Safety

Implement `rename_plan` and `safe_delete_plan` against fact graph + confidence + dynamic boundaries.

### Milestone 8 (Wave 8+): Incremental Invalidation and Performance

Add semantic fingerprints and targeted invalidation so higher semantic quality does not regress edit-time latency.

### Milestone 9 (Wave 9): Release Proof

Track release-readiness scorecards for semantic quality and latency on representative real-world workspaces.

## One-Line Sequence

```text
Wave 2: facts emitted, stored, indexed, and comparable
Wave 3: imports/exports -> visible_symbols_at
Wave 4: definition/reference query consumers migrate
Wave 5: completion + diagnostics migrate behind flags
Wave 6: package graph + generated members
Wave 7: value-shape-lite + method candidates
Wave 8: rename/safe-delete planning
Wave 9: incremental invalidation + release proof
```

## Guardrails

- Do not migrate rename early.
- Do not attempt full type inference before visibility/package graph basics.
- Do not re-fragment semantic truth across multiple internal models.
- Do not cut over providers without shadow comparison and scorecard evidence.
