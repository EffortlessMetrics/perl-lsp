//! Monitoring, limits, and lifecycle instrumentation primitives for workspace index coordination.
//!
//! This microcrate extracts coordinator support types from the main workspace index
//! implementation so the indexing engine can stay focused on symbol extraction and
//! query behavior.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Build phase while the index is in `Building` state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexPhase {
    /// No scan has started yet.
    Idle,
    /// Workspace file discovery is in progress.
    Scanning,
    /// Symbol indexing is in progress.
    Indexing,
}

/// Coarse index state kinds for instrumentation and transition tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexStateKind {
    /// Index is being built.
    Building,
    /// Index is ready for full queries.
    Ready,
    /// Index is degraded and serving partial results.
    Degraded,
}

/// A state transition for index lifecycle instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexStateTransition {
    /// Transition start state.
    pub from: IndexStateKind,
    /// Transition end state.
    pub to: IndexStateKind,
}

/// A phase transition while building the workspace index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexPhaseTransition {
    /// Transition start phase.
    pub from: IndexPhase,
    /// Transition end phase.
    pub to: IndexPhase,
}

/// Early-exit reasons for workspace indexing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EarlyExitReason {
    /// Initial scan exceeded the configured time budget.
    InitialTimeBudget,
    /// Incremental update exceeded the configured time budget.
    IncrementalTimeBudget,
    /// Workspace contained too many files to index within limits.
    FileLimit,
}

/// Record describing the latest early-exit event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EarlyExitRecord {
    /// Why the early exit occurred.
    pub reason: EarlyExitReason,
    /// Elapsed time in milliseconds when the exit occurred.
    pub elapsed_ms: u64,
    /// Files indexed when the exit occurred.
    pub indexed_files: usize,
    /// Total files discovered when the exit occurred.
    pub total_files: usize,
}

/// Snapshot of index lifecycle instrumentation.
#[derive(Clone, Debug)]
pub struct IndexInstrumentationSnapshot {
    /// Accumulated time spent per state (milliseconds).
    pub state_durations_ms: HashMap<IndexStateKind, u64>,
    /// Accumulated time spent per build phase (milliseconds).
    pub phase_durations_ms: HashMap<IndexPhase, u64>,
    /// Counts of state transitions.
    pub state_transition_counts: HashMap<IndexStateTransition, u64>,
    /// Counts of phase transitions.
    pub phase_transition_counts: HashMap<IndexPhaseTransition, u64>,
    /// Counts of early exit reasons.
    pub early_exit_counts: HashMap<EarlyExitReason, u64>,
    /// Most recent early exit record.
    pub last_early_exit: Option<EarlyExitRecord>,
}

/// Type of resource limit that was exceeded.
#[derive(Clone, Debug, PartialEq)]
pub enum ResourceKind {
    /// Maximum number of files in index exceeded.
    MaxFiles,
    /// Maximum total symbols exceeded.
    MaxSymbols,
    /// Maximum AST cache bytes exceeded.
    MaxCacheBytes,
}

/// Reason for index degradation.
#[derive(Clone, Debug)]
pub enum DegradationReason {
    /// Parse storm (too many simultaneous changes).
    ParseStorm {
        /// Number of pending parse operations.
        pending_parses: usize,
    },
    /// IO error during indexing.
    IoError {
        /// Error message for diagnostics.
        message: String,
    },
    /// Timeout during workspace scan.
    ScanTimeout {
        /// Elapsed time in milliseconds.
        elapsed_ms: u64,
    },
    /// Resource limits exceeded.
    ResourceLimit {
        /// Which resource limit was exceeded.
        kind: ResourceKind,
    },
}

/// Configurable resource limits for workspace index.
#[derive(Clone, Debug)]
pub struct IndexResourceLimits {
    /// Maximum files to index (default: 10,000).
    pub max_files: usize,
    /// Maximum symbols per file (default: 5,000).
    pub max_symbols_per_file: usize,
    /// Maximum total symbols (default: 500,000).
    pub max_total_symbols: usize,
    /// Maximum AST cache size in bytes (default: 256MB).
    pub max_ast_cache_bytes: usize,
    /// Maximum AST cache items (default: 100).
    pub max_ast_cache_items: usize,
    /// Maximum workspace scan duration in milliseconds (default: 30,000ms = 30s).
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
    /// Initial workspace scan budget in milliseconds (default: 500ms).
    pub initial_scan_budget_ms: u64,
    /// Incremental update budget in milliseconds (default: 10ms).
    pub incremental_budget_ms: u64,
}

impl Default for IndexPerformanceCaps {
    fn default() -> Self {
        Self { initial_scan_budget_ms: 500, incremental_budget_ms: 10 }
    }
}

/// Metrics for index lifecycle management and degradation detection.
pub struct IndexMetrics {
    pending_parses: AtomicUsize,
    parse_storm_threshold: usize,
    #[allow(dead_code)]
    last_indexed: AtomicU64,
}

impl IndexMetrics {
    /// Create new metrics with default threshold (10 pending parses).
    pub fn new() -> Self {
        Self {
            pending_parses: AtomicUsize::new(0),
            parse_storm_threshold: 10,
            last_indexed: AtomicU64::new(0),
        }
    }

    /// Create new metrics with custom parse storm threshold.
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            pending_parses: AtomicUsize::new(0),
            parse_storm_threshold: threshold,
            last_indexed: AtomicU64::new(0),
        }
    }

    /// Get current pending parse count (lock-free).
    pub fn pending_count(&self) -> usize {
        self.pending_parses.load(Ordering::SeqCst)
    }

    /// Increment pending parse count and return the new value.
    pub fn increment_pending_parses(&self) -> usize {
        self.pending_parses.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement pending parse count and return the new value.
    pub fn decrement_pending_parses(&self) -> usize {
        self.pending_parses.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Determine whether the current pending parse count exceeds the threshold.
    pub fn is_parse_storm(&self) -> bool {
        self.pending_count() > self.parse_storm_threshold
    }

    /// Get the parse-storm threshold.
    pub fn parse_storm_threshold(&self) -> usize {
        self.parse_storm_threshold
    }
}

impl Default for IndexMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct IndexInstrumentationState {
    current_state: IndexStateKind,
    current_phase: IndexPhase,
    state_started_at: Instant,
    phase_started_at: Instant,
    state_durations_ms: HashMap<IndexStateKind, u64>,
    phase_durations_ms: HashMap<IndexPhase, u64>,
    state_transition_counts: HashMap<IndexStateTransition, u64>,
    phase_transition_counts: HashMap<IndexPhaseTransition, u64>,
    early_exit_counts: HashMap<EarlyExitReason, u64>,
    last_early_exit: Option<EarlyExitRecord>,
}

impl IndexInstrumentationState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            current_state: IndexStateKind::Building,
            current_phase: IndexPhase::Idle,
            state_started_at: now,
            phase_started_at: now,
            state_durations_ms: HashMap::new(),
            phase_durations_ms: HashMap::new(),
            state_transition_counts: HashMap::new(),
            phase_transition_counts: HashMap::new(),
            early_exit_counts: HashMap::new(),
            last_early_exit: None,
        }
    }
}

/// Index lifecycle instrumentation for state durations and transitions.
#[derive(Debug)]
pub struct IndexInstrumentation {
    inner: Mutex<IndexInstrumentationState>,
}

impl IndexInstrumentation {
    /// Create a new instrumentation tracker.
    pub fn new() -> Self {
        Self { inner: Mutex::new(IndexInstrumentationState::new()) }
    }

    /// Record a state transition.
    pub fn record_state_transition(&self, from: IndexStateKind, to: IndexStateKind) {
        let now = Instant::now();
        let mut inner = self.inner.lock();
        let elapsed_ms = now.duration_since(inner.state_started_at).as_millis() as u64;
        *inner.state_durations_ms.entry(from).or_default() += elapsed_ms;

        let transition = IndexStateTransition { from, to };
        *inner.state_transition_counts.entry(transition).or_default() += 1;

        if from == IndexStateKind::Building {
            let phase_elapsed = now.duration_since(inner.phase_started_at).as_millis() as u64;
            let current_phase = inner.current_phase;
            *inner.phase_durations_ms.entry(current_phase).or_default() += phase_elapsed;
        }

        inner.current_state = to;
        inner.state_started_at = now;

        if to == IndexStateKind::Building || from == IndexStateKind::Building {
            inner.current_phase = IndexPhase::Idle;
            inner.phase_started_at = now;
        }
    }

    /// Record a build-phase transition.
    pub fn record_phase_transition(&self, from: IndexPhase, to: IndexPhase) {
        let now = Instant::now();
        let mut inner = self.inner.lock();
        let elapsed_ms = now.duration_since(inner.phase_started_at).as_millis() as u64;
        *inner.phase_durations_ms.entry(from).or_default() += elapsed_ms;

        let transition = IndexPhaseTransition { from, to };
        *inner.phase_transition_counts.entry(transition).or_default() += 1;

        inner.current_phase = to;
        inner.phase_started_at = now;
    }

    /// Record an early-exit event.
    pub fn record_early_exit(&self, record: EarlyExitRecord) {
        let mut inner = self.inner.lock();
        *inner.early_exit_counts.entry(record.reason).or_default() += 1;
        inner.last_early_exit = Some(record);
    }

    /// Return a current snapshot including elapsed time in the active state/phase.
    pub fn snapshot(&self) -> IndexInstrumentationSnapshot {
        let now = Instant::now();
        let inner = self.inner.lock();
        let mut state_durations_ms = inner.state_durations_ms.clone();
        let mut phase_durations_ms = inner.phase_durations_ms.clone();

        let state_elapsed = now.duration_since(inner.state_started_at).as_millis() as u64;
        *state_durations_ms.entry(inner.current_state).or_default() += state_elapsed;

        if inner.current_state == IndexStateKind::Building {
            let phase_elapsed = now.duration_since(inner.phase_started_at).as_millis() as u64;
            *phase_durations_ms.entry(inner.current_phase).or_default() += phase_elapsed;
        }

        IndexInstrumentationSnapshot {
            state_durations_ms,
            phase_durations_ms,
            state_transition_counts: inner.state_transition_counts.clone(),
            phase_transition_counts: inner.phase_transition_counts.clone(),
            early_exit_counts: inner.early_exit_counts.clone(),
            last_early_exit: inner.last_early_exit.clone(),
        }
    }
}

impl Default for IndexInstrumentation {
    fn default() -> Self {
        Self::new()
    }
}
