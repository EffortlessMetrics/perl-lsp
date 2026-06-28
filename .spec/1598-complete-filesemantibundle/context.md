# Context: Complete FileSemanticBundle (Hash Last)

## Problem statement

`FileFactShard` category hashes (`entities_hash`, `anchors_hash`) do NOT cover all facts in the shard because synthetic facts (eval-sub boundaries + generated-member accessors) are merged AFTER hash computation.

**Current flow** (before this PR):
1. Build decl/ref facts from AST adapters
2. **Compute `entities_hash` / `anchors_hash`** (line 2540-2545 NOTE: without synthetic facts)
3. Push eval-sub entities/anchors onto shard (2546-2553)
4. Push generated-member entities/anchors onto shard (2550-2553)

**Result**: Incremental replacement (`replace_fact_shard_incremental`) must skip category-hash comparison and fall back to whole-file `content_hash`, defeating fine-grained change detection for synthetic-only changes.

**Root cause**: The post-build merge was a placeholder pending API extension. The NOTE at workspace_index.rs:2540-2545 documents it as debt to be removed.

## Decision: Scoped refactor (plan-review resolution of oppositional-planner objections)

### O1 resolution: No import/exports bundling

**Oppositional-planner objection**: Bundling imports/exports creates false lifecycle coupling.

**Decision**: `FileSemanticBundle` is a **conceptual name**, not a new public type. The change is narrower:
- Add `producer_schema_version: u32` field to `FileFactShard` (versioning metadata for #1601)
- Extend `build_canonical_fact_shard` signature to accept `synthetic_entities` and `synthetic_anchors` params
- Delete the post-build push loop
- Imports/exports remain separate (lock ordering at 1777-1782 unchanged)

**Why this works**: The fix targets the hash-completeness bug (synthetic facts merged pre-hash). Imports/exports have a separate lifecycle and are NOT part of the shard. Including them would risk the lock-ordering invariant.

### O2 resolution: producer_schema_version is concrete

**Oppositional-planner objection**: A field without a production value rots.

**Decision**: Add `pub const PRODUCER_SCHEMA_VERSION: u32 = 1;` in `facts.rs`. Non-optional field on `FileFactShard`.

**Why this works**: #1601 (snapshot layer) will read this field to validate that snapshot facts match producer schema. Future schema versions can bump the constant (e.g., add a new fact type) and snapshots can degrade gracefully. The field is now load-bearing for versioning negotiation.

### O3 resolution: API change cost is bounded

**Oppositional-planner objection**: Signature change affects hash formula.

**Decision**: Accept the API change. The hash formula MUST change because it's currently wrong. There is exactly one call site (`workspace_index.rs:2526`). Builder audits all call sites and updates test baselines.

**Why this works**: Hash formula change is intentional and documented. Tests are part of the change audit. Compilation enforces that all call sites are updated.

## Dependency chain

**Hard-dependency**: #1600 (file-scoped IDs) must merge first.
- Reason: This PR builds on file-scoped ID machinery for collision-free hashing.
- Status: ✓ Merged (commit 3d8e37f88 and earlier)

**Precedes**: #1601 (snapshot layer consumes complete bundles).
- Reason: Snapshots require hashes to be complete; this PR makes them so.
- Relationship: Sequential — #1601 will depend on this PR's branch.

## Architecture review feedback (resolved)

Architecture-reviewer signed off with one critical point for spec-planner:

> "The issue correctly identifies the file-scoped ID dependency from #1600 as load-bearing for collision-proofness, but `acceptance.md` §Hazards is missing one row: *'The fail-closed guard at workspace_index.rs:2659 (duplicate anchor detection) is unreachable in normal operation after file-scoped IDs ship, because distinct files now mint distinct IDs.'* This is a pre-build hazard row (ID/ref-space collision class). Before builder starts, route to spec-planner to add this assertion to `acceptance.md` so green-tdd has a test target."

**Resolution**: Added to acceptance.md §Hazards under COLLISION-1a:
```
Test: duplicate_anchor_guard_fires_on_collision — construct two shards with colliding AnchorIds, verify guard blocks both.
```

This test validates that the fail-closed guard at :2659 is reachable via direct API even after file-scoped IDs ship in normal operation (the guard is unreachable in forward operation but must still protect against synthetic collision scenarios).

## Plan-review synthesis

Plan-reviewer refined the spec by:
1. **Accepting O1** (no imports/exports) — trimmed false coupling
2. **Resolving O2** (producer_schema_version is concrete) — eliminated forward-looking debt
3. **Accepting O3** (hash formula change) — documented as intentional and test-driven
4. **Adding architecture-reviewer hazard** — to acceptance.md as collision guard test

**Result**: Scope is clear, API surface is minimal, test obligations are explicit.

## Alternatives considered and rejected

| Alternative | Rationale for rejection |
|---|---|
| **Lazy evaluation (A1 from oppositional-planner)** — Compute eval/generated facts on-demand from AST | Defers cost to query time; answers change when facts become stale; incompatible with incremental indexing and snapshot layer (#1601 needs stable facts). |
| **Separate metadata struct (A2)** — Keep FileFactShard unchanged, add SyntheticFactMetadata stored separately | Adds a separate index; complicates lifecycle management; does not fix the hash-completeness problem (two sources of truth about synthetic facts). |
| **Separate hashes without bundling (A3)** — Add synthetic_entities_hash and synthetic_anchors_hash fields, keep FileFactShard shape unchanged | Six hash comparisons in incremental replacement instead of four; minimal API change but leaves the shard incomplete (synthetic facts still post-hash in the struct). Does not solve the problem — just moves it sideways. |
| **Post-merge approach (not from oppositional-planner)** — Extend hashing to ALSO cover post-build facts without API change | Violates hash-computation order invariant (hashes should be computed on data they claim to cover); requires re-hashing during incremental replacement if post-build facts are mutated; does not make synthetic facts trustworthy. |

## Prior art and related work

- **#1599**: FileSemanticBundle invariant specification (defines what the bundle should contain and why)
- **#1600**: File-scoped IDs (merged; provides collision-free ID minting per file)
- **#1601**: Snapshot layer (will consume the complete bundle and validate producer schema version)
- **workspace_index.rs:2540-2545 NOTE**: Original debt specification; exactly what this PR addresses

## Test strategy notes for red-tdd

The red-tdd builder should write tests that:

1. **Verify hash-completeness**: Build two shards (one with synthetic facts, one without) and confirm hashes differ
2. **Verify synthetic-only change detection**: Use incremental replacement to show that synthetic-only changes trigger re-indexing
3. **Verify no double-counting**: Occurrences_hash remains stable (occurrences flow through dynamic_boundaries, not synthetic slices)
4. **Verify lock ordering**: Imports/exports remain separate from shard (no lock-ordering violations)
5. **Verify guard behavior**: Collision guard at :2659 fires when synthetic facts have colliding IDs (pre-#1600 scenario)

Tests should use the `perl_tdd_support::must_some` and `Result<()>` patterns per CLAUDE.md coding standards.

## Builder notes

1. **Scope narrow**: Only touch `semantic/facts.rs` and `workspace/workspace_index.rs`. Do NOT refactor perl-symbol or perl-semantic-facts.
2. **Triple extraction caution**: eval_sub_triples has (entity, anchor, occurrence). Extract idx 0+1 for synthetic slices. Idx 2 flows through dynamic_boundaries unchanged.
3. **Hash audit**: After implementation, run tests and update any hard-coded hash values in assertions. Document why they changed (e.g., "now includes synthetic entity from eval sub").
4. **Compilation-driven**: Let the compiler guide you. Extend signature → compilation fails at call sites → fix those sites → success.
5. **Lock ordering**: Verify import/export write-lock block (1777-1782) is untouched and still executes after the main shard write-lock (1738-1769).

## Glossary

- **FileSemanticBundle**: Conceptual aggregation of all semantic facts for one file (anchors, entities, occurrences, edges, synthetic facts, hashes, metadata). Not a new type; a design invariant this PR realizes.
- **Synthetic facts**: Entity/anchor facts produced by eval-sub and generated-member extractors (not from AST adapters directly).
- **Category hashes**: Per-category hashes (entities_hash, anchors_hash, occurrences_hash, edges_hash) compared in incremental replacement.
- **Producer schema version**: Version constant indicating which semantic fact schema the producer is using (for #1601 snapshot validation).
- **Dynamic boundaries**: OccurrenceFact entries for eval-sub evidence and symbolic derefs (already merged pre-hash; unchanged by this PR).

## Links

- **Issue #1598**: https://github.com/Perl/perl-lsp/issues/1598
- **Issue #1599**: FileSemanticBundle invariant specification (related)
- **Issue #1600**: File-scoped IDs (hard dependency, merged)
- **Issue #1601**: Snapshot layer (successor, depends on this PR)
- **File**: `crates/perl-workspace/src/semantic/facts.rs` (core change location)
- **File**: `crates/perl-workspace/src/workspace/workspace_index.rs` (integration point)
- **Hazard defaults**: `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md` (perl-workspace hazards seeded from here)
- **Contract index**: `docs/reference/PARSER_CONTRACTS.md` (fact-shard invariants)
