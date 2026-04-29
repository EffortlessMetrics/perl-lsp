# Semantic Fact Substrate (Compiler-lite) — v1 Contract

## Purpose

This document defines the **neutral semantic substrate** that will improve workspace-wide awareness and analysis without immediate provider cutover.

The goal of this first wave is to prepare rails:

1. Canonical fact vocabulary.
2. Fixture and scorecard proof surfaces.
3. Adapter + migration contracts.

This document is intentionally architecture-first: it does **not** claim provider migration is complete.

## Layer model

```text
Perl source files
  -> perl-symbol (exact AST facts)
  -> perl-semantic-analyzer (scoped/framework/synthetic facts)
  -> perl-workspace-index (storage/index/invalidation/query)
  -> LSP providers (definitions/references/completion/rename/safe-delete)
```

## Crate responsibilities

- `perl-symbol`
  - Extract exact, syntax-proximal symbols and references.
  - Preserve lexical anchors and source ranges.
- `perl-semantic-analyzer`
  - Add scope-aware and framework-aware synthesis (e.g. generated members).
  - Mark dynamic boundaries conservatively.
- `perl-workspace-index`
  - Own file-shard storage, indexing, invalidation, and cross-file query execution.
  - Provide stable query APIs consumed by providers.
- `perl-lsp-*` providers
  - Consume query APIs only.
  - Avoid direct traversal of private semantic maps over time.

## Fact model summary (target vocabulary)

Canonical facts are split by role:

- **Identity**: typed IDs (`FileId`, `EntityId`, `OccurrenceId`, `AnchorId`, `EdgeId`, etc.).
- **Anchor facts**: source anchoring + URI/range identity.
- **Entity facts**: packages, classes, roles, methods, subs, fields, generated members.
- **Occurrence facts**: definition/reference/read/write/call/import/export and dynamic boundaries.
- **Edge facts**: typed relations (`Defines`, `Calls`, `ImportsSymbol`, `Inherits`, `ComposesRole`, etc.).
- **Diagnostic facts**: semantic warnings with provenance/confidence.

## Provenance and confidence policy

Each fact must carry:

- **Provenance**: where the fact came from (`ExactAst`, `SemanticAnalyzer`, `FrameworkSynthesis`, `ImportExportInference`, `DynamicBoundary`, etc.).
- **Confidence**: conservative certainty tier (`High`, `Medium`, `Low`).

Policy:

- Prefer exact and local evidence over heuristics.
- Do not silently upcast heuristic facts to high confidence.
- Preserve ambiguous candidates rather than selecting a lossy single winner.

## Dynamic-boundary policy

Treat constructs such as `eval STRING`, dynamic `require`, typeglob aliasing, and `AUTOLOAD` as explicit boundaries.

- Emit boundary facts/edges rather than overconfident resolution.
- Keep boundaries visible to definition/reference/rename safety planning.
- Allow later tooling to distinguish “unknown by design” from “missing implementation.”

## Migration order (wave plan)

```text
facts schema
-> exact adapters
-> workspace store
-> query APIs
-> provider migration
```

Practical wave sequencing:

1. Fact schema and crate contract.
2. Fixture banks + scorecard harness.
3. Adapters (`SymbolDecl`, `SymbolRef`, export metadata).
4. Workspace write-through and compatibility query layer.
5. Provider opt-in behind feature flags / shadow-compare receipts.

## Query API contract (provider-facing)

Planned query surface:

```rust
symbol_at(uri, pos)
definitions(occurrence, policy)
references(entity, scope, filter)
visible_symbols_at(uri, pos, context)
method_candidates(receiver, prefix)
rename_plan(entity, new_name)
safe_delete_plan(entity)
```

Contract expectations:

- Deterministic ordering for equal-score candidates.
- Explicit ambiguity payloads instead of implicit tie-breaking.
- Provenance/confidence available for diagnostics and UX ranking.
- Bounded-latency operation suitable for interactive editor use.

## Scope for v1 vs deferred

### In scope (v1 substrate)

- Canonical fact vocabulary + typed IDs.
- Adapter seams and fixture-backed expectations.
- Workspace query contracts and compatibility behavior.
- Shadow-compare path to evaluate old/new answers.

### Deferred (post-v1 substrate)

- Full provider cutover in a single PR.
- Aggressive heuristic expansion before scorecard baselines are stable.
- Removing legacy maps before parity/ambiguity receipts exist.

## Reviewability principle

For each implementation PR in this epic:

> Produce the most reviewable and tested complete slice of the specific change. Do not optimize for a tiny diff; optimize for rapid proof of correctness.
