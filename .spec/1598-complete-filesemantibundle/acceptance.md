# Acceptance Criteria: Complete FileSemanticBundle (Hash Last)

Issue: #1598 | Predecessor: #1600 (merged) | Successor: #1601

## §Behavior

Each row documents an input condition and the expected observable result after the change.

| # | Condition | Input | Expected Result | Evidence |
|---|-----------|-------|-----------------|----------|
| B1 | File with generated-member facts only (no eval subs) | AST with Moo `has` declarations | `entities_hash` changes when generated member is added/removed | New test `entities_hash_covers_generated_members` passes |
| B2 | File with eval-sub facts only (no generated members) | AST with `eval "sub NAME { ... }"` | `anchors_hash` and `entities_hash` both change when eval sub is added/removed | New test `category_hash_covers_eval_facts` passes |
| B3 | File with both eval-sub and generated-member facts | AST with both eval and has declarations | `entities_hash` and `anchors_hash` both change when either fact is added/removed | Both B1 and B2 tests pass together |
| B4 | File re-indexed with synthetic-only change | Same AST content except eval sub name changes | `replace_fact_shard_incremental` re-indexes entities/anchors (not skipped) | New test `replace_fact_shard_incremental_detects_synthetic_entity_change` passes |
| B5 | File with neither eval-sub nor generated-member facts | Baseline AST with only decl/ref facts | Hash computation works correctly (same behavior as before) | Existing tests pass; no regression |
| B6 | Producer schema version query | Any file | `FileFactShard.producer_schema_version` returns `1` (PRODUCER_SCHEMA_VERSION constant) | New test `file_fact_shard_carries_producer_schema_version` passes |

## §Hazards

All hazard classes enumerated per SUBSYSTEM_HAZARD_DEFAULTS.md (perl-workspace touches semantic/facts.rs and workspace_index.rs, triggering no DAP/Parser/LSP rows; COV/CI gates apply post-merge).

### Hazard Class 1: ID/Reference-Space Collision

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **COLLISION-1a**: Duplicate anchor IDs silently masked | `workspace_index.rs:2659` (`semantic_anchor_wire_location` fail-closed guard) | After file-scoped IDs (#1600) ship, the guard is unreachable in normal operation (each file mints distinct IDs). But the guard itself is the correctness boundary — if ever called, must fail closed. Pre-#1600 scenario (synthetic fact produced with identical AnchorId as another file) must still be detected. | The guard at :2659 is unreachable in forward operation (file-scoped IDs prevent collisions). But to validate the guard itself, construct a pre-collision scenario: two FileFactShards with identical `AnchorId` values (simulating pre-#1600), pass to workspace index, verify `semantic_anchor_wire_location` returns None (fail-closed). | **Test**: `duplicate_anchor_guard_fires_on_collision` — construct two shards with colliding AnchorIds, verify guard blocks both. Location: `crates/perl-workspace/tests/` |

### Hazard Class 2: Hashing Completeness and Incremental Invalidation

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **HASH-2a**: Entities/anchors hash now covers synthetic facts | `facts.rs:108-112` (hash computation) + `workspace_index.rs:2544-2553` (removed post-build push) | **Before**: synthetic facts pushed AFTER hash computation → `entities_hash`/`anchors_hash` incomplete. Incremental replacement missed synthetic-only changes (content_hash caught them, but category hashes lied). **After**: synthetic facts merged BEFORE hash computation → `entities_hash`/`anchors_hash` complete and trustworthy. This is the core fix: hash formula changes and must be validated. | (1) Extend `build_canonical_fact_shard` signature to accept synthetic slices. (2) Merge synthetic slices into entity/anchor vecs BEFORE hash computation (facts.rs:107-108). (3) Delete the post-build push loop (workspace_index.rs:2546-2553) — the NOTE at :2540-2545 is the specification of what to fix. | **Test 1**: `entities_hash_covers_generated_members` — file with ONLY generated-member facts must have non-None `entities_hash` distinct from a file with zero generated members. **Test 2**: `category_hash_covers_eval_facts` — file with ONLY eval-sub facts must have `entities_hash`/`anchors_hash` distinct from a file without eval subs. **Test 3**: `replace_fact_shard_incremental_detects_synthetic_entity_change` — verify incremental replacement re-indexes when only an eval-sub entity changes (not skipped). Location: `crates/perl-workspace/tests/` |
| **HASH-2b**: Triple extraction must not double-count occurrences | `workspace_index.rs:2514-2553` (eval_sub_triples processing) | `eval_sub_triples` is a `Vec<(EntityFact, AnchorFact, OccurrenceFact)>`. (1) The occurrence (idx 2) already flows to `dynamic_boundaries` (line 2517). (2) Synthetic slices must extract ONLY the entity (idx 0) and anchor (idx 1). (3) If occurrences are moved to synthetic slices, they would be merged twice: once as `dynamic_boundaries` and again as synthetic occurrences → double-count. **Risk**: Silent corruption of occurrences_hash and fact shard data. | Extract only idx 0 (entity) and idx 1 (anchor) from eval_sub_triples. Leave idx 2 (occurrence) flowing through `dynamic_boundaries` unchanged. Verify line 2517 is untouched after refactor. | **Test**: `category_hash_covers_eval_facts` validates that occurrences_hash is correct (does not double-count). Inspect diff at workspace_index.rs:2517 — must be unchanged. Location: code review + CI diff |

### Hazard Class 3: Synthetic Fact Producer Assumptions

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **SYN-3a**: Generated member facts produce both entity and anchor | `generated_member_extractor.rs` (`GeneratedMemberFact` struct) | Generated members are emitted as (entity, anchor) pairs. The entity carries `kind = EntityKind::GeneratedMember`. The anchor pinpoints the source `has` declaration. If only one is extracted, the shard is incomplete. **Risk**: Partial facts corrupt incremental indexing. | Verify `GeneratedMemberFact` carries both `.entity` and `.anchor`. Merge both into synthetic slices (Step 4.1 of checklist). | **Test**: `entities_hash_covers_generated_members` ensures both entity and anchor are included. Location: code review + integration test |
| **SYN-3b**: Eval-sub triples produce entity, anchor, and occurrence | `eval_sub_extractor.rs` (function return type `Vec<(EntityFact, AnchorFact, OccurrenceFact)>`) | Eval-sub triples carry all three fact types. The entity is the inferred sub name. The anchor is the evidence location in the eval string. The occurrence is the dynamic boundary proof. All three are needed for correctness. **Risk**: Missing entity/anchor/occurrence corrupts semantic queries. | (1) Verify all three are extracted. (2) Route entity/anchor to synthetic slices. (3) Route occurrence through `dynamic_boundaries` (unchanged). (4) Verify no fact is dropped. | **Test**: `category_hash_covers_eval_facts` ensures entity and anchor are included; occurrences_hash unchanged validates occurrence flow. Location: integration test |

### Hazard Class 4: Lock Ordering and Separate Indexing Paths

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **LOCK-4a**: Imports/exports must NOT be bundled with shard | `workspace_index.rs:1777-1782` (separate write-lock block for imports) + oppositional-planner O1 resolution | Imports and exports are published in a separate write-lock block (lines 1777-1782), after the main shard write lock (lines 1738-1769). They are NOT part of the category-hash computation. Their invalidation follows a different code path (`ie_idx.remove_file_imports()` vs `replace_fact_shard_incremental()`). **Risk**: Bundling imports into `FileSemanticBundle` conflates two independent versioning lifecycles. If the builder adds imports/exports fields to the struct, they risk violating lock ordering or creating false versioning coupling. | **Design decision (from plan-review)**: `FileSemanticBundle` is a conceptual name. The ONLY new API surface is: (1) `producer_schema_version: u32` field on `FileFactShard` (2) two new `&[EntityFact]`/`&[AnchorFact]` parameters on `build_canonical_fact_shard`. NO imports/exports bundling. | **Test/Verify**: (1) Code review: imports/exports fields MUST NOT appear on `FileFactShard`. (2) Line 1777-1782 (separate import write lock) MUST be unchanged. (3) `ImportExportIndex` remain separate (not embedded in `FileFactShard`). Location: diff audit |

### Hazard Class 5: Function Signature and Call-Site Breakage

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **SIG-5a**: `build_canonical_fact_shard` API change requires audit of all callers | `facts.rs:52` (function signature) + `workspace_index.rs:2526` (call site) | The function signature is changing from 6 params to 8 params (adding `synthetic_entities` and `synthetic_anchors`). There is exactly one call site (`workspace_index.rs:2526`) outside of tests. **Risk**: If builder misses a call site, compilation fails. If there are hidden call sites (e.g., in feature-gated code), they break silently in CI. | (1) Verify `pub(crate)` scope (not public to other crates). (2) Grep for all call sites: `grep -n "build_canonical_fact_shard" crates/perl-workspace/src/**/*.rs`. (3) Update all call sites (should be 1 production + test sites). (4) Run `cargo build -p perl-workspace` after signature extension to catch missing updates. | **Test/Verify**: (1) `cargo build -p perl-workspace` compiles after signature extension. (2) No `error\[E0061\]: this function takes X arguments but Y were supplied` in CI. Location: CI build logs |

### Hazard Class 6: Test Suite Regression and Hash-Value Assertions

| Hazard | Surface | Risk | Mitigation | Test obligation |
|--------|---------|------|-----------|-----------------|
| **TEST-6a**: Hash values in assertions change; tests must be updated | `crates/perl-workspace/tests/**/*.rs` (any test asserting `entities_hash` / `anchors_hash` / `occurrences_hash` / `edges_hash`) | The hash computation formula is changing: synthetic facts are now included pre-hash instead of post-hash. **Before**: A shard with eval subs had `entities_hash` that did NOT include the eval-sub entity. **After**: `entities_hash` includes the eval-sub entity. Any test hard-coding expected hash values will fail. **Risk**: Builder sees failing tests, doesn't understand why, either skips the test or commits wrong hash values. | (1) Run `cargo test -p perl-workspace` after implementation. (2) Identify all test failures related to hash mismatches. (3) For each failure, understand what fact was added/changed and update the expected value. (4) Document in test comments why the hash changed (e.g., "includes synthetic entity from eval sub"). (5) Verify new expected value is stable (recompile and test again). | **Test**: `cargo test -p perl-workspace` — all tests pass. Any test asserting hashes must have updated baseline values with comments explaining the change. Location: test output + code review |

## §Contracts

Contracts from PARSER_CONTRACTS.md and LSP/DAP protocol specs touched by this change.

| Contract | Source | Surface | Impact |
|----------|--------|---------|--------|
| **FileFactShard structure invariant** | `workspace_index.rs:1139-1162` (type definition) | New field: `producer_schema_version: u32` | Shard now carries producer schema version for future incremental upgrade scenarios (#1601). Backward-compat: field is non-optional (always set to 1 initially). Forward-compat: snapshot layer (#1601) can inspect this field to validate fact versions. |
| **Category hash completeness** | `facts.rs:108-112` (hash computation) | Hashes now computed AFTER synthetic fact merging | `entities_hash`, `anchors_hash` now provably cover all facts (decl, ref, synthetic). Incremental replacement can trust these hashes instead of relying on whole-file `content_hash` as a proxy. |
| **Incremental replacement invariant** | `workspace_index.rs:2558-2610` (`replace_fact_shard_incremental`) | No change to this function's logic; hashes are now trustworthy | Category hashes can now be the sole criterion for incremental re-indexing. (Content_hash still used for early-exit optimization, but not as the correctness fallback.) |
| **Synthetic fact producer boundary** | `semantic/eval_sub_extractor.rs` + `semantic/generated_member_extractor.rs` | Synthetic facts now flow through the canonical build path (not post-hoc push) | Produces now have a contract: return facts that will be merged into the shard BEFORE hashing. Refactoring these producers does not risk hash invalidation. |
| **Lock ordering at import/export boundary** | `workspace_index.rs:1731-1782` | Unchanged — imports remain separate from shard | Import/export publishing is decoupled from shard building. This change does not touch that path. |

## §API-Shape

New public surface and potential duplication/dup-risk.

| Item | Type | Surface | Rationale | Dup risk | Caller count (production) |
|------|------|---------|-----------|----------|--------------------------|
| `FileFactShard::producer_schema_version` | Field (`u32`) | `crates/perl-workspace/src/workspace/workspace_index.rs:1139-1162` | Carries producer schema version for snapshots (#1601) and future fact-version negotiation. Non-optional, defaults to `1`. | Low — field name is clear; no alias needed. | 1 (snapshot layer #1601 reads this) |
| `build_canonical_fact_shard::synthetic_entities` | Param (`&[EntityFact]`) | `crates/perl-workspace/src/semantic/facts.rs:52` | New trailing param for accepting synthetic entity facts before hashing. | Low — name is descriptive; only used at 1 call site. | 1 (workspace_index.rs:2526) |
| `build_canonical_fact_shard::synthetic_anchors` | Param (`&[AnchorFact]`) | `crates/perl-workspace/src/semantic/facts.rs:52` | New trailing param for accepting synthetic anchor facts before hashing. | Low — name is descriptive; only used at 1 call site. | 1 (workspace_index.rs:2526) |
| `PRODUCER_SCHEMA_VERSION` | Constant (`u32 = 1`) | `crates/perl-workspace/src/semantic/facts.rs` | Version constant for producer schema. Settable in future as producer versioning matures. | Low — constant name is clear; no conflicts expected. | 2 (facts.rs return + workspace_index.rs empty_shard) |

**Dup-risk grep checks**:
- `grep -n "producer_schema_version" crates/perl-workspace/src/**/*.rs` — should find only `FileFactShard` definition + `facts.rs` constant definition + 2 usages (facts.rs:2.1, workspace_index.rs:2.2)
- `grep -n "synthetic_entities\|synthetic_anchors" crates/perl-workspace/src/**/*.rs` — should find only function signature + call site

## §Test-Grid

Exhaustive test plan covering positive, negative, adversarial, and state-transition scenarios.

### Positive tests (expected to pass after change)

| Test name | Input | Setup | Action | Expected result | Evidence/Assertion |
|-----------|-------|-------|--------|-----------------|-------------------|
| `entities_hash_covers_generated_members` | File with Moo `has` declarations | Build two shards: (1) file with 0 generated members, (2) file with 1+ generated members | Compare `entities_hash` between the two | Hashes differ (the hash includes the synthetic entity) | `assert_ne!(shard1.entities_hash, shard2.entities_hash)` |
| `category_hash_covers_eval_facts` | File with `eval "sub Foo { ... }"` | Build two shards: (1) file with 0 eval subs, (2) file with 1+ eval subs | Compare `entities_hash` and `anchors_hash` | Both hashes differ (both include synthetic facts) | `assert_ne!(shard1.entities_hash, shard2.entities_hash)` && `assert_ne!(shard1.anchors_hash, shard2.anchors_hash)` |
| `file_fact_shard_carries_producer_schema_version` | Any file | Build a shard via `build_canonical_fact_shard` | Inspect `shard.producer_schema_version` | Equals `1` (PRODUCER_SCHEMA_VERSION constant) | `assert_eq!(shard.producer_schema_version, 1)` |
| `replace_fact_shard_incremental_detects_synthetic_entity_change` | File with eval sub, re-indexed with same eval sub name but different body | Index file, store shard. Re-parse file (eval sub name same, AST different). Call `replace_fact_shard_incremental` with new shard | Check if `ReplaceResult` indicates re-indexing (not skipped) | Re-indexing triggered (categories re-indexed, not skipped) | `assert!(replace_result.re_indexed_categories.contains(&"entities"))` |
| `duplicate_anchor_guard_fires_on_collision` | Two FileFactShards with identical AnchorId (synthetic pre-#1600 collision) | Construct shard1 with AnchorId(123), shard2 with AnchorId(123) | Call `semantic_anchor_wire_location` on both | Fails closed (returns None) | `assert_eq!(index.semantic_anchor_wire_location(...), None)` |

### Negative tests (expected to fail or reject input)

| Test name | Input | Setup | Action | Expected result | Evidence/Assertion |
|-----------|-------|-------|--------|-----------------|-------------------|
| `synthetic_facts_not_added_twice` | File with eval sub | Build shard via new `build_canonical_fact_shard` with synthetic slices | Verify entity/anchor count | Entities/anchors appear once (not duplicated by post-build push) | `assert_eq!(shard.entities.len(), expected_count)` — count matches synthetic slice size + decl/ref facts, no doubling |

### Adversarial tests (boundary conditions, stress)

| Test name | Input | Setup | Action | Expected result | Evidence/Assertion |
|-----------|-------|-------|--------|-----------------|-------------------|
| `empty_file_synthetic_slices` | Empty file (no decl, ref, synthetic) | Build shard with empty synthetic slices | Compute hashes | Hashes are stable and non-None | `assert!(shard.entities_hash.is_some())` — hashes even on empty shard |
| `large_synthetic_fact_count` | File with 1000+ eval subs or generated members | Build shard with large synthetic slices | Hash computation time / shard memory | Completes in reasonable time (<100ms), memory-efficient | `assert!(elapsed < Duration::from_millis(100))` |
| `synthetic_fact_order_invariant` | Same file, parsed twice, synthetic facts extracted in different order (simulated) | Build two shards with synthetic slices in different order | Compare hashes | Hashes are identical (deterministic) | `assert_eq!(shard1.entities_hash, shard2.entities_hash)` — order-invariant hash |

### State-transition tests (incremental replacement scenarios)

| Test name | Input | Setup | Action | Expected result | Evidence/Assertion |
|-----------|-------|-------|--------|-----------------|-------------------|
| `incremental_synthetic_only_change` | Same file, parsed twice: eval sub added | Index file (v1, no eval subs). Parse v2 with 1 eval sub. | Call `replace_fact_shard_incremental(v1, v2)` | Only entities/anchors categories re-indexed (occurrences/edges skipped) | `assert!(result.re_indexed_categories.contains(&"entities"))` && `!result.re_indexed_categories.contains(&"occurrences")` |
| `incremental_no_synthetic_change` | Same file, parsed twice: eval sub untouched, other AST changes | Index file (v1, with eval sub X). Parse v2 with same eval sub X but other code changed. | Call `replace_fact_shard_incremental(v1, v2)` | Content_hash differs, full re-index triggered (or per-category hashes show no synthetic change) | If content_hash changed but no synthetic facts changed, test documents current behavior (content_hash triggers re-index) |
| `incremental_synthetic_and_ref_change` | Same file, parsed twice: eval sub changed AND reference added | Index file (v1, eval sub X, no refs to Y). Parse v2 with eval sub Y, ref to Y. | Call `replace_fact_shard_incremental(v1, v2)` | Both entities and occurrences categories re-indexed | `assert!(result.re_indexed_categories.contains(&"entities"))` && `assert!(result.re_indexed_categories.contains(&"occurrences"))` |

## §Blast-Radius

Changes that touch boundaries and downstream consumers.

| Boundary | Status | Surface | Risk | Mitigation |
|-----------|--------|---------|------|-----------|
| **crates/perl-workspace/** | Changed | `semantic/facts.rs` (function signature + constant) + `workspace/workspace_index.rs` (struct field + call site) | Function signature change requires all callers to update. Field addition requires all construction sites to populate the field. | (1) Single call site (workspace_index.rs:2526). (2) Two construction sites (facts.rs return + empty_shard). (3) Grep verified; all sites updated in checklist. (4) Compilation catches any missed sites. |
| **crates/perl-symbol/** | Untouched | No changes to symbol producers | Low — producers output `EntityFact`/`AnchorFact` unchanged; only consumption path changes | N/A |
| **crates/perl-semantic-facts/** | Untouched | Leaf types (EntityFact, AnchorFact, etc.) unchanged | Low — fact shapes unchanged; only production timing changes | N/A |
| **crates/perl-lsp-rs/** | Affected (downstream) | Queries via `SemanticQueries` that read shard facts | **Impact**: Positive — hashes are now correct and trustworthy. Queries see the same facts but with fixed hashes. No behavior change. | Builder to verify queries still work by running `cargo test -p perl-workspace` (integration tests use `SemanticQueries`). Snapshot layer (#1601) inherits the benefit. |
| **#1601 (snapshot layer)** | Hard dependency | Snapshot consumes `FileSemanticBundle` (the refactored FileFactShard) | The snapshot expects the shard to be complete before hashing. This change enables that invariant. | #1601 can now trust `producer_schema_version` and `entities_hash`/`anchors_hash` as complete. |
| **Lock ordering (import/export)** | Untouched | `workspace_index.rs:1777-1782` (separate write lock) | Low — change does not touch the import/export path. Separate lock ordering remains intact. | Code review verifies no changes to lines 1731-1782. |
| **Test suite** | Modified | Hash-value assertions in `crates/perl-workspace/tests/` | Test failures expected where hard-coded hash values were asserted. Hash formulas are now correct. | Builder audits failures and updates expected values (documented in checklist Step 5.1). |

## Summary

This change fixes the hash-completeness bug documented in the NOTE at `workspace_index.rs:2540-2545` by extending the canonical fact-building API to accept synthetic entity/anchor slices and merging them BEFORE hash computation. The post-build push loops are deleted. The `producer_schema_version` field is added to enable future snapshot versioning (#1601). No imports/exports bundling; lock ordering unchanged. Incremental replacement now has trustworthy category hashes. All synthetic-fact producers (eval-sub, generated-member) integrate cleanly through the new signature.
