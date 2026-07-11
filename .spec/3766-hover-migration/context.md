# Context: #3766 — Migrate hover to generation-owned analyzer/type_environment

## Problem

Hover currently reads semantic analysis facts from LspServer-level `(uri, content_hash)`-keyed caches, not from the generation-owned lazy cells on `ParsedSnapshot`. This means:

1. **Freshness gap**: Hover is content-hash-gated, not generation-gated. If the document content stays the same hash but regenerates (e.g., during a slow parse cycle), hover will re-use old cached results instead of re-analyzing the current generation. This violates the "fresh facts fast" guarantee that Phase 5 (#3760/#3765) established for completion and should extend to hover.

2. **Constructor inefficiency**: Every hover request on the same document triggers `get_or_build_analyzer()` and `get_or_build_type_engine()`, which re-analyze the AST and re-infer types per request (even though completion in #3765 proved that OnceLock-based lazy construction is safe and efficient).

3. **Architecture mismatch**: Completion was migrated to `snapshot.semantic_analyzer()` / `snapshot.type_environment()` in #3765, but hover was deliberately left on the old caches as a follow-up. Now that #3765 is merged, hover is the only remaining consumer of the old cache API, making it a blocker for cleanup.

**User-visible impact**: Hovering over a variable multiple times or after quick edits may show stale type information if the document regenerates but keeps the same content hash. For users on slow machines or with expensive parse operations, this is a real UX regression vs. the freshness promise of Phase 5.

## Why this approach

The Phase 5 approach (generation-owned lazy cells on ParsedSnapshot) was chosen for completion in #3765 because it achieves:
- **Inherent generation-correctness**: Each ParsedSnapshot carries its own generation counter. When a document edits, a new generation is published, and old snapshots naturally age out. Hover facts are scoped to the snapshot's generation.
- **Lazy construction**: OnceLock on ParsedSnapshot guarantees exactly-once construction per snapshot. Repeated hovers on the same generation share the same Arc<SemanticAnalyzer>, no rebuild per request.
- **Removal of dual-write risk**: Old approach had separate `ast` and `cache` fields (risk of disagreement). New approach: snapshot is the single source of truth; its cells are derived from its own AST.
- **Off-lock analysis**: Snapshot is Arc-owned; callers drop the documents lock before analysis begins. Analyzer and type-engine work entirely off-lock (no re-entry risk).

Hover follows the same pattern because it's the same problem: hover needs fresh semantic facts, scoped to the current generation, not re-built per request. The proven solution from #3765 is the right approach.

## Alternatives rejected

1. **Keep the old caches but make them generation-gated**: Rejected because it duplicates the lazy-construction logic that ParsedSnapshot already provides (OnceLock, exactly-once, Arc-sharing). Also maintains two paths to the same data (redundancy, inconsistency risk).

2. **Migrate hover first, leave references/rename for later**: Rejected by task framing — the task explicitly says "scope to HOVER ONLY (references/rename are SEPARATE follow-up slices)". This keeps the PR focused and testable.

3. **Migrate hover but keep the old cache fields "just in case"**: Rejected because dead code is a maintenance burden. Once hover is the only consumer and we migrate it, the caches are unreachable (no caller). Deletion is mandatory for coherence (the issue explicitly requires it).

4. **Add a new "generation-scoped cache" layer on LspServer**: Rejected as over-architecture. ParsedSnapshot's OnceLock is simpler, faster, and already proven by #3765. No need for a new abstraction.

## Prior art / duplicates

- **#3765 (PR #3765 — merged)**: Migration of completion to `snapshot.semantic_analyzer()` / `snapshot.type_environment()`. This is the reference implementation. Hover follows the same pattern: extract snapshot, call the methods, handle Option-wrapped returns.
- **#3760 (Phase 5 kick-off)**: Established the generation-owned cells on ParsedSnapshot. Hover benefits from this groundwork without adding new infrastructure.
- **#3396 (off-lock providers)**: Established the pattern of releasing the documents lock before analysis begins. Hover already follows this pattern (lock released at line 68, analysis at line 77).

No duplicate hover-migration work exists. This is the first (and only) hover migration to the new pattern.

## Links

- **Issue**: [#3766](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3766)
- **Parent**: [#3760 — Fresh Facts Fast Phase 5](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3760)
- **Reference**: [PR #3765 — completion migration (merged)](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3765)
- **PARSER_CONTRACTS.md**: Not directly relevant (semantic analyzer is not a parser contract)
- **docs/concepts**: 
  - [orchestrator-substrate-model](docs/concepts/orchestrator-substrate-model.md) — off-lock provider pattern
  - [shift-left-ladder](docs/concepts/shift-left-ladder.md) — haiku scouts/verifiers before sonnet builders
- **docs/learnings**: 
  - [2026-06-agentic-maintenance-field-notes.md](docs/learnings/2026-06-agentic-maintenance-field-notes.md) — Fresh Facts Fast observations
- **Related issues**:
  - [#3765 — completion migration](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3765) — reference implementation
  - [#3767 (follow-up slice 2)](#) — references migration (same approach, separate PR)
  - [#3768 (follow-up slice 3)](#) — rename migration (same approach, separate PR)

## Scope boundaries (HOVER ONLY)

**Excluded from this spec** (marked as follow-up slices):
- References provider (`references.rs`) — reads semantic analyzer via `SemanticAnalyzer::analyze(ast)` (empty-source overload, lower quality)
- Rename provider (`rename.rs`) — same issue as references
- Navigation provider (`navigation.rs`) — also uses `analyze_with_source` at a few sites, but may be lower priority

The task explicitly constrains this slice to **HOVER ONLY** to keep the PR focused and testable. A single-provider migration is easier to review and reason about than a three-provider rollout. Each follow-up slice can then be independent.

## Builder notes

1. **Test feature flag**: Construction-count test (Step 13 in checklist) requires `--features expose_lsp_test_api`. See `crates/perl-lsp-rs/CLAUDE.md` for threading constraints (`RUST_TEST_THREADS=2`).

2. **Compiler will guide you**: Once you delete the cache methods and fields, the compiler will report every stray reference. That's a feature — no silent dead code, no missed consumers.

3. **Doc comment update**: The hover.rs file has a doc comment (line 197) that says hover uses `get_or_build_analyzer` to cache results. Update this to explain the snapshot-based approach (generation-scoped, lazy, OnceLock-backed).

4. **Grep-verify before deleting**: Before Step 3-4 (delete methods), run:
   ```bash
   grep -rn "get_or_build_analyzer\|get_or_build_type_engine" crates/perl-lsp-rs/src
   ```
   Expected result: only references from `hover.rs` (which you've already migrated in Steps 1-2). If any other match appears, investigate and update it first.

5. **Integration test data**: The new test files (Steps 10-13) will need small Perl code snippets (e.g., `my $x = 42;`). Use `perl_tdd_support::must_some` and `insta` snapshots (see completion tests for examples).

6. **Optional: perf test**: If you want to prove the construction-count improvement empirically, you can add a simple benchmark: N hovers on the same generation, measure time / verify it's linear in N (not exponential). This is optional for acceptance but makes the efficiency win visible.
