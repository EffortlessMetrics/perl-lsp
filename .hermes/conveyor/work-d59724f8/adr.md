# ADR: Include-Path Scanning for Module Completion

## Title
ADR-2024-XXXX: Include-Path Scanning for Module Completion

## Status
Proposed

## Context

When typing `use DB` or `require DBI`, the module completion provider only suggests modules from the workspace index (modules defined in workspace files). It does NOT search:
- Configured `includePaths` from `.perl-lsp.toml`
- `PERL5LIB` environment variable paths
- System `@INC` when `useSystemInc: true` is configured

This is GitHub issue #4314. Users don't get completions for core modules like `DBI`, `Moo`, `Moose`, `Data::Dumper`, etc. unless those modules happen to be indexed from workspace files.

The module resolution system (`resolve_module_to_path_with_doc()`) already handles @INC correctly by:
1. Parsing `PERL5LIB` environment variable
2. Calling `config.effective_include_paths(&perl5lib_paths)` which merges `includePaths` config with `PERL5LIB`
3. Calling `config.get_system_inc()` when `useSystemInc: true`

The gap is that the completion provider doesn't follow the same pattern.

## Decision

We will add include-path scanning to the `add_use_module_completions()` function by:

### 1. Pass Include Paths Through the Call Chain

The `perl-lsp-completion` crate does NOT depend on `perl-lsp-config` (verified via Cargo.toml). Therefore, include paths must be passed from the runtime layer (`perl-lsp`) down to the completion crate.

In `handle_completion()` (`runtime/language/completion.rs`):
```rust
let perl5lib_paths = std::env::var("PERL5LIB")
    .map(|v| perl_lsp_config::WorkspaceConfig::parse_perl5lib(&v))
    .unwrap_or_default();
let config = self.config_for_doc(uri).unwrap_or_else(...);
let include_paths = config.effective_include_paths(&perl5lib_paths);
let system_inc_paths = if config.use_system_inc {
    config.get_system_inc().to_vec()
} else {
    Vec::new()
};
```

This corrects the bug in the initial plan (which used `include_paths_for_doc()` that doesn't merge PERL5LIB).

### 2. Extend CompletionProvider to Accept Include Paths

Add `include_paths: Vec<PathBuf>` and `system_inc_paths: Vec<PathBuf>` fields to `CompletionProvider` and create a new constructor `new_with_index_and_source_with_inc()` that accepts these paths. Keep the existing constructor working with empty paths for backward compatibility.

### 3. Implement Bounded Include-Path Scanning

Add `scan_directory_for_modules()` helper in `workspace.rs` that:
- Walks directories using `WalkDir` with max depth of 8
- Only matches `.pm` files
- Converts paths to module names using `path_to_module_name()`
- Respects LSP cancellation via `is_cancelled()` checks
- Limits results to top 20 per include path to prevent flooding

### 4. Use Bounded Caching Consistent with WorkspaceIndex Patterns

Rather than a simple TTL-based cache, we use a bounded cache approach:
- Cache key: include path directory
- Cache value: sorted list of module names discovered
- Bounded by path count (not time) — each include path has one cache entry
- Invalidation: cache entries are invalidated when the config changes (config change triggers new provider instance)
- Memory bounded: only stores module names, not full CompletionItems

**Note**: This creates a separate cache from WorkspaceIndex, which is architectural debt. The completion crate's cache will not benefit from WorkspaceIndex's BoundedLruCache, LRU eviction, or SLO tracking. A follow-on effort should consider unifying this with WorkspaceIndex or creating a shared include-path cache.

### 5. Respect Sort Tiering

External modules from include paths should use sort text prefix `"2_"` (after workspace tier `"1_"` but before generic tier `"9_"`):
- Tier 0: Hardcoded common modules (strict, warnings, DBI, etc.)
- Tier 1: Workspace modules
- Tier 2: External modules from include paths (NEW)
- Tier 3: System @INC modules when enabled (NEW, same tier as external)
- Tier 9: Generic symbols

## Consequences

### Positive
- Fixes the immediate user problem: completions now include modules from @INC
- Completion crate remains decoupled from `perl-lsp-config` (paths passed as parameters)
- Bounded cache prevents unbounded memory growth
- Path-based invalidation ensures cache freshness without complex TTL logic

### Negative
- Duplicates scanning logic between completion and module resolution
- Creates a separate cache that won't benefit from WorkspaceIndex SLO tracking
- If module resolution is updated for new @INC edge cases, completion won't automatically benefit

### Neutral
- Performance: scanning is bounded by caching, but still happens on completion hot path
- The existing `module_sort_tier()` infrastructure is reused for external modules

## Alternatives Considered

### Alternative 1: Extend WorkspaceIndex to Index Include Paths at Startup
- **Pros**: Single source of truth, O(1) lookups, reuses existing caching infrastructure
- **Cons**: Larger architectural change touching the most complex file in the workspace, higher regression risk, longer implementation time
- **Verdict**: Rejected for this issue due to scope (this is a bug fix, not a feature redesign). Recommended as follow-on effort.

### Alternative 2: Hook Into Module Resolution System for All-Match Queries
- **Pros**: Zero duplication of scanning logic, automatic handling of complex cases
- **Cons**: Module resolution API designed for single-match, not enumeration, would need significant redesign
- **Verdict**: Rejected as impractical for this scope.

### Alternative 3: Targeted On-Disk Lookup Per Prefix (No Full Directory Scan)
- **Pros**: No full directory walk needed, can work without persistent cache
- **Cons**: Doesn't discover unknown modules, essentially degenerates to hardcoded list approach
- **Verdict**: Rejected as insufficient for the use case.

## Notes

- The cache strategy accepts architectural debt from having separate caches. A follow-on should unify include-path caching with WorkspaceIndex or create a shared cache.
- Lexical `use lib` paths from source files are NOT included in this fix (deferred as more complex change).
- The `include_paths_for_doc()` method returns `Vec<PathBuf>` while `effective_include_paths()` returns `Vec<String>`. The completion crate will convert between them.
