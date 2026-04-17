# ADR: HashMap-Based Symbol Search Index for WorkspaceIndex

**Work Item:** work-dd44521b
**Title:** [PERL] Linear symbol search in workspace symbol provider (O(n) complexity)
**Status:** Proposed

---

## Context

The `WorkspaceIndex::search_symbols()` method in `perl-workspace-index` performs an O(n) linear scan over all files and all symbols for every search query. As workspace size grows (target: 10K files, 500K symbols per v0.12.6 milestone), this becomes a performance bottleneck.

The codebase has a `perl-symbol-index` crate containing a `SymbolIndex` with a trie and inverted index. However, using it directly has critical architectural gaps:

1. **Result reconstruction gap**: `SymbolIndex::search_prefix()` and `search_fuzzy()` return `Vec<String>` (symbol names only), not `Vec<WorkspaceSymbol>` with location, range, and kind data. The plan would need an additional structure to map names back to full objects.

2. **Removal API gap**: `SymbolIndex` has no `remove_symbol()` method. File updates/removes would leave stale symbols in the index, causing incorrect search results.

3. **Query semantics mismatch**: The existing search uses substring matching (`name.contains(query)`), but `perl-symbol-index` uses prefix matching or tokenized fuzzy matching. A query like `"process_data"` would NOT match `"MyModule::process_data"` via prefix search.

The original plan attempted to use `perl-symbol-index` as a drop-in, but the gaps above make it architecturally unsuitable without fundamental changes to `perl-symbol-index`.

---

## Decision

**Use a HashMap-based inverted index built directly within `WorkspaceIndex`**:

```rust
// New field in WorkspaceIndex:
global_name_index: Arc<RwLock<HashMap<String, Vec<WorkspaceSymbol>>>>,
```

This index maps lowercase symbol names (both bare names AND qualified names) to the actual `WorkspaceSymbol` objects.

### Search Strategy

1. **Exact name match**: O(1) HashMap lookup → return matching `Vec<WorkspaceSymbol>`
2. **Prefix/contains match**: For each HashMap entry, iterate symbols and filter by `name.contains(query)` — this is bounded by the size of the matching name bucket, not the entire workspace
3. **Fallback**: If query is very short (1-2 chars), the HashMap lookup returns a large bucket; apply the existing `contains` filter to narrow results

### Index Maintenance

- **On file index**: Insert each symbol's name and qualified_name into the HashMap (dual indexing)
- **On file update**: Remove old symbols for that file's URI, then insert new ones
- **On file remove**: Remove all symbols for that file's URI

This uses HashMap's natural insert/remove semantics — no new crate API needed.

### Dual-Name Indexing

Per the codebase's "dual indexing strategy" (documented in `CRATE_ARCHITECTURE_GUIDE`):
- Index under bare name for unqualified calls
- Index under qualified name for `Package::function` calls

Both are inserted into the same `HashMap<String, Vec<WorkspaceSymbol>>`, with deduplication by URI.

---

## Alternatives Considered

### Alternative 1: Integrate `perl-symbol-index` Directly

Use `SymbolIndex` for trie-based prefix matching and fuzzy search.

**Rejected because**:
- Critical gaps (no removal API, returns names not full objects, query semantics mismatch)
- Would require fundamental changes to `perl-symbol-index` before integration
- Risk of correctness regression: substring searches would return different results

### Alternative 2: Cooperative Cancellation

Add cooperative yielding to `search_symbols()` so it exits early on cancellation.

**Rejected because**:
- UX patch, not a fix — doesn't improve algorithmic complexity
- Doesn't solve the root cause for small workspaces or uncancelled searches
- Doesn't reduce O(n) latency, only makes it interruptible

### Alternative 3: Consolidate Dual-Path First

Before optimizing `WorkspaceIndex::search_symbols()`, consolidate it with `WorkspaceSymbolsProvider::search()` (the non-`workspace` feature path).

**Deferred because**:
- Larger refactoring across crates, outside current scope
- The `workspace` feature flag controls which path is used; `workspace` is default
- Documented as a concern but not addressed in this work item

---

## Consequences

### Benefits

- **O(1) lookup** for exact name matches via HashMap
- **Bounded iteration** for partial/contains matches — iterates only over symbols with matching names, not the entire workspace
- **Clean incremental maintenance** with HashMap insert/remove semantics
- **No external dependency changes** — doesn't modify `perl-symbol-index`
- **Aligns with existing patterns** — extends the `symbols: HashMap<String, String>` pattern already used for `find_definition()`
- **Correctness preserved** — maintains existing substring matching semantics, not prefix or fuzzy

### Tradeoffs

- **Memory overhead**: Symbols stored twice — per-file `Vec<WorkspaceSymbol>` in `FileIndex` and global `HashMap<String, Vec<WorkspaceSymbol>>`. However, only `Arc<WorkspaceSymbol>` references are cloned, not full objects. Acceptable given the v0.12.6 memory targets.

- **Short query handling**: Single-character queries (`"a"`) return all symbols starting with "a" — same as existing behavior, but the HashMap approach makes this more visible. Mitigation: apply `contains` filter within the matching bucket.

- **Dual-name deduplication**: Must store both bare name and qualified name entries, but deduplicate by URI to avoid duplicate results.

### Risks

- **Index corruption on failure**: If `index_file()` or `remove_file()` fails after partial update, the HashMap may be inconsistent with `FileIndex`. Mitigation: use atomic updates (remove-all-then-insert) rather than incremental modifications.

- **Memory at scale**: At 500K symbols, the HashMap overhead (String keys + Vec buckets + Arc references) should be measured against baseline. Acceptable if <2x baseline.

---

## Scope Boundaries

### In Scope
- `WorkspaceIndex::search_symbols()` optimization (the default `workspace` feature path)
- Dual-name indexing (bare name + qualified name)
- Incremental index maintenance on file add/update/remove
- Backward compatibility: same `search_symbols()` signature and behavior

### Out of Scope
- `WorkspaceSymbolsProvider::search()` optimization (non-default path)
- Changes to `perl-symbol-index` crate
- LSP protocol or API changes
- Ranking/sorting changes
- Adding new search features (e.g., search by type)
