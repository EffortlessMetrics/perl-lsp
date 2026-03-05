//! Resource and performance limits for workspace indexing.

#![deny(unsafe_code)]
#![warn(missing_docs)]

/// Type of resource limit that was exceeded.
///
/// Identifies which bounded resource triggered index degradation,
/// enabling targeted eviction strategies and capacity planning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    /// Maximum number of files in index exceeded
    MaxFiles,

    /// Maximum total symbols exceeded
    MaxSymbols,

    /// Maximum AST cache bytes exceeded
    MaxCacheBytes,
}

/// Configurable resource limits for workspace index.
#[derive(Clone, Debug)]
pub struct IndexResourceLimits {
    /// Maximum files to index (default: 10,000)
    pub max_files: usize,

    /// Maximum symbols per file (default: 5,000)
    pub max_symbols_per_file: usize,

    /// Maximum total symbols (default: 500,000)
    pub max_total_symbols: usize,

    /// Maximum AST cache size in bytes (default: 256MB)
    pub max_ast_cache_bytes: usize,

    /// Maximum AST cache items (default: 100)
    pub max_ast_cache_items: usize,

    /// Maximum workspace scan duration in milliseconds (default: 30,000ms = 30s)
    pub max_scan_duration_ms: u64,
}

impl Default for IndexResourceLimits {
    fn default() -> Self {
        Self {
            max_files: 10_000,
            max_symbols_per_file: 5_000,
            max_total_symbols: 500_000,
            max_ast_cache_bytes: 256 * 1024 * 1024,
            max_ast_cache_items: 100,
            max_scan_duration_ms: 30_000,
        }
    }
}

/// Performance caps for workspace indexing operations.
#[derive(Clone, Debug)]
pub struct IndexPerformanceCaps {
    /// Initial workspace scan budget in milliseconds (default: 100ms)
    pub initial_scan_budget_ms: u64,
    /// Incremental update budget in milliseconds (default: 10ms)
    pub incremental_budget_ms: u64,
}

impl Default for IndexPerformanceCaps {
    fn default() -> Self {
        Self { initial_scan_budget_ms: 100, incremental_budget_ms: 10 }
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexPerformanceCaps, IndexResourceLimits, ResourceKind};

    #[test]
    fn default_resource_limits_are_stable() {
        let limits = IndexResourceLimits::default();
        assert_eq!(limits.max_files, 10_000);
        assert_eq!(limits.max_symbols_per_file, 5_000);
        assert_eq!(limits.max_total_symbols, 500_000);
        assert_eq!(limits.max_ast_cache_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_ast_cache_items, 100);
        assert_eq!(limits.max_scan_duration_ms, 30_000);
    }

    #[test]
    fn default_performance_caps_are_stable() {
        let caps = IndexPerformanceCaps::default();
        assert_eq!(caps.initial_scan_budget_ms, 100);
        assert_eq!(caps.incremental_budget_ms, 10);
    }

    #[test]
    fn resource_kind_variants_are_comparable() {
        assert_eq!(ResourceKind::MaxFiles, ResourceKind::MaxFiles);
        assert_ne!(ResourceKind::MaxSymbols, ResourceKind::MaxCacheBytes);
    }
}
