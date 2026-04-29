# Semantic Fact Substrate (Compiler-lite) — v1 Contract

## Goal

Produce the **most reviewable and tested complete slice** of semantic-substrate groundwork so later PRs can migrate providers without guessing.

This document defines the neutral semantic-fact vocabulary and migration rails for workspace-wide awareness (definitions, references, imports/exports, inheritance, generated members, rename/safe-delete safety).

## Scope (v1) vs deferred

### In scope now

- Canonical fact vocabulary and typed IDs (crate target: `perl-semantic-facts`)
- Fixture-first regression banks for current behavior
- Deterministic scorecard harness rows for semantic UX outcomes
- Shadow-compare receipt shape for old-vs-new query answers
- Query API contract that providers will eventually consume

### Explicitly deferred

- Provider cutover to new query APIs
- Any immediate behavior rewrite in existing LSP providers
- Full dynamic-language precision beyond conservative boundaries

## Layer diagram

```text
Perl source files
  -> perl-symbol (exact AST-origin facts)
  -> perl-semantic-analyzer (scoped + framework + synthetic facts)
  -> perl-workspace-index (storage, invalidation, query surfaces)
  -> LSP providers (definition/reference/completion/rename/etc.)

Cross-cutting:
  perl-semantic-facts = canonical vocabulary shared between producers + store
```

## Crate responsibilities

- `perl-symbol`: exact declarations/references from parse trees.
- `perl-semantic-analyzer`: framework-aware semantic enrichment (Moose/Moo, exporter patterns, scoped analysis).
- `perl-workspace-index` (workspace crate): cross-file storage, invalidation, and query execution.
- `perl-semantic-facts` (new): neutral type layer for IDs, anchors, entities, occurrences, edges, diagnostics, provenance, confidence.

`perl-semantic-facts` is intentionally not a parser, not an LSP provider, and not a workspace store.

## Canonical fact model summary

Target fact families:

- Identity: `FileId`, `ScopeId`, `EntityId`, `AnchorId`, `OccurrenceId`, `EdgeId`, `DiagnosticId`
- Location: anchors/ranges tied to files
- Meaning: entities, occurrences, typed edges, diagnostics
- Quality: provenance + confidence

Expected model qualities:

- Typed IDs instead of raw strings in public structs
- Deterministic serialization/formatting for snapshots and receipts
- Stable, migration-friendly representation for write-through storage

## Provenance and confidence policy

Every fact should carry both:

- `provenance`: where this fact came from (`ExactAst`, `SemanticAnalyzer`, `FrameworkSynthesis`, `ImportExportInference`, etc.)
- `confidence`: how strongly it should influence rank/safety (exact > inferred > heuristic > boundary)

Policy:

- Exact AST facts should dominate ranking and safety decisions.
- Framework-synthesized facts are valid but must stay labeled.
- Heuristic/search fallback facts should never masquerade as exact.

## Dynamic-boundary policy

Perl dynamic constructs are first-class boundaries, not silent misses.

Examples:

- `AUTOLOAD`
- `eval STRING`
- dynamic `require`
- dynamic import invocation

When exact resolution is unavailable, emit boundary-classified occurrences/edges so downstream UX can remain conservative and explain uncertainty.

## First-wave plan (prepare rails, not provider cutover)

Dependency chain:

```text
facts schema
-> exact adapters
-> workspace store
-> query APIs
-> provider migration
```

Parallel first-wave boxes:

1. `perl-semantic-facts` crate skeleton (types/tests/docs)
2. semantic fixture + scorecard harness
3. definition ambiguity regression bank
4. typed-reference regression bank
5. import/export visibility regression bank
6. `SymbolRef` phase-2 fixture bank
7. package/class/role/generated-member fixture bank
8. this architecture contract
9. workspace shadow-compare receipt shape
10. release-readiness semantic criteria

## Migration order (implementation waves)

### Wave 2 (adapters + write-through)

1. `SymbolDecl -> EntityFact`
2. `SymbolRef -> OccurrenceFact`
3. `ExportInfo -> ExportSet`
4. `FileFactShard` write-through in workspace
5. `DefinitionCandidate` multimap behind compatibility APIs
6. typed global `ReferenceEdge` index behind compatibility APIs

### Wave 3 (query enablement)

1. `ImportSpec` extraction
2. `visible_symbols_at` query
3. completion uses `visible_symbols_at` behind flag
4. undefined diagnostics uses `visible_symbols_at` behind flag

Provider migration follows only after shadow-compare evidence is stable.

## Query API contract (target surface)

```rust
symbol_at(uri, pos)
definitions(occurrence, policy)
references(entity, scope, filter)
visible_symbols_at(uri, pos, context)
method_candidates(receiver, prefix)
rename_plan(entity, new_name)
safe_delete_plan(entity)
```

Contract notes:

- Return deterministic ordering for ties/ambiguity.
- Preserve typed edge kinds through query output.
- Include provenance/confidence where ambiguity or safety matters.
- Keep compatibility wrappers until parity scorecards pass.

## Semantic scorecard contract (readiness-facing)

Initial required rows:

- `definition_hit_at_1`, `definition_hit_at_5`
- `reference_precision`, `reference_recall`
- `completion_top1`, `completion_top5`
- `undefined_symbol_false_positive_rate`
- `rename_unsafe_edit_count`
- `safe_delete_external_ref_detection`
- `query_latency_p50`, `query_latency_p95`

This scorecard is fixture-backed first, then ratcheted after stability.

## Reviewer checklist for substrate PRs

A substrate PR is review-ready when it shows:

- one concern (schema, fixture bank, adapter, or query slice)
- deterministic tests/receipts
- explicit unsupported/dynamic boundary behavior
- no silent provider behavior cutover
- clear verification commands in PR body
