# Implementation Checklist: Complete FileSemanticBundle (Hash Last)

Issue: #1598 | Dep: #1600 (merged) | Related: #1599, #1601

## Ordered implementation steps

### Phase 1: Add producer_schema_version constant and field

**Step 1.1** — Add constant to `crates/perl-workspace/src/semantic/facts.rs`

File: `crates/perl-workspace/src/semantic/facts.rs`
Location: After line 25 (after the imports and before the module docstring or function definition)
Change: Add `pub const PRODUCER_SCHEMA_VERSION: u32 = 1;`
Verify: `cargo build -p perl-workspace` compiles

**Step 1.2** — Add field to `FileFactShard` struct

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: `FileFactShard` struct definition (lines 1139-1162)
Change: Add `pub producer_schema_version: u32,` as a new field after `content_hash: u64,` (after line 1145)
Dependencies: Declare the constant in facts.rs first (Step 1.1)
Verify: `cargo build -p perl-workspace` compiles

### Phase 2: Update FileFactShard construction sites

**Step 2.1** — Fix `build_canonical_fact_shard` return statement

File: `crates/perl-workspace/src/semantic/facts.rs`
Location: Return statement inside `build_canonical_fact_shard` (around lines 114-126)
Change: Populate `producer_schema_version: PRODUCER_SCHEMA_VERSION` in the `FileFactShard { .. }` struct literal
Dependencies: Constant added in Step 1.1, field added in Step 1.2
Verify: `cargo build -p perl-workspace` compiles

**Step 2.2** — Fix empty shard construction in `WorkspaceIndex::empty_shard`

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: `empty_shard()` method (around lines 2464-2483)
Change: Add `producer_schema_version: PRODUCER_SCHEMA_VERSION,` to the returned `FileFactShard { .. }` struct literal
Import: Add `use crate::semantic::facts::PRODUCER_SCHEMA_VERSION;` at the top of the workspace_index.rs file or impl block
Dependencies: Constant added in Step 1.1, field added in Step 1.2
Verify: `cargo build -p perl-workspace` compiles

### Phase 3: Extend build_canonical_fact_shard signature and implementation

**Step 3.1** — Extend function signature

File: `crates/perl-workspace/src/semantic/facts.rs`
Location: `build_canonical_fact_shard` function signature (line 52)
Change:
- Add two new trailing parameters:
  - `synthetic_entities: &[EntityFact],`
  - `synthetic_anchors: &[AnchorFact],`
Dependencies: Steps 1.1–2.2 must be complete
Verify: The function signature reads:
```rust
pub fn build_canonical_fact_shard(
    uri: &str,
    content_hash: u64,
    decl_facts: &SymbolDeclSemanticFacts,
    ref_facts: &SymbolRefSemanticFacts,
    imports: &[ImportSpec],
    dynamic_boundaries: &[OccurrenceFact],
    synthetic_entities: &[EntityFact],
    synthetic_anchors: &[AnchorFact],
) -> FileFactShard
```

**Step 3.2** — Merge synthetic facts into entity/anchor vecs BEFORE hash computation

File: `crates/perl-workspace/src/semantic/facts.rs`
Location: Inside `build_canonical_fact_shard`, after the main vec merges (around lines 92–106) and BEFORE hash computation (around line 108)
Change:
- After line 106 (`edges.extend_from_slice(...)`), add two new lines:
  ```rust
  entities.extend_from_slice(synthetic_entities);
  anchors.extend_from_slice(synthetic_anchors);
  ```
- This must occur BEFORE the hash computations on lines 109–112
Dependencies: Step 3.1 (signature extended)
Verify: The order is now:
  1. Extend entities from decl_facts.entities
  2. Extend anchors from decl_facts, ref_facts, imports, and dynamic_boundaries
  3. **NEW:** Extend entities from synthetic_entities
  4. **NEW:** Extend anchors from synthetic_anchors
  5. Compute hashes (now covering the complete set)

### Phase 4: Update call site and remove post-build push loop

**Step 4.1** — Build synthetic slices at call site

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: `build_canonical_fact_shard_for_ast` method (around line 2492–2556)
Change:
- After line 2521 (where `generated_member_facts` is extracted), add the synthetic slice building code:
  ```rust
  let synthetic_entities_from_eval: Vec<EntityFact> = eval_sub_triples
      .iter()
      .map(|(entity, _, _)| entity.clone())
      .collect();
  let synthetic_anchors_from_eval: Vec<AnchorFact> = eval_sub_triples
      .iter()
      .map(|(_, anchor, _)| anchor.clone())
      .collect();
  let synthetic_entities_from_generated: Vec<EntityFact> = generated_member_facts
      .iter()
      .map(|f| f.entity.clone())
      .collect();
  let synthetic_anchors_from_generated: Vec<AnchorFact> = generated_member_facts
      .iter()
      .map(|f| f.anchor.clone())
      .collect();
  let mut all_synthetic_entities = synthetic_entities_from_eval;
  all_synthetic_entities.extend(synthetic_entities_from_generated);
  let mut all_synthetic_anchors = synthetic_anchors_from_eval;
  all_synthetic_anchors.extend(synthetic_anchors_from_generated);
  ```
Dependencies: Steps 1.1–3.2 complete
Verify: The slices are built before the call to `build_canonical_fact_shard`

**Step 4.2** — Update call to build_canonical_fact_shard

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: Call site around lines 2526–2533
Change: Update the call to include the new synthetic slices:
```rust
let mut shard = crate::semantic::facts::build_canonical_fact_shard(
    uri,
    content_hash,
    &decl_facts,
    &ref_facts,
    &[],
    &dynamic_boundaries,
    &all_synthetic_entities,     // NEW
    &all_synthetic_anchors,      // NEW
);
```
Dependencies: Step 4.1 (slices built)
Verify: `cargo build -p perl-workspace` compiles

**Step 4.3** — Delete NOTE comment block and post-build push loops

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: Lines 2540–2553
Change: Delete the entire block:
```rust
// NOTE: This post-build merge means `entities_hash` and `anchors_hash` do
// not reflect these additions. Incremental replacement
// (`replace_fact_shard_incremental`) may miss a change if only synthetic
// facts change — the `content_hash` (whole-file) will still catch it.
// A future refactor should extend `build_canonical_fact_shard`'s API to
// accept extra entity/anchor slices alongside `dynamic_boundaries`.
for (entity, anchor, _) in eval_sub_triples {
    shard.entities.push(entity);
    shard.anchors.push(anchor);
}
for fact in generated_member_facts {
    shard.entities.push(fact.entity);
    shard.anchors.push(fact.anchor);
}
```
And replace the block with just `shard` (the function now returns immediately)
Dependencies: Steps 4.1–4.2 complete (all synthetic facts now flow through the function)
Verify: The function returns `shard` immediately after construction, with no post-build mutations
Verify: `cargo build -p perl-workspace` compiles

**Step 4.4** — Verify dynamic_boundaries unchanged

File: `crates/perl-workspace/src/workspace/workspace_index.rs`
Location: Line 2517 (inside `build_canonical_fact_shard_for_ast`)
Verify: The line extracting occurrences is unchanged:
```rust
let dynamic_boundaries: Vec<perl_semantic_facts::OccurrenceFact> =
    eval_sub_triples.iter().map(|(_, _, occ)| occ.clone()).collect();
```
This line remains unchanged — occurrences already flow correctly; only entities and anchors move to synthetic slices.
Verify: `cargo test -p perl-workspace` passes

### Phase 5: Test audit and format

**Step 5.1** — Audit hash-value assertions in tests

File: `crates/perl-workspace/tests/` (all test files)
Location: Any test that asserts specific `entities_hash`, `anchors_hash`, `occurrences_hash`, or `edges_hash` values
Change: Run tests to identify failures; for each failure, update the expected hash value to match the new computed hash
Rationale: The hash formula is now correct and complete. Tests must reflect the new baseline.
Dependencies: Step 4 complete (hash computation now includes synthetic facts)
Verify: All hash assertions pass with `cargo test -p perl-workspace`

**Step 5.2** — Format and lint

File: All files touched in this PR
Change: Run:
```bash
cargo xtask fmt
cargo clippy -p perl-workspace --lib
```
Dependencies: All code changes complete
Verify: No warnings or formatting issues

**Step 5.3** — Run full test suite

File: `crates/perl-workspace/`
Command: `cargo test -p perl-workspace`
Verify: All tests pass
Verify: No new clippy warnings

## Test map (acceptance grid to test names)

| Grid row | Named test | Responsibility | Location |
|---|---|---|---|
| 1 — synthetic-only entity change updates `entities_hash` | `entities_hash_covers_generated_members` | Red TDD → Green TDD | `crates/perl-workspace/tests/` (integration) |
| 2 — synthetic-only anchor change updates `anchors_hash` | `category_hash_covers_eval_facts` | Red TDD → Green TDD | `crates/perl-workspace/tests/` (integration) |
| 3 — post-build push loop removed | Diff inspection | Builder → Code review | (No test needed; deletion is the evidence) |
| 4 — `producer_schema_version` present on shard | `file_fact_shard_carries_producer_schema_version` | Red TDD → Green TDD | `crates/perl-workspace/tests/` (integration) |
| 5 — incremental replacement re-indexes on synthetic change | `replace_fact_shard_incremental_detects_synthetic_entity_change` | Red TDD → Green TDD | `crates/perl-workspace/tests/` (integration) |
| 6 — CI green | `cargo test -p perl-workspace` + `cargo clippy` + `cargo xtask fmt` | CI | CI logs |
| Hazard (arch-reviewer) — duplicate anchor guard fires on collision | `duplicate_anchor_guard_fires_on_collision` | Green TDD | `crates/perl-workspace/tests/` |

## Dependency and compilation order

1. ✓ #1600 must be merged first (provides file-scoped IDs; this PR builds on it)
2. Step 1: Add constant + field (Phases 1–2) — enables compilation of remaining steps
3. Step 2: Extend function signature (Phase 3.1) — unlocks Phase 3.2 and Phase 4
4. Step 3: Move synthetic facts to pre-hash (Phase 3.2 + Phase 4) — implements the core fix
5. Step 4: Delete post-build loops (Phase 4.3) — finalizes the design
6. Step 5: Audit tests, format (Phase 5) — stabilizes the implementation

Each phase compiles independently after its dependencies are met.

## Verification commands

After each step (or phase), run:
```bash
cargo build -p perl-workspace
```

After all implementation is complete:
```bash
cargo test -p perl-workspace
cargo clippy -p perl-workspace --lib
cargo xtask fmt
```

## Edge cases covered by acceptance tests

1. **File with ONLY generated-member facts** — `entities_hash` must be non-None and differ from a file with no generated members
2. **File with eval-sub AND generated-member facts** — both contribute to hash changes
3. **File with neither eval-sub nor generated members** — hash is computed over decl/ref facts only (baseline behavior unchanged)
4. **Incremental replacement with only synthetic change** — re-index is triggered (unlike before, when `content_hash` had to catch it)
5. **Lock ordering preservation** — imports/exports are NOT bundled; they remain separate (per oppositional-planner O1 resolution)

## Notes for builder

- **CAUTION**: The eval_sub_triples carry THREE elements: (entity, anchor, occurrence). Only extract idx 0 and 1 for synthetic slices. Idx 2 (occurrence) already flows through `dynamic_boundaries` — do NOT move it again.
- **No import/export bundling**: The plan-review resolved oppositional-planner O1 by NOT bundling imports/exports. FileSemanticBundle is a conceptual name; the only new surface is the field and the function parameters.
- **Scope narrow**: Touch ONLY `crates/perl-workspace/src/semantic/facts.rs` and `crates/perl-workspace/src/workspace/workspace_index.rs`. Do not touch `perl-symbol` or `perl-semantic-facts`.
- **Hash formula completeness**: After this PR, `entities_hash`/`anchors_hash` provably cover the complete shard for the first time. Tests asserting specific hash values will change baseline.
