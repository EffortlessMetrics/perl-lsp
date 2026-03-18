//! Bounded workspace cache primitives for Perl tooling.
//!
//! This microcrate isolates generic cache infrastructure from higher-level
//! workspace indexing logic so caches can evolve and be tested independently.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache configuration for bounded LRU caches.
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Maximum number of items in the cache.
    pub max_items: usize,
    /// Maximum memory usage in bytes.
    pub max_bytes: usize,
    /// TTL for cache entries (`None` = no expiration).
    pub ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { max_items: 10_000, max_bytes: 50 * 1024 * 1024, ttl: None }
    }
}

/// Cache statistics for monitoring and diagnostics.
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Total number of cache hits.
    pub hits: u64,
    /// Total number of cache misses.
    pub misses: u64,
    /// Total number of evictions.
    pub evictions: u64,
    /// Current number of items in cache.
    pub current_items: usize,
    /// Current memory usage in bytes.
    pub current_bytes: usize,
    /// Hit rate (`hits / (hits + misses)`).
    pub hit_rate: f64,
}

impl CacheStats {
    /// Calculate hit rate from hits and misses.
    pub fn calculate_hit_rate(hits: u64, misses: u64) -> f64 {
        let total = hits + misses;
        if total == 0 { 0.0 } else { hits as f64 / total as f64 }
    }
}

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    last_accessed: Instant,
    _inserted_at: Instant,
    size_bytes: usize,
}

impl<V> CacheEntry<V> {
    fn new(value: V, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self { value, last_accessed: now, _inserted_at: now, size_bytes }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.last_accessed.elapsed() > ttl
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Thread-safe bounded LRU cache.
pub struct BoundedLruCache<K, V>
where
    K: Hash + Eq + Clone,
{
    entries: Arc<Mutex<HashMap<K, CacheEntry<V>>>>,
    access_order: Arc<Mutex<Vec<K>>>,
    config: CacheConfig,
    stats: Arc<Mutex<CacheStats>>,
}

impl<K, V> BoundedLruCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new bounded LRU cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(Vec::new())),
            config,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Insert a value into the cache using an explicit size.
    pub fn insert_with_size(&self, key: K, value: V, size_bytes: usize) -> bool {
        let mut entries = self.entries.lock();
        let mut access_order = self.access_order.lock();
        let mut stats = self.stats.lock();

        if let Some(entry) = entries.get_mut(&key) {
            entry.value = value;
            entry.size_bytes = size_bytes;
            entry.touch();

            if let Some(pos) = access_order.iter().position(|k| k == &key) {
                access_order.remove(pos);
            }
            access_order.push(key.clone());

            stats.current_bytes = entries.values().map(|entry| entry.size_bytes).sum();
            stats.current_items = entries.len();
            return true;
        }

        while !entries.is_empty()
            && (entries.len() >= self.config.max_items
                || stats.current_bytes + size_bytes > self.config.max_bytes)
        {
            if let Some(lru_key) = access_order.first() {
                if let Some(entry) = entries.remove(lru_key) {
                    stats.current_bytes -= entry.size_bytes;
                    stats.evictions += 1;
                }
                access_order.remove(0);
            } else {
                break;
            }
        }

        if entries.len() >= self.config.max_items
            || stats.current_bytes + size_bytes > self.config.max_bytes
        {
            return false;
        }

        entries.insert(key.clone(), CacheEntry::new(value, size_bytes));
        access_order.push(key);
        stats.current_bytes = entries.values().map(|entry| entry.size_bytes).sum();
        stats.current_items = entries.len();
        true
    }

    /// Insert a value using its estimated size.
    pub fn insert(&self, key: K, value: V)
    where
        V: EstimateSize,
    {
        let size_bytes = value.estimate_size();
        self.insert_with_size(key, value, size_bytes);
    }

    /// Get a cloned value from the cache, refreshing its LRU position.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut entries = self.entries.lock();
        let mut access_order = self.access_order.lock();
        let mut stats = self.stats.lock();

        if let Some(ttl) = self.config.ttl
            && let Some(entry) = entries.get(key)
            && entry.is_expired(ttl)
        {
            entries.remove(key);
            if let Some(pos) = access_order.iter().position(|cached_key| cached_key == key) {
                access_order.remove(pos);
            }
            stats.misses += 1;
            stats.hit_rate = CacheStats::calculate_hit_rate(stats.hits, stats.misses);
            return None;
        }

        if let Some(entry) = entries.get_mut(key) {
            entry.touch();
            if let Some(pos) = access_order.iter().position(|cached_key| cached_key == key) {
                let key_clone = access_order.remove(pos);
                access_order.push(key_clone);
            }
            stats.hits += 1;
            stats.hit_rate = CacheStats::calculate_hit_rate(stats.hits, stats.misses);
            Some(entry.value.clone())
        } else {
            stats.misses += 1;
            stats.hit_rate = CacheStats::calculate_hit_rate(stats.hits, stats.misses);
            None
        }
    }

    /// Remove a value from the cache.
    pub fn remove(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut entries = self.entries.lock();
        let mut access_order = self.access_order.lock();
        let mut stats = self.stats.lock();

        if let Some(entry) = entries.remove(key) {
            stats.current_bytes -= entry.size_bytes;
            stats.current_items = entries.len();
            if let Some(pos) = access_order.iter().position(|cached_key| cached_key == key) {
                access_order.remove(pos);
            }
            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear all cached values.
    pub fn clear(&self) {
        let mut entries = self.entries.lock();
        let mut access_order = self.access_order.lock();
        let mut stats = self.stats.lock();
        entries.clear();
        access_order.clear();
        stats.current_bytes = 0;
        stats.current_items = 0;
    }

    /// Return whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// Return the number of cached items.
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Return a snapshot of cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().clone()
    }

    /// Return the cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}

impl<K, V> Default for BoundedLruCache<K, V>
where
    K: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Trait for estimating the memory size of cached values.
pub trait EstimateSize {
    /// Estimate the memory size of this value in bytes.
    fn estimate_size(&self) -> usize;
}

impl EstimateSize for String {
    fn estimate_size(&self) -> usize {
        self.len()
    }
}

impl<T> EstimateSize for Vec<T>
where
    T: EstimateSize,
{
    fn estimate_size(&self) -> usize {
        self.iter().map(EstimateSize::estimate_size).sum()
    }
}

impl<K, V> EstimateSize for HashMap<K, V>
where
    K: EstimateSize,
    V: EstimateSize,
{
    fn estimate_size(&self) -> usize {
        self.iter().map(|(key, value)| key.estimate_size() + value.estimate_size()).sum()
    }
}

impl EstimateSize for str {
    fn estimate_size(&self) -> usize {
        self.len()
    }
}

impl<T: EstimateSize + ?Sized> EstimateSize for &T {
    fn estimate_size(&self) -> usize {
        (**self).estimate_size()
    }
}

impl EstimateSize for [u8] {
    fn estimate_size(&self) -> usize {
        self.len()
    }
}

impl EstimateSize for () {
    fn estimate_size(&self) -> usize {
        0
    }
}

impl<T> EstimateSize for Option<T>
where
    T: EstimateSize,
{
    fn estimate_size(&self) -> usize {
        self.as_ref().map(EstimateSize::estimate_size).unwrap_or(0)
    }
}

impl<T, E> EstimateSize for Result<T, E>
where
    T: EstimateSize,
    E: EstimateSize,
{
    fn estimate_size(&self) -> usize {
        match self {
            Ok(value) => value.estimate_size(),
            Err(err) => err.estimate_size(),
        }
    }
}

/// AST node cache configuration.
#[derive(Clone, Debug)]
pub struct AstCacheConfig {
    /// Maximum number of AST nodes to cache.
    pub max_nodes: usize,
    /// Maximum memory for AST cache in bytes.
    pub max_bytes: usize,
}

impl Default for AstCacheConfig {
    fn default() -> Self {
        Self { max_nodes: 10_000, max_bytes: 50 * 1024 * 1024 }
    }
}

/// Symbol cache configuration.
#[derive(Clone, Debug)]
pub struct SymbolCacheConfig {
    /// Maximum number of symbols to cache.
    pub max_symbols: usize,
    /// Maximum memory for symbol cache in bytes.
    pub max_bytes: usize,
}

impl Default for SymbolCacheConfig {
    fn default() -> Self {
        Self { max_symbols: 50_000, max_bytes: 30 * 1024 * 1024 }
    }
}

/// Workspace cache configuration.
#[derive(Clone, Debug)]
pub struct WorkspaceCacheConfig {
    /// Maximum number of workspace files to cache.
    pub max_files: usize,
    /// Maximum memory for workspace cache in bytes.
    pub max_bytes: usize,
}

impl Default for WorkspaceCacheConfig {
    fn default() -> Self {
        Self { max_files: 1_000, max_bytes: 20 * 1024 * 1024 }
    }
}

/// Combined cache configuration for all workspace caches.
#[derive(Clone, Debug, Default)]
pub struct CombinedWorkspaceCacheConfig {
    /// AST node cache configuration.
    pub ast: AstCacheConfig,
    /// Symbol cache configuration.
    pub symbol: SymbolCacheConfig,
    /// Workspace cache configuration.
    pub workspace: WorkspaceCacheConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn cache_insert_get_round_trips() {
        let cache = BoundedLruCache::<String, String>::default();
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = BoundedLruCache::<String, String>::default();
        assert_eq!(cache.get(&"missing".to_string()), None);
    }

    #[test]
    fn cache_evicts_lru_entry() {
        let cache = BoundedLruCache::<String, String>::new(CacheConfig {
            max_items: 2,
            max_bytes: 100,
            ttl: None,
        });
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string());

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), Some("value3".to_string()));
    }

    #[test]
    fn cache_ttl_expires_entries() {
        let cache = BoundedLruCache::<String, String>::new(CacheConfig {
            max_items: 4,
            max_bytes: 1024,
            ttl: Some(Duration::from_millis(10)),
        });
        cache.insert("key".to_string(), "value".to_string());
        thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    #[test]
    fn cache_stats_track_hits_and_misses() {
        let cache = BoundedLruCache::<String, String>::default();
        cache.insert("key1".to_string(), "value1".to_string());
        let _ = cache.get(&"key1".to_string());
        let _ = cache.get(&"key2".to_string());
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn estimate_size_supports_common_composites() {
        let values = vec!["hello", "world"];
        assert_eq!(values.estimate_size(), 10);
        let result: Result<&str, &str> = Ok("perl");
        assert_eq!(result.estimate_size(), 4);
    }
}
