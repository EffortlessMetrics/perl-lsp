# Specifications: HashMap-Based Symbol Search Index

**Work Item:** work-dd44521b
**Feature:** Replace O(n) linear symbol search with indexed lookup in `WorkspaceIndex`

---

## Feature Description

Optimize `WorkspaceIndex::search_symbols()` from O(n) linear scan to indexed lookup by adding a `HashMap<String, Vec<WorkspaceSymbol>>` global name index inside `WorkspaceIndex`. This index maps lowercase symbol names (both bare names and qualified names) to the actual `WorkspaceSymbol` objects.

### Search Behavior

The optimized `search_symbols()` must preserve existing behavior:

1. **Case-insensitive matching**: `search_symbols("Calc")` matches `calculate_total`
2. **Substring matching**: `search_symbols("calc")` matches `MyModule::calculate_total` (contains match on qualified name)
3. **Dual-name search**: Query matches both `symbol.name` and `symbol.qualified_name`
4. **Result ordering**: Returns all matching symbols (no ranking/sorting changes)

### Index Maintenance

The global name index must stay synchronized with the per-file `FileIndex::symbols` on:

- **File indexed**: Insert all symbols (name + qualified_name entries) into the global HashMap
- **File updated**: Remove all symbols with that file's URI, then insert updated symbols
- **File removed**: Remove all symbols with that file's URI from the global HashMap

---

## Acceptance Criteria

### AC1: Correctness — Same Search Results as Baseline

**Test**: For a representative set of 20+ search queries across different patterns (exact match, substring, qualified name, single char), `search_symbols()` returns the same `Vec<WorkspaceSymbol>` as the baseline O(n) implementation.

**Verification**: A new integration test compares optimized results against a reference implementation that performs the original O(n) scan.

### AC2: Performance — Measurable Latency Reduction at Scale

**Test**: `bench_search_symbols_at_scale` (existing benchmark at `crates/perl-workspace-index/benches/workspace_index_benchmark.rs:770`) shows >50% latency reduction at 100K+ symbols.

**Verification**: Benchmark comparison before/after optimization. Target: <100ms for 500K symbol workspace (down from ~2s baseline).

### AC3: Index Consistency — Correct Maintenance on File Changes

**Test**: After `index_file()`, `update_file()`, and `remove_file()` operations, the global name index contains exactly the symbols from current `files` HashMap.

**Verification**: Unit tests that perform index/update/remove sequences and assert the global name index matches the expected state.

### AC4: Memory — Reasonable Overhead at Scale

**Test**: Memory usage at 500K symbols is <2x the baseline memory usage (excluding workspace source files).

**Verification**: Memory benchmark comparing baseline vs. optimized implementation.

### AC5: Backward Compatibility — Same API and Behavior

**Test**: The `search_symbols()` method signature and return type are unchanged.

**Verification**: Compilation succeeds with existing call sites in `crates/perl-lsp/src/runtime/workspace.rs`.

---

## Non-Goals

- **Not optimizing `WorkspaceSymbolsProvider::search()`**: This is the non-default path (used when `workspace` feature is disabled). Out of scope for this work item.

- **Not modifying `perl-symbol-index`**: The HashMap approach does not require or depend on changes to the `perl-symbol-index` crate.

- **Not changing LSP protocol or ranking**: The search results behavior must match the existing implementation. No new ranking algorithms or LSP feature additions.

- **Not supporting new query types**: This optimization preserves existing substring matching semantics. Does not add regex, type-based, or other search modes.

---

## Dependencies

- **Internal**: Requires `WorkspaceIndex` struct (in `crates/perl-workspace-index/src/workspace/workspace_index.rs`)
- **Internal**: Requires `FileIndex::symbols` (the per-file symbol list that seeds the global index)
- **Internal**: Requires `incremental_add_symbols()` / `incremental_remove_symbols()` patterns (existing index maintenance methods)
- **External**: None — this optimization does not add new crate dependencies

---

## Data Structures

### New Field in WorkspaceIndex

```rust
// Added to WorkspaceIndex struct:
global_name_index: Arc<RwLock<HashMap<String, Vec<WorkspaceSymbol>>>>,
```

- Key: lowercase symbol name (bare name OR qualified name)
- Value: `Vec<WorkspaceSymbol>` with that name (multiple if same name from different files/uris)

### Dual-Name Indexing

Each symbol contributes TWO entries to the HashMap (deduplicated by URI):
1. `symbol.name.to_lowercase()` → `symbol`
2. `symbol.qualified_name.as_ref().map(|qn| qn.to_lowercase())` → `symbol` (if present)

### Search Flow

```
search_symbols(query):
  query_lower = query.to_lowercase()

  // Phase 1: Get candidate symbols via HashMap
  candidates = Vec<WorkspaceSymbol>::new()
  for each key matching query_lower (exact or prefix):
    candidates.extend(global_name_index.get(key))

  // Phase 2: Filter with substring semantics
  results = candidates.iter()
    .filter(|s| s.name.contains(&query_lower)
          || s.qualified_name.contains(&query_lower))
    .collect()

  return deduplicated results (by URI)
```

---

## Verification Plan

1. **Correctness test**: Capture baseline result sets for 20+ queries, assert optimized matches baseline
2. **Benchmark**: Run `bench_search_symbols_at_scale` before/after, assert >50% improvement at 100K+
3. **Consistency test**: Verify index matches `files` HashMap after add/update/remove operations
4. **Memory benchmark**: Measure RSS before/after at 500K symbols
5. **Compilation check**: Verify `cargo check --all-features` passes in `perl-workspace-index`
