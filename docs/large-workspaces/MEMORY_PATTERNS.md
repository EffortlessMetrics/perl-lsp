# Memory Patterns for Large Workspaces

This document describes how memory grows with workspace size, which
components hold the most memory, and how to reduce usage without
sacrificing correctness.

---

## Memory Scaling Overview

From the `WorkspaceIndex` module documentation:

> Memory usage: ~1MB per 10K symbols with optimized storage.

At the cache defaults (`CacheConfig::default`):

| Cache          | Max items | Max bytes | Notes                          |
|----------------|-----------|-----------|--------------------------------|
| AST node cache | 10 000    | 50 MB     | Evicted LRU when limit reached |
| Symbol cache   | 50 000    | 30 MB     | Most frequently accessed       |
| Workspace cache | 1 000    | 20 MB     | Per-file metadata              |

The index itself stores two `HashMap` entries per symbol (qualified name
`Package::sub` and bare name `sub`), each holding a `Location` value
(URI + `Range`). At 64-bit pointers:

- ~100 bytes per symbol entry (key string + location struct + hash overhead)
- 50 000 symbols -> ~5 MB for the index tables alone
- At 10 000 files with 5 subs each -> ~50 000 symbols -> ~5 MB index

The dominant cost for large workspaces is therefore the **AST cache**, not
the symbol tables.

---

## WorkspaceIndex Memory Scaling

### Symbol Count Projections

| Workspace files | Avg. subs/file | Total symbols | Approx. index RAM |
|-----------------|----------------|---------------|-------------------|
| 100             | 5              | 500           | ~50 KB            |
| 1 000           | 5              | 5 000         | ~500 KB           |
| 5 000           | 5              | 25 000        | ~2.5 MB           |
| 10 000          | 5              | 50 000        | ~5 MB             |
| 50 000          | 5              | 250 000       | ~25 MB            |

These figures are for the index tables only. Add AST cache and document
store on top.

### What Holds Memory After Indexing

1. **Symbol HashMap** (`qualified_index`, `bare_index`) — proportional to
   symbol count; bounded by `IndexResourceLimits::max_total_symbols`.
2. **AST cache** (`BoundedLruCache`) — caches parsed ASTs; bounded by
   `CacheConfig::max_items` and `CacheConfig::max_bytes`.
3. **DocumentStore** — holds the raw text of open documents; bounded by
   the number of open editor tabs, not workspace size.
4. **Reference index** — stores back-references from symbol to use sites;
   grows faster than definitions.

---

## AST Cache Behavior

The `BoundedLruCache` in `crates/perl-workspace-index/src/workspace/cache.rs`
uses an LRU eviction policy:

- **Hit**: Returns the cached AST; touches the entry to mark it recently
  used.
- **Miss**: Parses the file on demand; inserts the new AST, evicting the
  least-recently-used entry if either `max_items` or `max_bytes` is
  exceeded.
- **TTL**: Optional. If set, entries expire regardless of access pattern.
  Useful for preventing stale ASTs after external file changes.

### When ASTs Are Cached

An AST is cached when `index_file` is called and the result is retained
by the `ProductionIndexCoordinator`. The cache is queried first; if the
file's content has not changed (same text hash), the existing AST is
returned without re-parsing.

### When ASTs Are Evicted

- The cache exceeds `max_items` (default 10 000).
- The cache exceeds `max_bytes` (default 50 MB).
- `DocumentStore::update` is called with new content (content change
  invalidates the cache entry for that URI).
- The coordinator receives a workspace invalidation signal.

### Tuning the AST Cache

In the LSP configuration (`perllsp.json` or editor settings):

```json
{
  "perl": {
    "limits": {
      "astCacheMaxEntries": 2000,
      "astCacheMaxBytes": 10485760
    }
  }
}
```

Lower values reduce peak memory; higher values reduce re-parse latency.
For a 5 000-file workspace where most files fit in memory, 5 000 entries
with 25 MB is a reasonable sweet spot.

---

## Common Memory Anti-Patterns

### 1. Unbounded String Duplication

**Pattern**: Cloning `String` values from the symbol table into every
caller.

**Consequence**: At 50 000 symbols, returning `Vec<String>` from
`find_symbols` allocates thousands of copies per query.

**Fix**: Return `&str` references, `Arc<str>`, or `Cow<str>` from symbol
lookup functions. The symbol table keys are already `String` values owned
by the index; returning a reference avoids the copy entirely.

### 2. Holding All Symbols in a `Vec` for Filtering

**Pattern**: `index.all_symbols().into_iter().filter(|s| s.contains(q))`

**Consequence**: Forces all symbols into a temporary `Vec` before
filtering, even if only a handful match.

**Fix**: Use the `find_symbols` prefix query, which filters inside the
index before allocating the result set. The index is already sorted in
a way that supports prefix scanning.

### 3. Circular References via `Arc`

**Pattern**: `Arc<WorkspaceIndex>` inside a `DocumentStore` that is also
inside `WorkspaceIndex`.

**Consequence**: Neither side is ever dropped; both leak indefinitely.

**Fix**: Use `Weak<T>` for back-references. The `ProductionIndexCoordinator`
owns both; individual components should not own each other.

### 4. Retaining Stale File Content

**Pattern**: `DocumentStore` never calling `close` for files removed from
the workspace.

**Consequence**: Raw text for deleted files accumulates in memory.

**Fix**: Send `textDocument/didClose` notifications from the editor, or
wire a file-watcher that calls `DocumentStore::close` on `fs::remove`.

---

## Memory Optimization Techniques

### Capacity Hints

When you know the workspace size upfront, pre-allocate the symbol table:

```rust
// Future API — not yet on WorkspaceIndex, but demonstrates the pattern
let index = WorkspaceIndex::with_capacity(
    50_000,  // expected symbol count
    10_000,  // expected file count
);
```

This avoids incremental `HashMap` resize storms during initial indexing.

### `Arc<str>` for Repeated String Keys

If the same package name appears in thousands of symbols, interning the
package prefix under an `Arc<str>` reduces allocations:

```rust
use std::sync::Arc;

// Instead of:
let key = format!("{}::{}", package, name);

// Consider:
let pkg: Arc<str> = Arc::from(package);
let key = format!("{}::{}", pkg, name);
// Arc<str> clones share the heap allocation
```

For heavier deduplication, a string-interning crate such as `string_cache`
or `lasso` reduces per-symbol memory further.

### Weak References for Back-Pointers

```rust
use std::sync::{Arc, Weak};

struct WorkspaceIndex {
    coordinator: Weak<ProductionIndexCoordinator>, // not Arc
    ...
}
```

### Memory-Mapped File Reading

For very large workspaces, reading each file with `mmap` rather than
`fs::read_to_string` avoids copying the entire file content into heap
memory before parsing. This is a significant change and should be
benchmarked carefully.

---

## Monitoring Memory in Production

The `SloTracker` records request latencies but not memory. To monitor
memory at runtime:

```bash
# Check RSS of the running LSP server
ps -o pid,rss,comm -p $(pgrep perllsp)

# Detailed per-mapping view
cat /proc/$(pgrep perllsp)/smaps_rollup
```

For continuous monitoring, the `--health` flag prints index statistics
including estimated symbol count, which correlates with memory:

```bash
perllsp --health
```

---

## See Also

- `TESTING_GUIDE.md` — generating workspaces to trigger these patterns
- `PROFILING_GUIDE.md` — measuring actual allocation with heaptrack
- `TROUBLESHOOTING.md` — diagnosing memory growth in a running server
- `docs/how-to/PERFORMANCE_TUNING.md` — configuration knobs for memory limits
