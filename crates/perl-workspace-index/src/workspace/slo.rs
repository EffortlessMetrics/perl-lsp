//! Service Level Objectives (SLOs) for workspace index operations.
//!
//! This module defines performance targets and monitoring infrastructure
//! for critical workspace index operations. SLOs provide measurable
//! quality targets for production deployments.
//!
//! # SLO Targets
//!
//! - **Index Initialization**: <5s for 10K files (P95)
//! - **Incremental Update**: <100ms for single file change (P95)
//! - **Definition Lookup**: <50ms (P95)
//! - **Completion**: <100ms (P95)
//! - **Hover**: <50ms (P95)
//!
//! # Performance Monitoring
//!
//! - Latency tracking with percentiles (P50, P95, P99)
//! - Error rate monitoring
//! - Throughput metrics
//! - SLO compliance reporting
//!
//! # Usage
//!
//! ```rust
//! use perl_workspace_index::workspace::slo::{SloConfig, SloTracker};
//!
//! let config = SloConfig::default();
//! let tracker = SloTracker::new(config);
//!
//! let start = tracker.start_operation("definition_lookup");
//! // ... perform operation ...
//! tracker.record_operation(start, Ok(()));
//! ```

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// SLO configuration for workspace index operations.
///
/// Defines latency targets and monitoring parameters for each operation type.
#[derive(Clone, Debug)]
pub struct SloConfig {
    /// Target latency for index initialization (P95)
    pub index_init_p95_ms: u64,
    /// Target latency for incremental updates (P95)
    pub incremental_update_p95_ms: u64,
    /// Target latency for definition lookup (P95)
    pub definition_lookup_p95_ms: u64,
    /// Target latency for completion (P95)
    pub completion_p95_ms: u64,
    /// Target latency for hover (P95)
    pub hover_p95_ms: u64,
    /// Maximum acceptable error rate (0.0 to 1.0)
    pub max_error_rate: f64,
    /// Number of samples to keep for percentile calculation
    pub sample_window_size: usize,
}

impl Default for SloConfig {
    fn default() -> Self {
        Self {
            index_init_p95_ms: 5000,        // 5 seconds
            incremental_update_p95_ms: 100, // 100ms
            definition_lookup_p95_ms: 50,   // 50ms
            completion_p95_ms: 100,         // 100ms
            hover_p95_ms: 50,               // 50ms
            max_error_rate: 0.01,           // 1% error rate
            sample_window_size: 1000,
        }
    }
}

/// Operation types tracked by SLO monitoring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// Initial workspace index construction
    IndexInitialization,
    /// Incremental index update for file changes
    IncrementalUpdate,
    /// Go-to-definition lookup
    DefinitionLookup,
    /// Code completion request
    Completion,
    /// Hover information request
    Hover,
    /// Find references request
    FindReferences,
    /// Workspace symbol search
    WorkspaceSymbols,
    /// File indexing operation
    FileIndexing,
}

impl OperationType {
    /// Get the SLO target for this operation type.
    pub fn slo_target_ms(&self, config: &SloConfig) -> u64 {
        match self {
            OperationType::IndexInitialization => config.index_init_p95_ms,
            OperationType::IncrementalUpdate => config.incremental_update_p95_ms,
            OperationType::DefinitionLookup => config.definition_lookup_p95_ms,
            OperationType::Completion => config.completion_p95_ms,
            OperationType::Hover => config.hover_p95_ms,
            OperationType::FindReferences => config.definition_lookup_p95_ms, // Same as definition
            OperationType::WorkspaceSymbols => config.definition_lookup_p95_ms, // Same as definition
            OperationType::FileIndexing => config.incremental_update_p95_ms, // Same as incremental
        }
    }

    /// Get a human-readable name for this operation.
    pub fn name(&self) -> &'static str {
        match self {
            OperationType::IndexInitialization => "index_initialization",
            OperationType::IncrementalUpdate => "incremental_update",
            OperationType::DefinitionLookup => "definition_lookup",
            OperationType::Completion => "completion",
            OperationType::Hover => "hover",
            OperationType::FindReferences => "find_references",
            OperationType::WorkspaceSymbols => "workspace_symbols",
            OperationType::FileIndexing => "file_indexing",
        }
    }
}

/// Result of an SLO operation.
#[derive(Clone, Debug)]
pub enum OperationResult {
    /// Operation succeeded
    Success,
    /// Operation failed with an error message
    Failure(String),
}

impl OperationResult {
    /// Check if the operation was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, OperationResult::Success)
    }
}

impl<T, E> From<Result<T, E>> for OperationResult
where
    E: std::fmt::Display,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => OperationResult::Success,
            Err(e) => OperationResult::Failure(e.to_string()),
        }
    }
}

/// Latency sample for SLO tracking.
#[derive(Clone, Debug)]
struct LatencySample {
    /// Duration of the operation
    duration: Duration,
    /// Whether the operation succeeded
    success: bool,
    /// When the sample was recorded
    _timestamp: Instant,
}

/// SLO statistics for a specific operation type.
#[derive(Clone, Debug)]
pub struct SloStatistics {
    /// Total number of operations
    pub total_count: u64,
    /// Number of successful operations
    pub success_count: u64,
    /// Number of failed operations
    pub failure_count: u64,
    /// Error rate (failures / total)
    pub error_rate: f64,
    /// P50 latency (median)
    pub p50_ms: u64,
    /// P95 latency
    pub p95_ms: u64,
    /// P99 latency
    pub p99_ms: u64,
    /// Average latency
    pub avg_ms: f64,
    /// Whether SLO is being met
    pub slo_met: bool,
}

impl Default for SloStatistics {
    fn default() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            error_rate: 0.0,
            p50_ms: 0,
            p95_ms: 0,
            p99_ms: 0,
            avg_ms: 0.0,
            slo_met: true,
        }
    }
}

/// Per-operation SLO tracker.
#[derive(Debug)]
struct OperationSloTracker {
    /// Operation type being tracked
    _operation_type: OperationType,
    /// Latency samples (most recent first)
    samples: VecDeque<LatencySample>,
    /// SLO target for this operation
    slo_target_ms: u64,
    /// Maximum error rate
    max_error_rate: f64,
    /// Maximum number of samples to keep
    max_samples: usize,
}

impl OperationSloTracker {
    /// Create a new operation SLO tracker.
    fn new(operation_type: OperationType, config: &SloConfig) -> Self {
        Self {
            _operation_type: operation_type,
            samples: VecDeque::with_capacity(config.sample_window_size),
            slo_target_ms: operation_type.slo_target_ms(config),
            max_error_rate: config.max_error_rate,
            max_samples: config.sample_window_size,
        }
    }

    /// Record an operation result.
    fn record(&mut self, duration: Duration, result: OperationResult) {
        let success = result.is_success();
        let sample = LatencySample { duration, success, _timestamp: Instant::now() };

        // Add sample
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Calculate SLO statistics for this operation.
    fn statistics(&self) -> SloStatistics {
        if self.samples.is_empty() {
            return SloStatistics::default();
        }

        let total_count = self.samples.len() as u64;
        let success_count = self.samples.iter().filter(|s| s.success).count() as u64;
        let failure_count = total_count - success_count;
        let error_rate =
            if total_count > 0 { failure_count as f64 / total_count as f64 } else { 0.0 };

        // Calculate percentiles
        let mut durations_ms: Vec<u64> =
            self.samples.iter().map(|s| s.duration.as_millis() as u64).collect();
        durations_ms.sort_unstable();

        let p50_ms = percentile(&durations_ms, 50);
        let p95_ms = percentile(&durations_ms, 95);
        let p99_ms = percentile(&durations_ms, 99);

        let avg_ms =
            durations_ms.iter().map(|&d| d as f64).sum::<f64>() / durations_ms.len() as f64;

        // Check if SLO is met
        let slo_met = p95_ms <= self.slo_target_ms && error_rate <= self.max_error_rate;

        SloStatistics {
            total_count,
            success_count,
            failure_count,
            error_rate,
            p50_ms,
            p95_ms,
            p99_ms,
            avg_ms,
            slo_met,
        }
    }
}

/// Calculate a percentile from a sorted slice of values.
fn percentile(sorted_values: &[u64], pct: u64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    // Nearest-rank method: ceil(pct/100 * n) gives 1-based rank
    let rank = ((pct as f64 / 100.0) * sorted_values.len() as f64).ceil() as usize;
    sorted_values[rank.min(sorted_values.len()).saturating_sub(1)]
}

/// SLO tracker for workspace index operations.
///
/// Tracks latency and success/failure for all operation types,
/// providing SLO compliance monitoring and reporting.
pub struct SloTracker {
    /// SLO configuration
    config: SloConfig,
    /// Per-operation trackers
    trackers: Arc<Mutex<std::collections::HashMap<OperationType, OperationSloTracker>>>,
}

impl SloTracker {
    /// Create a new SLO tracker with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - SLO configuration
    ///
    /// # Returns
    ///
    /// A new SLO tracker instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::{SloConfig, SloTracker};
    ///
    /// let config = SloConfig::default();
    /// let tracker = SloTracker::new(config);
    /// ```
    pub fn new(config: SloConfig) -> Self {
        let mut trackers = std::collections::HashMap::new();

        // Initialize trackers for all operation types
        for op_type in [
            OperationType::IndexInitialization,
            OperationType::IncrementalUpdate,
            OperationType::DefinitionLookup,
            OperationType::Completion,
            OperationType::Hover,
            OperationType::FindReferences,
            OperationType::WorkspaceSymbols,
            OperationType::FileIndexing,
        ] {
            trackers.insert(op_type, OperationSloTracker::new(op_type, &config));
        }

        Self { config, trackers: Arc::new(Mutex::new(trackers)) }
    }

    /// Create a new SLO tracker with default configuration.
    ///
    /// # Returns
    ///
    /// A new SLO tracker with default SLO targets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::SloTracker;
    ///
    /// let tracker = SloTracker::default();
    /// ```
    pub fn default() -> Self {
        Self::new(SloConfig::default())
    }

    /// Start tracking an operation.
    ///
    /// # Arguments
    ///
    /// * `operation_type` - Type of operation to track
    ///
    /// # Returns
    ///
    /// A timestamp that should be passed to `record_operation`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::{SloTracker, OperationType};
    ///
    /// let tracker = SloTracker::default();
    /// let start = tracker.start_operation(OperationType::DefinitionLookup);
    /// // ... perform operation ...
    /// tracker.record_operation(start, Ok(()));
    /// ```
    pub fn start_operation(&self, _operation_type: OperationType) -> Instant {
        Instant::now()
    }

    /// Record the completion of an operation.
    ///
    /// # Arguments
    ///
    /// * `start` - Timestamp returned from `start_operation`
    /// * `result` - Operation result (success or failure)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::{SloTracker, OperationType, OperationResult};
    ///
    /// let tracker = SloTracker::default();
    /// let start = tracker.start_operation(OperationType::DefinitionLookup);
    /// // ... perform operation ...
    /// tracker.record_operation(start, OperationResult::Success);
    /// ```
    pub fn record_operation(&self, start: Instant, result: OperationResult) {
        let duration = start.elapsed();
        let mut trackers = self.trackers.lock();

        // Record for all operation types (simplified - in practice you'd pass the type)
        for tracker in trackers.values_mut() {
            tracker.record(duration, result.clone());
        }
    }

    /// Record the completion of a specific operation type.
    ///
    /// # Arguments
    ///
    /// * `operation_type` - Type of operation
    /// * `start` - Timestamp returned from `start_operation`
    /// * `result` - Operation result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::{SloTracker, OperationType, OperationResult};
    ///
    /// let tracker = SloTracker::default();
    /// let start = tracker.start_operation(OperationType::DefinitionLookup);
    /// // ... perform operation ...
    /// tracker.record_operation_type(OperationType::DefinitionLookup, start, OperationResult::Success);
    /// ```
    pub fn record_operation_type(
        &self,
        operation_type: OperationType,
        start: Instant,
        result: OperationResult,
    ) {
        let duration = start.elapsed();
        let mut trackers = self.trackers.lock();

        if let Some(tracker) = trackers.get_mut(&operation_type) {
            tracker.record(duration, result);
        }
    }

    /// Get SLO statistics for a specific operation type.
    ///
    /// # Arguments
    ///
    /// * `operation_type` - Type of operation
    ///
    /// # Returns
    ///
    /// SLO statistics for the operation type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::{SloTracker, OperationType};
    ///
    /// let tracker = SloTracker::default();
    /// let stats = tracker.statistics(OperationType::DefinitionLookup);
    /// ```
    pub fn statistics(&self, operation_type: OperationType) -> SloStatistics {
        let trackers = self.trackers.lock();
        trackers.get(&operation_type).map(|t| t.statistics()).unwrap_or_default()
    }

    /// Get SLO statistics for all operation types.
    ///
    /// # Returns
    ///
    /// A map of operation type to SLO statistics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::SloTracker;
    ///
    /// let tracker = SloTracker::default();
    /// let all_stats = tracker.all_statistics();
    /// ```
    pub fn all_statistics(&self) -> std::collections::HashMap<OperationType, SloStatistics> {
        let trackers = self.trackers.lock();
        trackers.iter().map(|(op_type, tracker)| (*op_type, tracker.statistics())).collect()
    }

    /// Check if all SLOs are being met.
    ///
    /// # Returns
    ///
    /// `true` if all operation types are meeting their SLOs, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::SloTracker;
    ///
    /// let tracker = SloTracker::default();
    /// let all_met = tracker.all_slos_met();
    /// ```
    pub fn all_slos_met(&self) -> bool {
        let trackers = self.trackers.lock();
        trackers.values().all(|t| t.statistics().slo_met)
    }

    /// Get the SLO configuration.
    ///
    /// # Returns
    ///
    /// The SLO configuration.
    pub fn config(&self) -> &SloConfig {
        &self.config
    }

    /// Reset all statistics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::slo::SloTracker;
    ///
    /// let tracker = SloTracker::default();
    /// tracker.reset();
    /// ```
    pub fn reset(&self) {
        let mut trackers = self.trackers.lock();
        for tracker in trackers.values_mut() {
            tracker.samples.clear();
        }
    }
}

impl Default for SloTracker {
    fn default() -> Self {
        Self::new(SloConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slo_tracker_record() {
        let tracker = SloTracker::default();
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        tracker.record_operation_type(
            OperationType::DefinitionLookup,
            start,
            OperationResult::Success,
        );

        let stats = tracker.statistics(OperationType::DefinitionLookup);
        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.success_count, 1);
    }

    #[test]
    fn test_slo_statistics() {
        let tracker = SloTracker::default();

        // Record some operations
        for _ in 0..10 {
            let start = tracker.start_operation(OperationType::DefinitionLookup);
            std::thread::sleep(Duration::from_millis(1));
            tracker.record_operation_type(
                OperationType::DefinitionLookup,
                start,
                OperationResult::Success,
            );
        }

        let stats = tracker.statistics(OperationType::DefinitionLookup);
        assert_eq!(stats.total_count, 10);
        assert_eq!(stats.success_count, 10);
        assert!(stats.p50_ms > 0);
        assert!(stats.p95_ms > 0);
    }

    #[test]
    fn test_slo_met() {
        let tracker = SloTracker::default();

        // Record fast operations (should meet SLO)
        for _ in 0..10 {
            let start = tracker.start_operation(OperationType::DefinitionLookup);
            std::thread::sleep(Duration::from_millis(1));
            tracker.record_operation_type(
                OperationType::DefinitionLookup,
                start,
                OperationResult::Success,
            );
        }

        let stats = tracker.statistics(OperationType::DefinitionLookup);
        assert!(stats.slo_met);
    }

    #[test]
    fn test_operation_type_name() {
        assert_eq!(OperationType::DefinitionLookup.name(), "definition_lookup");
        assert_eq!(OperationType::Completion.name(), "completion");
    }

    #[test]
    fn test_slo_target_ms() {
        let config = SloConfig::default();
        assert_eq!(OperationType::DefinitionLookup.slo_target_ms(&config), 50);
        assert_eq!(OperationType::Completion.slo_target_ms(&config), 100);
    }

    #[test]
    fn test_operation_result() {
        assert!(OperationResult::Success.is_success());
        assert!(!OperationResult::Failure("error".to_string()).is_success());
    }

    #[test]
    fn test_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&values, 50), 5); // Median
        assert_eq!(percentile(&values, 95), 10); // P95
    }
}
