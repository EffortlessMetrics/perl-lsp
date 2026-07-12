# PR 1711-A -- didChange re-extraction work-shape measurement receipt

**Controlling issue:** [#1711](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1711)
(LSP-freshness reliability lane). **Related:** #3396.

This is a **measurement-only** receipt. No extraction or propagation behavior
changed. The instrumentation that produced these numbers
(`crates/perl-workspace/src/workspace/workspace_index.rs`, `reindex_metrics`
module) is compiled ONLY under `#[cfg(test)]` -- verified by `cargo check -p
perl-workspace` and `cargo check -p perl-lsp-rs` (no `--tests`) producing an
identical build before and after this change.

**Source of truth for the structural counts below:** the checked-in `insta`
snapshot
`crates/perl-workspace/src/workspace/snapshots/perl_workspace__workspace__workspace_index__reindex_workshape_measurement__reindex_workshape_receipt.snap`,
produced by `reextraction_workshape_receipt_snapshot`. Every count, call
number, and category-hash-changed flag quoted here is read from that
snapshot, not hand-transcribed -- if the underlying extraction/cache-churn
behavior ever drifts, `cargo insta test` / `INSTA_UPDATE=no` fails before this
document's claims can silently go stale. Timing is excluded from the
snapshot (non-deterministic on shared hardware) and stays informational-only,
reported via `eprintln!` in the sibling tests.

## What was measured

The production `didChange` -> shard-update path
(`WorkspaceIndex::index_file_with_generation`, called from
`crates/perl-lsp-rs/src/runtime/text_sync.rs:320` and `:1137` via a
background task) on a fixture file with 80 `sub`s (565+ LOC, 81 symbols
including the enclosing package -- see `fixture_prefix`/`fixture_is_large_enough`
in `reindex_workshape_measurement`), across six edit classes. Table values are
taken directly from the `.snap` file:

| # | Edit class | anchors | entities | occurrences | edges | generation_outcome | Extraction re-run? |
|---|---|---|---|---|---|---|---|
| 1 | Comment/whitespace-only (trailing, no byte-span shift) | Unchanged | Unchanged | Unchanged | Unchanged | accepted | **YES -- every extractor runs once** |
| 2 | Reference-only (new call to existing sub) | **Changed** (see note) | Unchanged | Changed | Unchanged | accepted | YES |
| 3 | Declaration-changing (new sub) | Changed | Changed | Unchanged | Changed | accepted | YES |
| 4 | Dynamic/generated fact (`eval "sub NAME {...}"`) | Changed | Changed | Changed | Unchanged | accepted | YES |
| 5 | Revert-to-original | Unchanged (== original, bit-identical) | Unchanged | Unchanged | Unchanged | accepted | YES (deterministic) |
| 6 | Superseded generation (older, out-of-order) | NotApplicableRejected | NotApplicableRejected | NotApplicableRejected | NotApplicableRejected | stale_rejected_pre_parse | **NO extraction at all -- rejected pre-parse, no stale publish** |

**Note on class 2:** `anchors_hash` changes even though no declaration
changed. `symbol_refs_to_semantic_facts`
(`crates/perl-symbol/src/surface/facts.rs`) emits one `AnchorFact` per
REFERENCE, not only per declaration -- the `anchors` category conflates
declaration-anchors and reference-anchors. Any hypothetical category-hash-gated
skip would have to treat "anchors changed" as common (any new/removed
reference trips it), not rare. This does **not** affect the comment-only case
(class 1), where no reference is added or removed either.

**Note on class 6:** the rejection happens in the PRE-PARSE monotonic
generation guard, before `Parser::new(&text).parse()` even runs -- so the
rejected call does zero extraction work (`visitor_visit_calls: 0` and every
extractor count `0` in the snapshot). Rejection itself is cheap; the
"material" finding below is about the ACCEPTED-generation classes (1-5), not
this one.

## The key measurement: comment-only edit (class 1)

Despite **zero** category-hash change (all four `Unchanged` in the snapshot),
the current path re-runs, once each, every canonical extractor:

- `IndexVisitor::visit` (legacy symbol/reference walk) -- `visitor_visit_calls: 1`
- `extract_symbol_decls` -- `canonical_decl_extract_calls: 1`
- `extract_symbol_refs` -- `canonical_ref_extract_calls: 1`
- `extract_eval_sub_boundaries` -- `dynamic_boundary_extract_calls: 1`
- `extract_generated_member_facts` -- `generated_member_extract_calls: 1`
- `extract_import_specs` -- `import_extract_calls: 1`
- `extract_use_lib_facts` -- `use_lib_extract_calls: 1`

and passes THIS FILE's own legacy symbol/search/global-reference-index
CONTRIBUTION through the removal-then-re-add routine in full: 321
symbol-table entries (`file_symbol_contribution_removed` /
`_added`, both 321 -- 80 subs x [sub + 2 params + 1 lexical] + 1 package) and
643 global-reference entries (`file_global_ref_contribution_removed` /
`_added`, both 643), all for THIS ONE URI -- identical removed/added counts,
i.e. this per-file contribution churn produces no net change, only wasted
work. This is **not** a whole-workspace cache rebuild, and the counts are
**not** necessarily the number of entries removed from the global
qualified/bare-name map -- the dual-indexing pattern
(`perl-workspace/CLAUDE.md`, PR #122) may write each contributed symbol under
up to two global keys, so the true global-map delta could be larger than the
per-file contribution count quoted here.

**Lock structure (corrected):** `index_file_with_generation` acquires
`self.files.write()` TWICE, not once. The initial generation/document-store
check acquires it briefly (`workspace_index.rs:1899-1974`) to compare the
content hash and monotonic-generation high-water mark and to update
`document_store`, then RELEASES it. Parsing, `IndexVisitor::visit`, and every
canonical/import extractor call (`workspace_index.rs:1976-2048`) then run
with the document-map write lock RELEASED, before it is re-acquired
(`workspace_index.rs:2053`) for the index-update block that does the
per-file cache-contribution churn described above. So the duplicate
extraction work is **off-lock CPU cost** -- it does not extend the time the
document-map write lock is held; it just runs twice on the CPU regardless of
whether the categories changed.

### Informational timing (NOT gated -- no hard-millisecond threshold anywhere in this PR)

Single-sample and 15-iteration repeated-edit distributions, `cargo test`
`dev`-profile-with-opts build, shared/debug hardware (informational only, per
the measurement-discipline directive; excluded from the `insta` snapshot):

| Metric | Single sample | 15-iteration min / median / max |
|---|---|---|
| Instrumented extraction total (visit + 6 extractors) | ~1.0 ms | 0.84 ms / 1.23 ms / 1.60 ms |
| Independent full re-parse of the same text | ~1.3 ms | 1.01 ms / 1.28 ms / 2.53 ms |

Extraction is **the same order of magnitude as parsing itself** on this
fixture -- roughly 50-100% of parse cost, not a negligible fraction of it.
Reference-only, declaration-changing, and dynamic-fact edits all show similar
per-edit extraction totals (~1.2-1.8 ms single-sample) since the SAME
unconditional full re-extraction runs regardless of edit class.

## Disposition: MATERIAL (off-lock CPU cost, not a correctness or lock-latency failure)

Per the issue's own framing, this crosses from "bounded" to "material":

- The comment-only case (the edit class #1711 exists to ask about) still pays
  full extraction + full per-file cache-contribution churn on every edit,
  even though the category-scoped propagation machinery
  (`ShardCategoryHashes` / `plan_shard_replacement`) proves after the fact
  that nothing downstream needed to change.
- Extraction cost is comparable to parse cost on this fixture size (not
  "cheap relative to parse" -- the bounded-disposition criterion from the
  issue thread does not hold here).
- **This is specifically an off-lock CPU-cost finding, not a
  freshness-correctness or lock-latency failure.** As detailed in the
  "Lock structure (corrected)" note above, `self.files.write()` is acquired
  and released early for the generation/document-store check
  (`workspace_index.rs:1899-1974`), then every wasted extraction call runs
  with that lock RELEASED (`:1976-2048`), before it is re-acquired for the
  cache-churn step (`:2053`). So the duplicate work does not extend the
  document-map write-lock hold time -- it is pure off-lock CPU cost.
  Correctness is not at risk either way: class 6 confirms superseded
  generations are rejected before publish (and before any extraction work at
  all, in the pre-parse case measured here); class 5 confirms determinism
  across revert.

**What this does NOT prove:** it does not prove category preclassification is
impossible. The class-2 finding (reference-anchors share the `anchors`
category with declaration-anchors) means a cheap, EXACT category-membership
precheck is not available today without re-deriving most of what the full
extractors already compute -- so a naive "preview AST walk to classify which
categories this edit could touch, then conditionally skip the full
extractors" design would likely just replace one redundant full walk with
another. Category preclassification is therefore an **UNPROVEN, LATER**
option -- not ruled out, but not validated by this PR either, and it is
**out of scope to design here** (this PR measures and recommends; it
retires nothing).

**Recommendation:** rescope the retirement path toward **consolidation
first**, not edit-class skipping:

1. **1711-B (next PR): consolidate to a single authoritative extraction
   spine.** Today the legacy `IndexVisitor` projection and the canonical
   `build_canonical_fact_shard_for_ast` extractors run independently and each
   duplicate a full AST walk over the same file. Merging them into one walk
   shrinks the "extraction total" baseline measured here regardless of any
   future skip logic, and is a prerequisite for any category-preclassification
   design to be worth its own cost.
2. **1711-C (optional, only after 1711-B lands and is remeasured):**
   reconsider edit-class skipping / category preclassification, but only once
   the post-consolidation extraction cost is remeasured against a candidate
   precheck's cost -- per this PR's measurement discipline, no skip logic
   should be built on an assumption that hasn't been measured as cheaper than
   what it avoids.

Do not over-recommend skip logic before consolidation is measured: this PR's
evidence supports "duplicate extraction work exists and is material," not
"the fix is obviously to skip it."

## Reproduction

```bash
cargo test -p perl-workspace --lib reindex_workshape_measurement -- --nocapture --test-threads=1
INSTA_UPDATE=no cargo test -p perl-workspace --lib reindex_workshape_measurement
```

Six edit-class tests, a repeated-sampling timing test, and one
`insta`-snapshotted structural-receipt test, all under
`crates/perl-workspace/src/workspace/workspace_index.rs` ::
`reindex_workshape_measurement`. Every category-hash assertion in the
edit-class tests is a hard `assert_eq!`/`assert_ne!`; the structural counts
are additionally mechanically bound via the checked-in `.snap` file; every
timing number is `eprintln!`-only (never asserted against a threshold, never
part of the snapshot).
