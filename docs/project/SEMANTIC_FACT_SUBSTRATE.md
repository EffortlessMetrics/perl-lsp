# Semantic Fact Substrate (Compiler-lite) — v1 Contract

## Purpose

This document defines the **canonical semantic substrate** we will build before provider cutover.
It is intentionally scoped as a planning and contract artifact: it does **not** claim that migration is complete.

Guiding principle for every box in this epic:

> Produce the most reviewable and tested complete slice of this specific change. Do not optimize for a tiny diff; optimize for a reviewer being able to prove the change is correct quickly.

## Why this exists

Semantic data is currently spread across multiple crates:

- `perl-symbol`: exact AST-oriented declarations/references
- `perl-semantic-analyzer`: scoped and framework-informed synthesis
- `perl-workspace-index`: cross-file indexing and query surfaces
- export/import analysis paths in analyzer/workspace layers

The substrate adds one neutral vocabulary layer so adapters and storage are explicit, testable, and migration-safe.

## Layer model (target)

```text
Per-file producers
  ├─ perl-symbol (exact facts)
  └─ perl-semantic-analyzer (synthesized/framework facts)
        ↓ adapters
Canonical facts
  └─ perl-semantic-facts (IDs, facts, edges, provenance, confidence)
        ↓ write-through
Workspace store/index
  └─ perl-workspace-index (file shards, invalidation, candidate sets)
        ↓ compatibility APIs + shadow compare
Query contract
  └─ definitions/references/visible symbols/rename/safe-delete
        ↓ final migration
LSP providers
  └─ consume query APIs (not private semantic maps)
```

## Crate responsibilities

- `perl-semantic-facts` (new): types only; no parsing, no provider logic, no workspace ownership.
- `perl-symbol`: emit exact symbol declarations/references with high-confidence provenance.
- `perl-semantic-analyzer`: emit framework, inheritance, role, and generated-member facts with explicit confidence.
- `perl-workspace-index`: persist per-file fact shards, build query indexes, invalidate deterministically.
- provider crates: consume stable query APIs and stop walking crate-private semantic structures.

## Fact model summary (v1 vocabulary)

Core IDs:

- `FileId`, `ScopeId`, `EntityId`, `AnchorId`, `OccurrenceId`, `EdgeId`, `DiagnosticId`

Core facts:

- `AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`, `DiagnosticFact`

Core enums:

- `EntityKind`, `OccurrenceKind`, `EdgeKind`, `Provenance`, `Confidence`

Expected minimum coverage for these enums is tracked in the Box 1 implementation prompt.

## Provenance and confidence policy

Every non-trivial semantic result should carry both:

- **Provenance** (`ExactAst`, `SemanticAnalyzer`, `FrameworkSynthesis`, `ImportExportInference`, etc.)
- **Confidence** (high-to-low certainty used for ranking and safety decisions)

Policy:

1. Prefer exact facts over heuristic facts when both exist.
2. Keep heuristic/dynamic results visible but explicitly tagged.
3. Do not silently upgrade confidence when data crosses crate boundaries.

## Dynamic-boundary policy

When static certainty is unavailable (for example: `eval STRING`, dynamic `require`, dynamic import dispatch, heavy typeglob indirection):

- record an explicit dynamic-boundary occurrence/edge,
- keep navigation/editing conservative,
- preserve explainability in scorecards and receipts.

## Query API contract (provider-facing target)

```rust
symbol_at(uri, pos)
definitions(occurrence, policy)
references(entity, scope, filter)
visible_symbols_at(uri, pos, context)
method_candidates(receiver, prefix)
rename_plan(entity, new_name)
safe_delete_plan(entity)
```

Contract intent:

- deterministic ordering,
- candidate-aware results where ambiguity exists,
- explicit handling of unsupported/dynamic boundaries,
- predictable serialization for fixtures and shadow receipts.

## Migration order

Dependency order remains:

```text
facts schema
→ exact adapters
→ workspace store
→ query APIs
→ provider migration
```

Execution waves:

- **Wave 1 (rails)**: docs + fixture banks + scorecards + fact crate skeleton.
- **Wave 2 (adapters/store)**: `SymbolDecl -> EntityFact`, `SymbolRef -> OccurrenceFact`, `ExportInfo -> ExportSet`, file shard write-through, candidate multimap, typed reference index.
- **Wave 3 (first UX jump)**: import spec extraction, visible symbols query, completion/undefined diagnostics behind feature flags.

## In scope vs deferred (v1)

In scope now:

- canonical vocabulary,
- fixture and regression banks,
- shadow-compare receipt design,
- release-readiness scorecard hooks.

Deferred until adapters/store are in:

- full provider cutover,
- broad ranking retuning,
- dynamic execution modeling beyond conservative boundaries.
