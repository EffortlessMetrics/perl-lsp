# perl-workspace-cache

Small SRP microcrate for bounded workspace caches, cache statistics, and cache sizing helpers.

## Exports

- `BoundedLruCache<K, V>` -- thread-safe bounded LRU cache with optional TTL expiry.
- `CacheConfig` / `CacheStats` -- cache limits and runtime metrics.
- `EstimateSize` -- trait for estimating cache entry sizes.
- `AstCacheConfig` / `SymbolCacheConfig` / `WorkspaceCacheConfig` / `CombinedWorkspaceCacheConfig` -- typed cache presets for workspace indexing.
