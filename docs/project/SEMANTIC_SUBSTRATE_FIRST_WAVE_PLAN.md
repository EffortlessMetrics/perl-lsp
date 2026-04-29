# Semantic Substrate First-Wave Plan (Rails Before Cutover)

This document defines the first-wave execution plan for improving workspace-wide awareness and semantic analysis without cutting LSP providers over yet.

## Principle

Produce the **most reviewable and tested complete slice** for each PR. Do not optimize for tiny diffs; optimize for reviewers proving correctness quickly.

The dependency chain is:

```text
facts schema
→ exact adapters
→ workspace store
→ query APIs
→ provider migration
```

First wave focuses on preparing the rails:

1. Canonical semantic vocabulary (facts, typed IDs, provenance, confidence).
2. Fixture-heavy regression banks that lock down current behavior.
3. Scorecards and migration contracts so follow-on implementation PRs are measurable.

## Box Overview

| Box | Purpose | Merge Dependency |
|---:|---|---|
| 1 | Add `perl-semantic-facts` crate skeleton | Foundation; merge first if good |
| 2 | Add semantic fixture / scorecard harness | Independent |
| 3 | Add workspace definition ambiguity regression bank | Independent |
| 4 | Add typed-reference regression bank | Independent |
| 5 | Add import/export visibility regression bank | Independent |
| 6 | Add `SymbolRef` phase-2 fixture bank | Independent |
| 7 | Add package/class/role/generated-member fixture bank | Independent |
| 8 | Add semantic-substrate architecture doc / query API contract | Independent |
| 9 | Add workspace shadow-compare design stub | Independent (doc/test-only) |
| 10 | Add release-readiness semantic scorecard integration | Independent |

## Suggested Merge Order

1. Box 8 docs
2. Box 2 scorecard harness
3. Boxes 3–7 fixture banks
4. Box 9 shadow-compare receipt shape
5. Box 10 release-readiness integration
6. Box 1 semantic-facts crate skeleton

This ordering reduces architecture ambiguity before load-bearing implementation work lands.

## Box 1 Contract: `perl-semantic-facts`

`perl-semantic-facts` is a neutral vocabulary crate, not a parser, not a provider, and not workspace storage.

### Core IDs

- `FileId`
- `ScopeId`
- `EntityId`
- `AnchorId`
- `OccurrenceId`
- `EdgeId`
- `DiagnosticId`

### Core Fact Types

- `AnchorFact`
- `EntityFact`
- `OccurrenceFact`
- `EdgeFact`
- `DiagnosticFact`

### Core Enums

- `EntityKind`
- `OccurrenceKind`
- `EdgeKind`
- `Provenance`
- `Confidence`

The initial implementation should include deterministic serialization/roundtrip tests and crate docs clarifying scope boundaries.

## Query API Contract (Targeted, Not Yet Fully Implemented)

Future provider consumers should target query APIs rather than internal maps:

```rust
symbol_at(uri, pos)
definitions(occurrence, policy)
references(entity, scope, filter)
visible_symbols_at(uri, pos, context)
method_candidates(receiver, prefix)
rename_plan(entity, new_name)
safe_delete_plan(entity)
```

## Wave 2 (Facts Become Real, Providers Stay Stable)

Wave 2 is where semantic facts move from design to runtime substrate. The scope is intentionally below LSP provider cutover.

### Wave 2 target state

```text
perl-symbol / exporter / workspace
  → emit canonical facts

perl-workspace
  → stores fact shards
  → builds typed definition/reference indexes
  → keeps old public APIs working
  → can compare old vs new query answers
```

### Wave 2 boxes (8 parallel tracks)

| Box | Purpose | Provider behavior change? |
|---:|---|---|
| 1 | `SymbolDecl -> EntityFact` adapter | No |
| 2 | `SymbolRef -> OccurrenceFact` adapter | No |
| 3 | `ExportInfo -> ExportSet` adapter | No |
| 4 | `FileFactShard` write-through store in `perl-workspace` | No |
| 5 | `DefinitionCandidate` multimap behind compatibility APIs | No (shadow-compatible) |
| 6 | Typed `ReferenceEdge` global index behind compatibility APIs | No (shadow-compatible) |
| 7 | Shadow-compare receipts for definition/reference queries | No |
| 8 | Semantic scorecard v1: fact counts + fixture coverage | No |

### Wave 2 merge order

Do not merge all boxes in numeric order:

1. Boxes 1–3 (exact adapters)
2. Box 8 (scorecard v1, if adapter outputs are cleanly consumed)
3. Box 4 (`FileFactShard` write-through)
4. Boxes 5–6 (definition/reference indexes behind compatibility APIs)
5. Box 7 (shadow-compare receipts)

### Explicit Wave 2 non-goals

- Completion migration
- Undefined-symbol diagnostics migration
- Rename/safe-delete migration
- Full package graph
- External `@INC`/CPAN indexing
- On-disk semantic persistence

### Wave 2 success criteria

After Wave 2 lands, we should be able to say:

```text
Symbol declarations can become canonical entities.
Symbol references can become canonical occurrences.
Export analysis can become canonical export sets.
Workspace can store per-file fact shards.
Workspace can represent multiple definition candidates.
Workspace can preserve typed references globally.
Old and new query answers can be compared.
Semantic scorecard can show fact coverage.
```

## Overall Path After Wave 2

### Wave 3 (Imports and visibility become first-class)

1. `ImportSpec` extraction for `use` forms.
2. `require Module; Module->import(...)` extraction.
3. Resolve `ExportSet` into `VisibleSymbols`.
4. Add `visible_symbols_at(uri, offset)`.
5. Add import/export coverage rows to semantic scorecards.
6. Add completion shadow-compare using `VisibleSymbols`.
7. Add undefined-symbol diagnostic shadow-compare.
8. Add dynamic import boundary policy tests.

### Wave 4 (Low-risk query consumer migration)

- Migrate `goto-definition`, `find-references`, and `count-usages` to provider-facing semantic queries with compatibility wrappers and scorecard validation.

### Wave 5 (First major UX cutover)

- Migrate completion and undefined-symbol diagnostics onto confidence-aware semantic visibility, still behind staged rollout controls.

### Wave 6 (Perl package graph and generated members)

- Add inheritance/role/generated-member modeling and integrate with completion/navigation/diagnostics.

### Wave 7 (Value-shape-lite for receiver-aware behavior)

- Add lightweight receiver/value-shape inference to improve method candidates and ranking without full type inference.

### Wave 8 (Refactor safety)

- Build `rename_plan`/`safe_delete_plan` with dynamic-boundary awareness and explicit unsafe/ambiguous outcomes.

### Wave 9 (Incremental invalidation + release proof)

- Add per-file semantic fingerprints and invalidation rules, then prove quality/latency on representative real workspaces.

## Out of Scope for First Wave

- Full provider migration.
- Broad rewrite of existing semantic producers.
- Claiming implementation completeness before fixtures and scorecards prove behavior.

## Verification Expectations by Box

- Box 1: `cargo test -p perl-semantic-facts`, `cargo check --workspace --all-targets`
- Box 2: `cargo test -p xtask`, `cargo xtask semantic-scorecard`, `cargo check --workspace --all-targets`
- Boxes 3–7: crate-targeted test commands per fixture bank plus workspace check
- Box 8/9/10: at minimum `cargo check --workspace --all-targets`

## Canonical Inputs and Truth Sources

- Workspace/version truth: `Cargo.toml`
- Capability truth: `features.toml`
- Evidence-backed status: `docs/project/CURRENT_STATUS.md`
- Canonical planning: `docs/project/ROADMAP.md`
