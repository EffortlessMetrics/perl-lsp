# Semantic Substrate Initiative Plan (Rails, Wave 2, and Migration Path)

This document captures the current execution plan for the semantic-substrate initiative:

- what Wave 1 delivered,
- what Wave 2 should deliver,
- and how we sequence user-visible migrations afterward.

## Principle

Produce the **most reviewable and tested complete slice** for each PR. Optimize for evidence density and correctness proof, not smallest diff.

Core dependency chain:

```text
facts schema
→ exact adapters
→ workspace storage
→ typed indexes + query compatibility
→ shadow-compare receipts + scorecards
→ provider migration
```

## Initiative Goal

Stop carrying overlapping semantic truth in incompatible shapes across `perl-symbol`, `perl-workspace`, and `perl-semantic-analyzer`.

Target architecture:

- one canonical typed fact substrate,
- facts with anchors, occurrences, stable entities, typed edges, provenance, and confidence,
- `perl-workspace` as a derived index/query layer rather than a semantic attic.

## Wave 1 (Rails) — Delivered Scope

Wave 1 prepared foundations without provider cutover:

1. Canonical semantic vocabulary (`perl-semantic-facts`).
2. Fixture/regression banks for current behavior.
3. Scorecard and contract scaffolding for measurable follow-on work.
4. Query-contract and shadow-compare design rails.

## Wave 2 (Facts Become Real, No Provider Migration)

Wave 2 must make facts flow through producers and workspace internals while preserving existing public behavior.

### Target State After Wave 2

```text
perl-symbol / exporter / workspace
  → emit canonical facts

perl-workspace
  → stores fact shards
  → builds typed definition/reference indexes
  → keeps old public APIs working
  → can compare old vs new query answers
```

No completion migration yet. No diagnostics migration yet. No rename migration yet.

### Wave 2 Boxes

| Box | Purpose | Provider behavior change? |
|---:|---|---|
| 1 | `SymbolDecl -> EntityFact` adapter | No |
| 2 | `SymbolRef -> OccurrenceFact` adapter | No |
| 3 | `ExportInfo -> ExportSet` adapter | No |
| 4 | `FileFactShard` write-through store in `perl-workspace` | No |
| 5 | `DefinitionCandidate` multimap behind compatibility APIs | No / shadow-only acceptable |
| 6 | Typed `ReferenceEdge` global index behind compatibility APIs | No / shadow-only acceptable |
| 7 | Shadow-compare receipts for definition/reference queries | No |
| 8 | Semantic scorecard v1: fact counts + fixture coverage | No |

### Merge Order for Wave 2

Do not merge blindly in numeric order.

```text
1. Boxes 1–3: exact adapters
2. Box 8: scorecard v1 (if consuming adapter outputs cleanly)
3. Box 4: FileFactShard write-through
4. Boxes 5–6: candidate/reference indexes
5. Box 7: shadow compare receipts
```

If Box 4 lands before 5/6 are ready, merge Box 4 first and rebase/cascade 5/6 on top.

### Wave 2 Completion Criteria

Wave 2 is successful when:

```text
Symbol declarations can become canonical entities.
Symbol references can become canonical occurrences.
Exporter analysis can become canonical export sets.
Workspace can store per-file fact shards.
Workspace can represent multiple definition candidates.
Workspace can preserve typed references globally.
Old and new query answers can be compared.
Semantic scorecard can show fact coverage.
```

## Path After Wave 2

After Wave 2, facts are emitted, stored, indexed, and comparable. Then migrate consumers in risk-aware order.

### Wave 3 — Imports and Visibility

Build:

- `ImportSpec` extraction,
- `ImportExportIndex`,
- `visible_symbols_at(uri, offset) -> Vec<VisibleSymbol>`.

Why first: quickest user-visible semantic payoff and resolves completion/diagnostics/import contradictions.

### Wave 4 — Low-Risk Query Consumer Migration

Migrate first:

- goto-definition,
- find-references,
- count-usages,
- safe candidate ranking upgrades.

Why before diagnostics/rename: navigation regressions are easier to validate and recover from.

### Wave 5 — Completion + Undefined Diagnostics

Move completion to visibility-aware graph first, then undefined diagnostics with confidence/provenance/dynamic-boundary awareness.

### Wave 6 — Package Graph + Generated Members

Add inheritance/role/generated-member modeling and wire into completion/navigation/diagnostics.

### Wave 7 — Value-Shape Lite

Add lightweight receiver shape (not full type inference) to improve method candidate ranking.

### Wave 8 — Rename/Safe-Delete Plans

Only after graph quality is proven by scorecards and shadow-compare evidence.

### Wave 9 — Incremental Invalidation + Release Proof

Harden performance/invalidation and publish quality scorecards against real workspace baselines.

## Explicitly Out of Scope for Wave 2

- Completion migration.
- Undefined-symbol diagnostics migration.
- Rename/safe-delete migration.
- Full package graph.
- Generated-member framework completeness (Moose/Moo/etc.).
- External `@INC`/CPAN index.
- On-disk semantic persistence.
- Full type/value-shape inference.

## Verification Expectations

For implementation PRs in this initiative, prefer crate-scoped checks plus workspace compile checks:

- `cargo test -p <crate>`
- `cargo check --all-targets -p <crate>`
- `cargo xtask fmt`
- `cargo clippy -p <crate>`
- `just pr-fast`

Use shadow-compare receipts and scorecards as migration evidence before provider cutovers.

## Canonical Truth Sources

Before stating project counts, versions, or capability coverage, verify against:

- `Cargo.toml` (workspace members/version)
- `features.toml` (capability catalog)
- `docs/project/CURRENT_STATUS.md` (evidence-backed metrics)
- `docs/project/ROADMAP.md` (canonical roadmap)
