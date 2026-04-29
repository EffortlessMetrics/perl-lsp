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

## Wave 2 (Current Implementation Status)

Wave 2 implementation has started. The substrate now includes the following rails and adapter/index
behaviors:

### Exact facts emitted now

- `EntityFact` is emitted from `SymbolDecl` adapter paths.
- `OccurrenceFact` is emitted from `SymbolRef` adapter paths.
- `ExportSet` is emitted from `ExportInfo` adapter paths.

### Workspace storage / index status

- `FileFactShard` is **write-through enabled** in workspace storage.
- Definition candidate multimap is present as a **compatibility-layer backing index**.
- Typed reference-edge global index is present as a **compatibility-layer backing index**.

### Shadow compare + scorecard status

- Shadow-compare receipts are available as an implementation receipt surface for parity tracking.
- Semantic scorecard v1 harness is integrated; metrics remain intentionally staged while provider cutover is deferred.

### Explicit non-goals (still true in Wave 2)

- No provider cutover yet.
- No rename/safe-delete cutover yet.
- No full type inference.

## Wave 3 (User-Visible Cutover Staging)

Wave 3 remains focused on landing the `ImportSpec` + `visible_symbols_at` bridge that unblocks
provider-facing behavior:

1. `ImportSpec` extraction.
2. `visible_symbols_at` query implementation (`VisibleSymbols` substrate query).
3. Completion consumes `VisibleSymbols` behind a feature flag.
4. Undefined diagnostics consume `VisibleSymbols` behind a feature flag.

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
