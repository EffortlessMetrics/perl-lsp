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

## Wave 2 (After First-Wave Rails Land)

1. `SymbolDecl -> EntityFact` adapter.
2. `SymbolRef -> OccurrenceFact` adapter.
3. `ExportInfo -> ExportSet` adapter.
4. `FileFactShard` write-through in workspace store.
5. Definition-candidate multimap behind compatibility APIs.
6. Typed reference-edge global index behind compatibility APIs.

## Wave 2 Implementation Status

This section is historical migration context. Current compiler-substrate state
is tracked in [COMPILER_CAPABILITY_STATUS.md](COMPILER_CAPABILITY_STATUS.md)
and [compiler_facts.md](status/compiler_facts.md). Current semantic proof rows
are tracked in [semantic_scorecard.md](status/semantic_scorecard.md) and
[semantic_shadow_compare.md](status/semantic_shadow_compare.md).

This section is the migration receipt for what has landed versus what remains staged.

### Landed

- **Neutral fact vocabulary:** `AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`, and `DiagnosticFact` exist in `perl-semantic-facts` with deterministic serde/roundtrip coverage. (PR #7314)
- **`SymbolDecl -> EntityFact` adapter:** `perl-symbol` now emits `EntityFact` and `EdgeFact` rows from `SymbolDecl` with `Defines` edges and provenance. (PR #7341)
- **Fact shard write-through:** `FileFactShard` struct and write-through storage in `WorkspaceIndex` are landed; workspace populates shards on index. Legacy symbol/reference indexes remain the source of truth for providers. (PR #7357)
- **Definition candidate multimap:** `DefinitionCandidate` multimap behind compatibility APIs is landed with deterministic sort and incremental removal. (PR #7360)
- **Shadow-compare receipt:** design/test rail is landed (`semantic_shadow_compare.rs`); no provider cutover or production shadow-read gating is enabled. (PR #7366)
- **Scorecard v1:** fixture harness and semantic scorecard are landed. Current
  fact counts and readiness rows are generated in
  [semantic_scorecard.md](status/semantic_scorecard.md). (PR #7367)
- **`SymbolRef -> OccurrenceFact` adapter:** landed in `perl-symbol`; phase-1 `SymbolRef` forms can emit canonical `OccurrenceFact`/`AnchorFact` with neutral unresolved handling. (PR #7444)
- **`ExportInfo -> ExportSet` adapter:** landed in `perl-semantic-analyzer` with deterministic adapter tests.

### Still staged or superseded by compiler-substrate tracking

- **Typed reference-edge global index:** not landed; typed-reference behavior is constrained to fixture/regression banks rather than a provider-facing global index.
- **Canonical producer wiring into `FileFactShard`:** partial; declarations are present but workspace-wide shard population from all canonical producers is not yet complete.
- **Compiler-path import/export ownership:** semantic `ImportSpec`, `ExportSet`,
  and `visible_symbols_at` proof exists, but [#8244](https://github.com/EffortlessMetrics/perl-lsp/issues/8244)
  tracks canonical compiler-substrate ownership.
- **Provider cutover:** tracked separately in
  [provider_cutover.md](status/provider_cutover.md) and [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197).

### Explicit non-goals for current Wave 2 state

- No provider cutover yet.
- No rename/safe-delete cutover yet.
- No full type inference.

## Wave 2 Execution Discipline (Curation + Test Floor)

When duplicate Wave 2 candidates exist, run a curator pass before merging additional tracks.

### Required Curator Output Shape

```json
{
  "cluster": "typed_reference_index",
  "keepers": [7348],
  "close_as_superseded": [7362, 7363],
  "port_from_losers": [
    {
      "from": 7363,
      "detail": "count_usages test shape",
      "to": 7348
    }
  ],
  "review_notes": [
    "verify ReferenceEdge shape lives in perl-semantic-facts"
  ]
}
```

### Parity Test Floor

Before merging the workspace-facing boxes (4–7), fix and re-green the parity test floor:

```bash
cargo test -p perl-workspace surface_workspace_parity
cargo test -p perl-workspace
cargo check --workspace --all-targets
```

Interpret failures explicitly as one of: real regression, stale expected output, fixture drift, or documented legacy divergence.

### Merge Train Discipline

1. Merge boxes 1–3 (exact adapters) and verify workspace-wide checks.
2. Merge one curated winner for each duplicate cluster (facts shard, typed refs, shadow receipts, scorecard rows).
3. Re-check `perl-workspace` parity tests after each merge-train step.
4. Close superseded PRs only after loser-test harvest is ported.

## Wave 3 (User-Visible Cutover Staging)

The Wave 3 fact and query rail has moved from speculative plan to fixture-backed
semantic proof:

1. `ImportSpec` extraction exists.
2. `ExportSet` facts exist.
3. `visible_symbols_at(...)` exists and is scorecarded.
4. Semantic shadow compare tracks definition/reference regressions.

The remaining Wave 3 work is provider cutover discipline: fact-source tracing,
per-provider shadow proof, real-workspace coverage, and fallback behavior before
normal LSP providers depend on new compiler facts.

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
