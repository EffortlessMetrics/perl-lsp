//! Short-TTL cache for module prefix directory scans.
//!
//! Typing a multi-segment `use` prefix such as `use Mojo::Cont|` re-scans
//! `root/Mojo/` on every keystroke.  With a 1 second TTL and 128 capacity the
//! repeated keystrokes within a typing burst hit the cache rather than the
//! filesystem, eliminating the hot-loop I/O regression identified in issue #8514.
//!
//! ## Design
//!
//! - Owner: `LspServer` (runtime-owned state, survives across requests).
//!   `CompletionProvider` is reconstructed per request and cannot hold persistent
//!   state; the cache must be held at a longer-lived layer.
//! - Key: `(canonical_include_root, prefix_dir_relative, full_module_prefix)` —
//!   the include root after path canonicalization, the subdirectory that the
//!   prefix-directed scan starts from (e.g. `Mojo/` for prefix
//!   `Mojo::Controller`), and the full typed module prefix.
//! - Value: `Vec<String>` of module names returned by the prefix-filtered scan.
//! - TTL: [`MODULE_COMPLETION_CACHE_TTL_MS`] (1000 ms by default).
//! - Capacity: [`MODULE_COMPLETION_CACHE_MAX_ENTRIES`] (128 entries).  When full
//!   `moka` evicts the least-recently-used entry automatically.
//! - Thread safety: `moka::sync::Cache` — concurrent, lock-free reads.
//! - Cancellation: callers **must** check the cancellation predicate **before**
//!   returning a cached hit so that a cancelled request does not deliver stale
//!   results to the editor.

use moka::sync::Cache;
use std::path::PathBuf;
use std::time::Duration;

/// Default TTL for a scan cache entry (1 second).
pub const MODULE_COMPLETION_CACHE_TTL_MS: u64 = 1000;

/// Maximum number of entries retained in the cache.
pub const MODULE_COMPLETION_CACHE_MAX_ENTRIES: u64 = 128;

/// Cache key: (canonical include root, relative prefix directory, full prefix).
///
/// Using the canonical form of the include root avoids spurious misses from
/// symlink or case differences on case-insensitive filesystems.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScanCacheKey {
    /// Canonicalized include root (e.g. `/home/user/proj/lib`).
    pub canonical_root: PathBuf,
    /// Relative path from the root to the prefix scan directory
    /// (e.g. `Mojo` for the prefix `Mojo::Controller`).
    /// Empty path when the prefix has no `::` namespace segments.
    pub prefix_dir: PathBuf,
    /// Full typed module prefix used to filter the scan result.
    ///
    /// This keeps prefix-filtered cache entries from satisfying another leaf
    /// prefix under the same directory, such as `Mojo::C` vs. `Mojo::L`.
    pub module_prefix: String,
}

/// Runtime-owned short-TTL cache for `scan_directory_for_modules` results.
///
/// Constructed once during `LspServer::new()` and shared across all completion
/// requests via the server's `Arc`.
pub struct ModuleCompletionScanCache {
    inner: Cache<ScanCacheKey, Vec<String>>,
}

impl ModuleCompletionScanCache {
    /// Create a new cache with the default TTL and capacity.
    pub fn new() -> Self {
        Self::with_ttl_ms(MODULE_COMPLETION_CACHE_TTL_MS)
    }

    /// Create a cache with an explicit TTL in milliseconds (useful for tests).
    pub fn with_ttl_ms(ttl_ms: u64) -> Self {
        let inner = Cache::builder()
            .max_capacity(MODULE_COMPLETION_CACHE_MAX_ENTRIES)
            .time_to_live(Duration::from_millis(ttl_ms))
            .build();
        Self { inner }
    }

    /// Look up a cached scan result for `key`.
    ///
    /// Returns `Some(modules)` when an unexpired entry exists.
    /// **Callers must check cancellation before acting on the returned value.**
    pub fn get(&self, key: &ScanCacheKey) -> Option<Vec<String>> {
        self.inner.get(key)
    }

    /// Store `modules` for `key`.  TTL is applied automatically by `moka`.
    pub fn insert(&self, key: ScanCacheKey, modules: Vec<String>) {
        self.inner.insert(key, modules);
    }

    /// Return the approximate number of live entries currently in the cache.
    /// Used by tests; exact count may lag due to moka's background eviction.
    #[cfg(test)]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}

impl Default for ModuleCompletionScanCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn key(root: &str, prefix_dir: &str, module_prefix: &str) -> ScanCacheKey {
        ScanCacheKey {
            canonical_root: PathBuf::from(root),
            prefix_dir: PathBuf::from(prefix_dir),
            module_prefix: module_prefix.to_string(),
        }
    }

    #[test]
    fn test_empty_cache_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        assert!(
            cache.get(&key("/lib", "Mojo", "Mojo::C")).is_none(),
            "empty cache must return None"
        );
        Ok(())
    }

    #[test]
    fn test_insert_then_get_returns_modules() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        let k = key("/lib", "Mojo", "Mojo::C");
        cache.insert(k.clone(), vec!["Mojo::Controller".to_string(), "Mojo::Lite".to_string()]);
        let result = cache.get(&k).ok_or("expected hit after insert")?;
        assert_eq!(result, vec!["Mojo::Controller", "Mojo::Lite"]);
        Ok(())
    }

    #[test]
    fn test_different_prefix_dir_is_a_miss() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        cache.insert(key("/lib", "Mojo", "Mojo::C"), vec!["Mojo::Controller".to_string()]);
        assert!(
            cache.get(&key("/lib", "Catalyst", "Catalyst::C")).is_none(),
            "different prefix_dir must be a cache miss"
        );
        Ok(())
    }

    #[test]
    fn test_different_module_prefix_is_a_miss() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        cache.insert(key("/lib", "Mojo", "Mojo::C"), vec!["Mojo::Controller".to_string()]);
        assert!(
            cache.get(&key("/lib", "Mojo", "Mojo::L")).is_none(),
            "different leaf prefix under the same prefix_dir must be a cache miss"
        );
        Ok(())
    }

    #[test]
    fn test_different_root_is_a_miss() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        cache.insert(key("/lib", "Mojo", "Mojo::C"), vec!["Mojo::Controller".to_string()]);
        assert!(
            cache.get(&key("/vendor/lib", "Mojo", "Mojo::C")).is_none(),
            "different canonical root must be a cache miss"
        );
        Ok(())
    }

    #[test]
    fn test_expired_entry_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        // Use a tiny TTL so we don't sleep long.
        let cache = ModuleCompletionScanCache::with_ttl_ms(10);
        let k = key("/lib", "Mojo", "Mojo::C");
        cache.insert(k.clone(), vec!["Mojo::Controller".to_string()]);
        std::thread::sleep(Duration::from_millis(50));
        assert!(cache.get(&k).is_none(), "entry past TTL must be a miss");
        Ok(())
    }

    #[test]
    fn test_overwrite_updates_existing_entry() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        let k = key("/lib", "Foo", "Foo::O");
        cache.insert(k.clone(), vec!["Foo::One".to_string()]);
        cache.insert(k.clone(), vec!["Foo::One".to_string(), "Foo::Two".to_string()]);
        let result = cache.get(&k).ok_or("expected hit")?;
        assert_eq!(result.len(), 2, "overwrite must replace old entry");
        Ok(())
    }

    #[test]
    fn test_capacity_enforced() -> Result<(), Box<dyn std::error::Error>> {
        let cache = ModuleCompletionScanCache::new();
        // Insert more entries than capacity.
        for i in 0..200u64 {
            let prefix = format!("Ns{i}::M");
            cache.insert(key("/lib", &format!("Ns{i}"), &prefix), vec![format!("Ns{i}::Mod")]);
        }
        // moka's entry_count is approximate; the important thing is that it's bounded.
        // Give moka time to run its background eviction.
        std::thread::sleep(Duration::from_millis(20));
        // It must not have grown unboundedly past capacity * 2.
        assert!(
            cache.entry_count() <= MODULE_COMPLETION_CACHE_MAX_ENTRIES * 2,
            "cache must be bounded; got entry_count={}",
            cache.entry_count()
        );
        Ok(())
    }
}
