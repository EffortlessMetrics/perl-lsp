//! Comprehensive tests for IndexState state machine and IndexCoordinator
//!
//! Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md
//!
//! This test suite validates the complete Index Lifecycle v1 specification,
//! including state transitions, parse storm detection, resource limits,
//! degradation handling, and recovery mechanisms.
//!
//! Test Structure:
//! - State transition tests (Building → Ready → Degraded)
//! - Parse storm detection and recovery
//! - Resource limit enforcement
//! - Thread-safety and Clone safety
//! - IndexCoordinator initialization patterns

use std::sync::Arc;
use std::time::Instant;

// These types will be implemented in perl-parser crate
// Currently writing TDD tests - implementation will follow

/// Index readiness state - explicit lifecycle management
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#indexstate-the-core-contract
#[derive(Clone, Debug, PartialEq)]
pub enum IndexState {
    /// Index is being constructed (workspace scan in progress)
    Building {
        /// Files indexed so far
        indexed_count: usize,
        /// Total files discovered
        total_count: usize,
        /// Started at
        started_at: Instant,
    },

    /// Index is consistent and ready for queries
    Ready {
        /// Total symbols indexed
        symbol_count: usize,
        /// Total files indexed
        file_count: usize,
        /// Timestamp of last successful index
        completed_at: Instant,
    },

    /// Index is serving but degraded
    Degraded {
        /// Why we degraded
        reason: DegradationReason,
        /// What's still available
        available_symbols: usize,
        /// When degradation occurred
        since: Instant,
    },
}

/// Reasons for index degradation
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#degradationreason
#[derive(Clone, Debug, PartialEq)]
pub enum DegradationReason {
    /// Parse storm (too many simultaneous changes)
    ParseStorm { pending_parses: usize },
    /// IO error during indexing
    IoError { message: String },
    /// Timeout during workspace scan
    ScanTimeout { elapsed_ms: u64 },
    /// Resource limits exceeded
    ResourceLimit { kind: ResourceKind },
}

/// Resource limit types
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#resourcekind
#[derive(Clone, Debug, PartialEq)]
pub enum ResourceKind {
    MaxFiles,
    MaxSymbols,
    MaxCacheBytes,
}

/// Resource limits for index operations
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#bounded-resources
#[derive(Clone, Debug, PartialEq)]
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

    /// Parse storm threshold (default: 10)
    pub parse_storm_threshold: usize,
}

impl Default for IndexResourceLimits {
    fn default() -> Self {
        Self {
            max_files: 10_000,
            max_symbols_per_file: 5_000,
            max_total_symbols: 500_000,
            max_ast_cache_bytes: 256 * 1024 * 1024,
            max_ast_cache_items: 100,
            parse_storm_threshold: 10,
        }
    }
}

/// Metrics for degradation detection
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#indexmetrics
#[derive(Debug)]
pub struct IndexMetrics {
    /// Pending parse operations
    pending_parses: std::sync::atomic::AtomicUsize,

    /// Parse storm threshold
    parse_storm_threshold: usize,

    /// Last successful index time
    ///
    /// TODO: Future use - telemetry reporting and cache invalidation timestamps
    #[allow(dead_code)]
    last_indexed: std::sync::atomic::AtomicU64,
}

impl IndexMetrics {
    fn new(parse_storm_threshold: usize) -> Self {
        Self {
            pending_parses: std::sync::atomic::AtomicUsize::new(0),
            parse_storm_threshold,
            last_indexed: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// Coordinates index lifecycle, state transitions, and handler queries
///
/// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#indexcoordinator-the-orchestrator
pub struct IndexCoordinator {
    /// Current index state (atomic for lock-free reads)
    state: Arc<std::sync::RwLock<IndexState>>,

    /// Resource limits
    limits: IndexResourceLimits,

    /// Metrics for degradation detection
    metrics: IndexMetrics,

    /// Current file count for resource limit tracking
    current_file_count: std::sync::atomic::AtomicUsize,

    /// Current symbol count for resource limit tracking
    current_symbol_count: std::sync::atomic::AtomicUsize,
}

impl IndexCoordinator {
    /// Create new coordinator starting in Building state
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_coordinator_new_starts_building
    pub fn new() -> Self {
        Self::with_limits(IndexResourceLimits::default())
    }

    /// Create coordinator with custom resource limits
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_max_files_triggers_degradation
    pub fn with_limits(limits: IndexResourceLimits) -> Self {
        let parse_storm_threshold = limits.parse_storm_threshold;
        Self {
            state: Arc::new(std::sync::RwLock::new(IndexState::Building {
                indexed_count: 0,
                total_count: 0,
                started_at: Instant::now(),
            })),
            limits,
            metrics: IndexMetrics::new(parse_storm_threshold),
            current_file_count: std::sync::atomic::AtomicUsize::new(0),
            current_symbol_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create coordinator in Ready state (for testing)
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_ready_to_degraded_on_parse_storm
    #[cfg(test)]
    pub fn new_ready(file_count: usize, symbol_count: usize) -> Self {
        let coord = Self::new();
        match coord.state.write() {
            Ok(mut guard) => {
                *guard =
                    IndexState::Ready { file_count, symbol_count, completed_at: Instant::now() };
            }
            Err(e) => {
                *e.into_inner() =
                    IndexState::Ready { file_count, symbol_count, completed_at: Instant::now() };
            }
        }
        coord.current_file_count.store(file_count, std::sync::atomic::Ordering::SeqCst);
        coord.current_symbol_count.store(symbol_count, std::sync::atomic::Ordering::SeqCst);
        coord
    }

    /// Create coordinator in Degraded state (for testing)
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_degraded_recovery
    #[cfg(test)]
    pub fn new_degraded(reason: DegradationReason) -> Self {
        let coord = Self::new();

        // Set pending parses if ParseStorm reason
        if let DegradationReason::ParseStorm { pending_parses } = &reason {
            coord
                .metrics
                .pending_parses
                .store(*pending_parses, std::sync::atomic::Ordering::SeqCst);
        }

        match coord.state.write() {
            Ok(mut guard) => {
                *guard =
                    IndexState::Degraded { reason, available_symbols: 0, since: Instant::now() };
            }
            Err(e) => {
                *e.into_inner() =
                    IndexState::Degraded { reason, available_symbols: 0, since: Instant::now() };
            }
        }
        coord
    }

    /// Check current state (lock-free for hot path)
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_state_is_clone_safe
    pub fn state(&self) -> IndexState {
        match self.state.read() {
            Ok(guard) => guard.clone(),
            Err(e) => e.into_inner().clone(),
        }
    }

    /// Complete initial workspace scan
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_building_to_ready_transition
    pub fn complete_initial_scan(&self, file_count: usize, symbol_count: usize) {
        let mut state = match self.state.write() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        *state = IndexState::Ready { file_count, symbol_count, completed_at: Instant::now() };

        // Update internal counters
        self.current_file_count.store(file_count, std::sync::atomic::Ordering::SeqCst);
        self.current_symbol_count.store(symbol_count, std::sync::atomic::Ordering::SeqCst);
    }

    /// Notify of file change (may trigger state transition)
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_ready_to_degraded_on_parse_storm
    pub fn notify_change(&self, _uri: &str) {
        use std::sync::atomic::Ordering;

        self.metrics.pending_parses.fetch_add(1, Ordering::SeqCst);

        // Check for parse storm
        let pending = self.metrics.pending_parses.load(Ordering::SeqCst);
        if pending > self.metrics.parse_storm_threshold {
            self.transition_to_degraded(DegradationReason::ParseStorm { pending_parses: pending });
        }
    }

    /// Notify parse complete
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_degraded_recovery
    pub fn notify_parse_complete(&self, _uri: &str) {
        use std::sync::atomic::Ordering;

        self.metrics.pending_parses.fetch_sub(1, Ordering::SeqCst);

        // Check for recovery from parse storm
        let pending = self.metrics.pending_parses.load(Ordering::SeqCst);
        if pending == 0
            && let IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. } =
                self.state()
        {
            self.attempt_recovery();
        }
    }

    /// Transition to degraded state
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#state-transitions
    fn transition_to_degraded(&self, reason: DegradationReason) {
        let available_symbols = self.current_symbol_count.load(std::sync::atomic::Ordering::SeqCst);

        let mut state = match self.state.write() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        *state = IndexState::Degraded { reason, available_symbols, since: Instant::now() };
    }

    /// Attempt recovery from degraded state
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_degraded_recovery
    fn attempt_recovery(&self) {
        let file_count = self.current_file_count.load(std::sync::atomic::Ordering::SeqCst);
        let symbol_count = self.current_symbol_count.load(std::sync::atomic::Ordering::SeqCst);

        let mut state = match self.state.write() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        *state = IndexState::Ready { file_count, symbol_count, completed_at: Instant::now() };
    }

    /// Index a file (may trigger resource limit degradation)
    ///
    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_max_files_triggers_degradation
    pub fn index_file(&self, _uri: &str, _content: &str) {
        use std::sync::atomic::Ordering;

        let new_count = self.current_file_count.fetch_add(1, Ordering::SeqCst) + 1;

        // Check resource limit
        if new_count > self.limits.max_files {
            self.transition_to_degraded(DegradationReason::ResourceLimit {
                kind: ResourceKind::MaxFiles,
            });
        }
    }
}

impl Default for IndexCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TEST SUITE: State Machine Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_building_to_ready_transition
    ///
    /// Validates the primary success path: Building → Ready transition when
    /// workspace scan completes successfully with proper state data preservation.
    #[test]
    fn test_building_to_ready_transition() {
        let coord = IndexCoordinator::new();

        // Initial state should be Building
        let initial_state = coord.state();
        assert!(
            matches!(initial_state, IndexState::Building { .. }),
            "IndexCoordinator should start in Building state, got: {:?}",
            initial_state
        );

        // Complete initial scan with 100 files and 5000 symbols
        coord.complete_initial_scan(100, 5000);

        // State should transition to Ready
        let state = coord.state();
        assert!(
            matches!(state, IndexState::Ready { file_count: 100, symbol_count: 5000, .. }),
            "Expected Ready state after scan completion, got: {:?}",
            state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_ready_to_degraded_on_parse_storm
    ///
    /// Validates parse storm detection: Ready → Degraded transition when
    /// pending parse count exceeds threshold (default: 10).
    #[test]
    fn test_ready_to_degraded_on_parse_storm() {
        let coord = IndexCoordinator::new_ready(100, 5000);

        // Verify initial Ready state
        let initial_state = coord.state();
        assert!(
            matches!(initial_state, IndexState::Ready { .. }),
            "Test fixture should start in Ready state, got: {:?}",
            initial_state
        );

        // Trigger parse storm with 15 simultaneous changes (threshold is 10)
        for i in 0..15 {
            coord.notify_change(&format!("file{}.pm", i));
        }

        let state = coord.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
            ),
            "Expected Degraded state with ParseStorm after 15 changes, got: {:?}",
            state
        );
        if let IndexState::Degraded {
            reason: DegradationReason::ParseStorm { pending_parses },
            available_symbols,
            ..
        } = state
        {
            assert!(
                pending_parses > 10,
                "Parse storm should trigger at threshold (10), got: {}",
                pending_parses
            );
            assert_eq!(
                available_symbols, 5000,
                "Available symbols should be preserved during degradation"
            );
        }
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_degraded_recovery
    ///
    /// Validates automatic recovery: Degraded (ParseStorm) → Ready when
    /// all pending parses complete (pending count reaches 0).
    #[test]
    fn test_degraded_recovery() {
        let coord =
            IndexCoordinator::new_degraded(DegradationReason::ParseStorm { pending_parses: 10 });

        // Verify initial Degraded state
        let initial_state = coord.state();
        assert!(
            matches!(
                initial_state,
                IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
            ),
            "Test fixture should start in Degraded state with ParseStorm, got: {:?}",
            initial_state
        );

        // Clear pending parses one by one
        for i in 0..10 {
            coord.notify_parse_complete(&format!("file{}.pm", i));
        }

        // State should recover to Ready when pending reaches 0
        let recovered_state = coord.state();
        assert!(
            matches!(recovered_state, IndexState::Ready { .. }),
            "Index should recover to Ready state after parse storm clears, got: {:?}",
            recovered_state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_max_files_triggers_degradation
    ///
    /// Validates resource limit enforcement: transition to Degraded when
    /// max_files limit is exceeded during indexing operations.
    #[test]
    fn test_max_files_triggers_degradation() {
        let limits = IndexResourceLimits { max_files: 10, ..Default::default() };
        let coord = IndexCoordinator::with_limits(limits);

        // Index files up to and beyond the limit
        for i in 0..15 {
            coord.index_file(&format!("file{}.pm", i), "");
        }

        let state = coord.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded {
                    reason: DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles },
                    ..
                }
            ),
            "Expected Degraded state with ResourceLimit(MaxFiles) after exceeding limit, got: {:?}",
            state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_coordinator_new_starts_building
    ///
    /// Validates initialization contract: IndexCoordinator::new() must start
    /// in Building state with zero indexed files.
    #[test]
    fn test_coordinator_new_starts_building() {
        let coord = IndexCoordinator::new();

        let state = coord.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 0, total_count: 0, .. }),
            "IndexCoordinator::new() must start in Building state with 0 files, got: {:?}",
            state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#test_state_is_clone_safe
    ///
    /// Validates thread-safety: IndexState::clone() must work correctly and
    /// coordinator.state() must return consistent snapshots across threads.
    #[test]
    fn test_state_is_clone_safe() {
        let coord = Arc::new(IndexCoordinator::new_ready(100, 5000));

        // Clone state multiple times
        let state1 = coord.state();
        let state2 = coord.state();
        let state3 = state1.clone();

        // All clones should be equal
        assert_eq!(state1, state2, "Multiple state() calls should return equal states");
        assert_eq!(state1, state3, "Cloned state should equal original");

        let state = coord.state();
        assert!(
            matches!(state, IndexState::Ready { file_count: 100, symbol_count: 5000, .. }),
            "Expected Ready state, got: {:?}",
            state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#state-transitions
    ///
    /// Validates that Building state can transition to Degraded on timeout
    /// without going through Ready first.
    #[test]
    fn test_building_to_degraded_on_timeout() {
        let coord = IndexCoordinator::new();

        // Verify Building state
        assert!(
            matches!(coord.state(), IndexState::Building { .. }),
            "Should start in Building state"
        );

        // Simulate scan timeout
        coord.transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: 35000 });

        let state = coord.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded { reason: DegradationReason::ScanTimeout { elapsed_ms: 35000 }, .. }
            ),
            "Expected Degraded state with ScanTimeout (35000ms), got: {:?}",
            state
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#state-transitions
    ///
    /// Validates that Degraded state can transition to Building during recovery
    /// attempt (re-scan after cooldown).
    #[test]
    fn test_degraded_to_building_on_recovery_attempt() {
        let coord = IndexCoordinator::new_degraded(DegradationReason::IoError {
            message: "disk full".to_string(),
        });

        // Verify Degraded state
        assert!(
            matches!(
                coord.state(),
                IndexState::Degraded { reason: DegradationReason::IoError { .. }, .. }
            ),
            "Should start in Degraded state with IoError"
        );

        // Attempt recovery by transitioning to Building (simulating re-scan)
        match coord.state.write() {
            Ok(mut guard) => {
                *guard = IndexState::Building {
                    indexed_count: 0,
                    total_count: 100,
                    started_at: Instant::now(),
                };
            }
            Err(e) => {
                *e.into_inner() = IndexState::Building {
                    indexed_count: 0,
                    total_count: 100,
                    started_at: Instant::now(),
                };
            }
        }

        // Verify transition to Building
        assert!(
            matches!(coord.state(), IndexState::Building { .. }),
            "Recovery should transition to Building for re-scan"
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#bounded-resources
    ///
    /// Validates that custom resource limits are respected by IndexCoordinator.
    #[test]
    fn test_custom_resource_limits() {
        let custom_limits = IndexResourceLimits {
            max_files: 5,
            max_symbols_per_file: 100,
            max_total_symbols: 500,
            max_ast_cache_bytes: 1024 * 1024,
            max_ast_cache_items: 10,
            parse_storm_threshold: 3,
        };

        let coord = IndexCoordinator::with_limits(custom_limits.clone());

        // Verify coordinator uses custom limits
        assert_eq!(coord.limits.max_files, 5, "Custom max_files limit should be applied");
        assert_eq!(
            coord.limits.parse_storm_threshold, 3,
            "Custom parse storm threshold should be applied"
        );

        // Trigger parse storm with custom threshold (3)
        for i in 0..5 {
            coord.notify_change(&format!("file{}.pm", i));
        }

        // Should degrade at lower threshold
        assert!(
            matches!(
                coord.state(),
                IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
            ),
            "Should degrade with custom parse storm threshold of 3"
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#state-transitions
    ///
    /// Validates that multiple rapid state transitions are handled correctly
    /// and state remains consistent.
    #[test]
    fn test_rapid_state_transitions() {
        let coord = IndexCoordinator::new();

        // Building → Ready
        coord.complete_initial_scan(50, 2500);
        assert!(matches!(coord.state(), IndexState::Ready { .. }));

        // Ready → Degraded (parse storm)
        for i in 0..12 {
            coord.notify_change(&format!("file{}.pm", i));
        }
        assert!(
            matches!(
                coord.state(),
                IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
            ),
            "Should degrade on parse storm"
        );

        // Degraded → Ready (recovery)
        for i in 0..12 {
            coord.notify_parse_complete(&format!("file{}.pm", i));
        }
        assert!(
            matches!(coord.state(), IndexState::Ready { .. }),
            "Should recover after parse storm clears"
        );
    }

    /// Tests feature spec: INDEX_LIFECYCLE_V1_SPEC.md#indexmetrics
    ///
    /// Validates that pending_parses counter is correctly maintained through
    /// notify_change and notify_parse_complete calls.
    #[test]
    fn test_pending_parses_tracking() {
        use std::sync::atomic::Ordering;

        let coord = IndexCoordinator::new_ready(10, 100);

        // Initially should be 0
        assert_eq!(
            coord.metrics.pending_parses.load(Ordering::SeqCst),
            0,
            "Pending parses should start at 0"
        );

        // Add some pending parses
        coord.notify_change("file1.pm");
        coord.notify_change("file2.pm");
        coord.notify_change("file3.pm");

        assert_eq!(
            coord.metrics.pending_parses.load(Ordering::SeqCst),
            3,
            "Pending parses should increment with notify_change"
        );

        // Complete one parse
        coord.notify_parse_complete("file1.pm");

        assert_eq!(
            coord.metrics.pending_parses.load(Ordering::SeqCst),
            2,
            "Pending parses should decrement with notify_parse_complete"
        );
    }
}
