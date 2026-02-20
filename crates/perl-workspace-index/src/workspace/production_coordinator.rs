//! Workspace index coordinator with cache and SLO integration.
//!
//! This module provides an enhanced index coordinator that integrates:
//! - Enhanced index lifecycle state machine
//! - Bounded LRU caches for AST nodes, symbols, and workspace data
//! - Service Level Objectives (SLOs) monitoring
//! - Performance optimization for large workspaces
//!
//! # Architecture
//!
//! ```text
//! ProductionIndexCoordinator
//!   ├── state_machine: IndexStateMachine
//!   ├── index: WorkspaceIndex
//!   ├── cache: WorkspaceCacheManager
//!   ├── slo_tracker: SloTracker
//!   └── limits: IndexResourceLimits
//! ```
//!
//! # Performance Characteristics
//!
//! - **State checks**: Lock-free reads (<100ns)
//! - **Cache lookups**: O(1) average with LRU eviction
//! - **SLO tracking**: <10μs overhead per operation
//! - **Memory usage**: Bounded by configured limits (default: 100MB total)
//!
//! # Usage
//!
//! ```rust
//! use perl_workspace_index::workspace::production_coordinator::ProductionIndexCoordinator;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let coordinator = ProductionIndexCoordinator::default();
//!
//! // Index a file
//! let uri = url::Url::parse("file:///example.pl")?;
//! coordinator.index_file(uri, "sub hello { return 42; }".to_string())?;
//! # Ok(())
//! # }
//! ```

use super::cache::{BoundedLruCache, CombinedWorkspaceCacheConfig};
use super::slo::{OperationResult, OperationType, SloConfig, SloTracker};
use super::state_machine::{IndexState, IndexStateMachine, InvalidationReason, TransitionResult};
use super::workspace_index::{IndexResourceLimits, WorkspaceIndex};
use crate::position::{Position, Range};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// Cache manager for workspace index components.
///
/// Manages bounded LRU caches for AST nodes, symbols, and workspace data.
pub struct WorkspaceCacheManager {
    /// AST node cache
    ast_cache: BoundedLruCache<String, Vec<u8>>,
    /// Symbol cache
    symbol_cache: BoundedLruCache<String, Vec<u8>>,
    /// Workspace file cache
    workspace_cache: BoundedLruCache<String, Vec<u8>>,
}

impl WorkspaceCacheManager {
    /// Create a new cache manager with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Cache configuration
    ///
    /// # Returns
    ///
    /// A new cache manager instance.
    pub fn new(config: &CombinedWorkspaceCacheConfig) -> Self {
        Self {
            ast_cache: BoundedLruCache::new(super::cache::CacheConfig {
                max_items: config.ast.max_nodes,
                max_bytes: config.ast.max_bytes,
                ttl: None,
            }),
            symbol_cache: BoundedLruCache::new(super::cache::CacheConfig {
                max_items: config.symbol.max_symbols,
                max_bytes: config.symbol.max_bytes,
                ttl: None,
            }),
            workspace_cache: BoundedLruCache::new(super::cache::CacheConfig {
                max_items: config.workspace.max_files,
                max_bytes: config.workspace.max_bytes,
                ttl: None,
            }),
        }
    }

    /// Get AST node from cache.
    pub fn get_ast(&self, key: &str) -> Option<Vec<u8>> {
        self.ast_cache.get(&key.to_string())
    }

    /// Insert AST node into cache.
    pub fn insert_ast(&self, key: String, value: Vec<u8>) {
        let size = value.len();
        self.ast_cache.insert_with_size(key, value, size);
    }

    /// Get symbol from cache.
    pub fn get_symbol(&self, key: &str) -> Option<Vec<u8>> {
        self.symbol_cache.get(&key.to_string())
    }

    /// Insert symbol into cache.
    pub fn insert_symbol(&self, key: String, value: Vec<u8>) {
        let size = value.len();
        self.symbol_cache.insert_with_size(key, value, size);
    }

    /// Get workspace data from cache.
    pub fn get_workspace(&self, key: &str) -> Option<Vec<u8>> {
        self.workspace_cache.get(&key.to_string())
    }

    /// Insert workspace data into cache.
    pub fn insert_workspace(&self, key: String, value: Vec<u8>) {
        let size = value.len();
        self.workspace_cache.insert_with_size(key, value, size);
    }

    /// Clear all caches.
    pub fn clear_all(&self) {
        self.ast_cache.clear();
        self.symbol_cache.clear();
        self.workspace_cache.clear();
    }

    /// Get combined cache statistics.
    pub fn stats(&self) -> HashMap<String, super::cache::CacheStats> {
        let mut stats = HashMap::new();
        stats.insert("ast".to_string(), self.ast_cache.stats());
        stats.insert("symbol".to_string(), self.symbol_cache.stats());
        stats.insert("workspace".to_string(), self.workspace_cache.stats());
        stats
    }

    /// Get total memory usage across all caches.
    pub fn total_memory_usage(&self) -> usize {
        self.ast_cache.stats().current_bytes
            + self.symbol_cache.stats().current_bytes
            + self.workspace_cache.stats().current_bytes
    }
}

/// Configuration for the production index coordinator.
#[derive(Clone, Debug, Default)]
pub struct ProductionCoordinatorConfig {
    /// Cache configuration
    pub cache_config: CombinedWorkspaceCacheConfig,
    /// SLO configuration
    pub slo_config: SloConfig,
    /// Resource limits
    pub resource_limits: IndexResourceLimits,
}

/// Workspace index coordinator.
///
/// Integrates state machine, caching, and SLO monitoring with
/// comprehensive performance optimization.
pub struct ProductionIndexCoordinator {
    /// Enhanced state machine for index lifecycle
    state_machine: IndexStateMachine,
    /// Underlying workspace index
    index: Arc<WorkspaceIndex>,
    /// Cache manager for bounded LRU caches
    cache: Arc<WorkspaceCacheManager>,
    /// SLO tracker for performance monitoring
    slo_tracker: Arc<SloTracker>,
    /// Configuration
    config: ProductionCoordinatorConfig,
}

impl ProductionIndexCoordinator {
    /// Create a new production coordinator with default configuration.
    ///
    /// # Returns
    ///
    /// A new production coordinator instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::production_coordinator::ProductionIndexCoordinator;
    ///
    /// let coordinator = ProductionIndexCoordinator::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(ProductionCoordinatorConfig::default())
    }

    /// Create a new production coordinator with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Coordinator configuration
    ///
    /// # Returns
    ///
    /// A new production coordinator instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::production_coordinator::{
    ///     ProductionCoordinatorConfig, ProductionIndexCoordinator,
    /// };
    ///
    /// let config = ProductionCoordinatorConfig::default();
    /// let coordinator = ProductionIndexCoordinator::with_config(config);
    /// ```
    pub fn with_config(config: ProductionCoordinatorConfig) -> Self {
        let cache = Arc::new(WorkspaceCacheManager::new(&config.cache_config));
        let slo_tracker = Arc::new(SloTracker::new(config.slo_config.clone()));

        Self {
            state_machine: IndexStateMachine::new(),
            index: Arc::new(WorkspaceIndex::new()),
            cache,
            slo_tracker,
            config,
        }
    }

    /// Get current index state.
    ///
    /// # Returns
    ///
    /// The current index state.
    pub fn state(&self) -> IndexState {
        self.state_machine.state()
    }

    /// Get reference to the underlying workspace index.
    pub fn index(&self) -> &Arc<WorkspaceIndex> {
        &self.index
    }

    /// Get reference to the cache manager.
    pub fn cache(&self) -> &Arc<WorkspaceCacheManager> {
        &self.cache
    }

    /// Get reference to the SLO tracker.
    pub fn slo_tracker(&self) -> &Arc<SloTracker> {
        &self.slo_tracker
    }

    /// Get the coordinator configuration.
    pub fn config(&self) -> &ProductionCoordinatorConfig {
        &self.config
    }

    /// Initialize the workspace index.
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeded, otherwise an error.
    pub fn initialize(&self) -> Result<(), String> {
        let start = self.slo_tracker.start_operation(OperationType::IndexInitialization);

        // Transition to Initializing
        match self.state_machine.transition_to_initializing() {
            TransitionResult::Success => {}
            result => {
                return Err(format!("Failed to transition to Initializing: {:?}", result));
            }
        }

        // Update progress
        self.state_machine.update_initialization_progress(100);

        // Transition to Building
        match self.state_machine.transition_to_building(0) {
            TransitionResult::Success => {}
            result => {
                return Err(format!("Failed to transition to Building: {:?}", result));
            }
        }

        // Transition to Ready
        match self.state_machine.transition_to_ready(0, 0) {
            TransitionResult::Success => {}
            result => {
                return Err(format!("Failed to transition to Ready: {:?}", result));
            }
        }

        self.slo_tracker.record_operation_type(
            OperationType::IndexInitialization,
            start,
            OperationResult::Success,
        );

        Ok(())
    }

    /// Index a file with caching and SLO tracking.
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI
    /// * `text` - File content
    ///
    /// # Returns
    ///
    /// `Ok(())` if indexing succeeded, otherwise an error.
    pub fn index_file(&self, uri: Url, text: String) -> Result<(), String> {
        let start = self.slo_tracker.start_operation(OperationType::FileIndexing);

        // Check cache first
        let cache_key = uri.to_string();
        if let Some(_cached) = self.cache.get_ast(&cache_key) {
            // Cache hit - skip re-indexing
            self.slo_tracker.record_operation_type(
                OperationType::FileIndexing,
                start,
                OperationResult::Success,
            );
            return Ok(());
        }

        // Index the file
        self.index.index_file(uri.clone(), text)?;

        // Cache the result
        let serialized = self.serialize_file_index(&uri)?;
        self.cache.insert_ast(cache_key, serialized);

        // Update state if needed
        if matches!(self.state(), IndexState::Ready { .. }) {
            // Transition to Updating
            let _ = self.state_machine.transition_to_updating(1);
            let _ = self
                .state_machine
                .transition_to_ready(self.index.file_count(), self.index.symbol_count());
        }

        self.slo_tracker.record_operation_type(
            OperationType::FileIndexing,
            start,
            OperationResult::Success,
        );

        Ok(())
    }

    /// Find definition with caching and SLO tracking.
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - Symbol name to look up
    ///
    /// # Returns
    ///
    /// Definition location if found.
    pub fn find_definition(&self, symbol_name: &str) -> Option<super::workspace_index::Location> {
        let start = self.slo_tracker.start_operation(OperationType::DefinitionLookup);

        // Check cache first
        let cache_key = format!("def:{}", symbol_name);
        if let Some(cached) = self.cache.get_symbol(&cache_key) {
            // Cache hit
            self.slo_tracker.record_operation_type(
                OperationType::DefinitionLookup,
                start,
                OperationResult::Success,
            );
            return self.deserialize_location(&cached);
        }

        // Perform lookup
        let result = self.index.find_definition(symbol_name);

        // Cache the result
        if let Some(ref location) = result {
            let serialized = self.serialize_location(location);
            self.cache.insert_symbol(cache_key, serialized);
        }

        self.slo_tracker.record_operation_type(
            OperationType::DefinitionLookup,
            start,
            OperationResult::Success,
        );

        result
    }

    /// Find references with caching and SLO tracking.
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - Symbol name to look up
    ///
    /// # Returns
    ///
    /// All reference locations found.
    pub fn find_references(&self, symbol_name: &str) -> Vec<super::workspace_index::Location> {
        let start = self.slo_tracker.start_operation(OperationType::FindReferences);

        // Check cache first
        let cache_key = format!("ref:{}", symbol_name);
        if let Some(cached) = self.cache.get_symbol(&cache_key) {
            // Cache hit
            self.slo_tracker.record_operation_type(
                OperationType::FindReferences,
                start,
                OperationResult::Success,
            );
            return self.deserialize_locations(&cached).unwrap_or_default();
        }

        // Perform lookup
        let result = self.index.find_references(symbol_name);

        // Cache the result
        let serialized = self.serialize_locations(&result);
        self.cache.insert_symbol(cache_key, serialized);

        self.slo_tracker.record_operation_type(
            OperationType::FindReferences,
            start,
            OperationResult::Success,
        );

        result
    }

    /// Invalidate the index.
    ///
    /// # Arguments
    ///
    /// * `reason` - Reason for invalidation
    pub fn invalidate(&self, reason: InvalidationReason) {
        // Transition to Invalidating
        let _ = self.state_machine.transition_to_invalidating(reason);

        // Clear caches
        self.cache.clear_all();

        // Clear index
        // Note: This is a simplified version - in production you'd want
        // more sophisticated invalidation logic
        self.index.clear();

        // Transition back to Idle
        let _ = self.state_machine.transition_to_idle();
    }

    /// Get combined statistics.
    ///
    /// # Returns
    ///
    /// Combined statistics including cache and SLO stats.
    pub fn statistics(&self) -> CoordinatorStatistics {
        CoordinatorStatistics {
            state: self.state(),
            cache_stats: self.cache.stats(),
            slo_stats: self.slo_tracker.all_statistics(),
            total_memory_usage: self.cache.total_memory_usage(),
            all_slos_met: self.slo_tracker.all_slos_met(),
        }
    }

    /// Serialize a location for caching.
    fn serialize_location(&self, location: &super::workspace_index::Location) -> Vec<u8> {
        // Simple serialization - in production use a proper serialization format
        format!(
            "{}:{}:{}:{}",
            location.uri,
            location.range.start.line,
            location.range.start.column,
            location.range.end.line
        )
        .into_bytes()
    }

    /// Deserialize a location from cache.
    fn deserialize_location(&self, data: &[u8]) -> Option<super::workspace_index::Location> {
        // Simple deserialization
        let s = String::from_utf8(data.to_vec()).ok()?;
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() >= 4 {
            Some(super::workspace_index::Location {
                uri: parts[0].to_string(),
                range: Range {
                    start: Position {
                        byte: 0,
                        line: parts[1].parse().ok()?,
                        column: parts[2].parse().ok()?,
                    },
                    end: Position { byte: 0, line: parts[3].parse().ok()?, column: 0 },
                },
            })
        } else {
            None
        }
    }

    /// Serialize locations for caching.
    fn serialize_locations(&self, locations: &[super::workspace_index::Location]) -> Vec<u8> {
        // Simple serialization
        locations.iter().flat_map(|loc| self.serialize_location(loc)).collect()
    }

    /// Deserialize locations from cache.
    fn deserialize_locations(&self, data: &[u8]) -> Option<Vec<super::workspace_index::Location>> {
        // Simple deserialization - split by null bytes
        let s = String::from_utf8(data.to_vec()).ok()?;
        s.split('\0')
            .filter_map(|part| self.deserialize_location(part.as_bytes()))
            .collect::<Vec<_>>()
            .into()
    }

    /// Serialize file index for caching.
    fn serialize_file_index(&self, _uri: &Url) -> Result<Vec<u8>, String> {
        // Placeholder - in production, serialize the actual file index
        Ok(Vec::new())
    }
}

impl Default for ProductionIndexCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined statistics for the production coordinator.
#[derive(Clone, Debug)]
pub struct CoordinatorStatistics {
    /// Current index state
    pub state: IndexState,
    /// Cache statistics
    pub cache_stats: HashMap<String, super::cache::CacheStats>,
    /// SLO statistics
    pub slo_stats: HashMap<OperationType, super::slo::SloStatistics>,
    /// Total memory usage across all caches
    pub total_memory_usage: usize,
    /// Whether all SLOs are being met
    pub all_slos_met: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = ProductionIndexCoordinator::new();
        assert!(matches!(coordinator.state(), IndexState::Idle { .. }));
    }

    #[test]
    fn test_coordinator_initialize() -> Result<(), String> {
        let coordinator = ProductionIndexCoordinator::new();
        coordinator.initialize()?;
        assert!(coordinator.state().is_ready());
        Ok(())
    }

    #[test]
    fn test_coordinator_index_file() -> Result<(), String> {
        let coordinator = ProductionIndexCoordinator::new();
        coordinator.initialize()?;

        let uri = Url::parse("file:///example.pl").map_err(|e| e.to_string())?;
        let code = "sub hello { return 42; }";
        coordinator.index_file(uri, code.to_string())?;
        Ok(())
    }

    #[test]
    fn test_coordinator_find_definition() -> Result<(), String> {
        let coordinator = ProductionIndexCoordinator::new();
        coordinator.initialize()?;

        let uri = Url::parse("file:///example.pl").map_err(|e| e.to_string())?;
        let code = "sub hello { return 42; }";
        coordinator.index_file(uri, code.to_string())?;

        let def = coordinator.find_definition("hello");
        assert!(def.is_some());
        Ok(())
    }

    #[test]
    fn test_coordinator_statistics() {
        let coordinator = ProductionIndexCoordinator::new();
        let stats = coordinator.statistics();

        assert!(matches!(stats.state, IndexState::Idle { .. }));
        assert_eq!(stats.cache_stats.len(), 3);
        assert_eq!(stats.slo_stats.len(), 8);
    }

    #[test]
    fn test_coordinator_invalidate() -> Result<(), String> {
        let coordinator = ProductionIndexCoordinator::new();
        coordinator.initialize()?;

        let uri = Url::parse("file:///example.pl").map_err(|e| e.to_string())?;
        coordinator.index_file(uri, "sub hello {}".to_string())?;

        coordinator.invalidate(InvalidationReason::ManualRequest);
        assert!(matches!(coordinator.state(), IndexState::Idle { .. }));
        Ok(())
    }
}
