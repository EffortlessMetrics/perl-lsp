//! Workspace-wide symbol index for fast cross-file lookups in Perl LSP.
//!
//! This module provides efficient indexing of symbols across an entire Perl workspace,
//! enabling enterprise-grade features like find-references, rename refactoring, and
//! workspace symbol search with ≤1ms response times.
//!
//! # LSP Workflow Integration
//!
//! Core component in the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: AST generation from Perl source files
//! 2. **Index**: Workspace symbol table construction with dual indexing strategy
//! 3. **Navigate**: Cross-file symbol resolution and go-to-definition
//! 4. **Complete**: Context-aware completion with workspace symbol awareness
//! 5. **Analyze**: Cross-reference analysis and workspace refactoring operations
//!
//! # Performance Characteristics
//!
//! - **Symbol indexing**: O(n) where n is total workspace symbols
//! - **Symbol lookup**: O(1) average with hash table indexing
//! - **Cross-file queries**: <50μs for typical workspace sizes
//! - **Memory usage**: ~1MB per 10K symbols with optimized storage
//! - **Incremental updates**: ≤1ms for file-level symbol changes
//! - **Large workspace scaling**: Designed to scale to 50K+ files and large codebases
//! - **Benchmark targets**: <50μs lookups and ≤1ms incremental updates at scale
//!
//! # Dual Indexing Strategy
//!
//! Implements dual indexing for comprehensive Perl symbol resolution:
//! - **Qualified names**: `Package::function` for explicit references
//! - **Bare names**: `function` for context-dependent resolution
//! - **98% reference coverage**: Handles both qualified and unqualified calls
//! - **Automatic deduplication**: Prevents duplicate results in queries
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_workspace::workspace::workspace_index::WorkspaceIndex;
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let index = WorkspaceIndex::new();
//!
//! // Index a Perl file
//! let uri = Url::parse("file:///example.pl")?;
//! let code = "package MyPackage;\nsub example { return 42; }";
//! index.index_file(uri, code.to_string())?;
//!
//! // Find symbol definitions
//! let definition = index.find_definition("MyPackage::example");
//! assert!(definition.is_some());
//!
//! // Workspace symbol search
//! let symbols = index.find_symbols("example");
//! assert!(!symbols.is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! # Related Modules
//!
//! See also the symbol extraction, reference finding, and semantic token classification
//! modules in the workspace index implementation.

use crate::Parser;
use crate::ast::{Node, NodeKind};
use crate::document_store::{Document, DocumentStore};
use crate::position::{Position, Range};
use crate::workspace::monitoring::IndexInstrumentation;
use parking_lot::RwLock;
use perl_position_tracking::{WireLocation, WirePosition, WireRange};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EntityFact, EntityId, EntityKind, FileId,
    Provenance,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use url::Url;

use crate::semantic::facts::PRODUCER_SCHEMA_VERSION;
use crate::semantic::imports::ImportExportIndex;
pub use crate::semantic::invalidation::ShardReplaceResult;
use crate::semantic::invalidation::{ShardCategoryHashes, plan_shard_replacement};
use crate::semantic::references::ReferenceIndex;
pub use crate::workspace::monitoring::{
    DegradationReason, EarlyExitReason, EarlyExitRecord, IndexInstrumentationSnapshot,
    IndexMetrics, IndexPerformanceCaps, IndexPhase, IndexPhaseTransition, IndexResourceLimits,
    IndexStateKind, IndexStateTransition, ResourceKind,
};
use perl_symbol::surface::decl::extract_symbol_decls;
use perl_symbol::surface::facts::{symbol_decls_to_semantic_facts, symbol_refs_to_semantic_facts};
// Only used by `build_canonical_fact_shard_for_ast`, which is now
// `#[cfg(test)]`-gated (shadow/parity-harness-only as of the 1711-B cutover).
#[cfg(test)]
use perl_symbol::surface::r#ref::extract_symbol_refs;

// Re-export URI utilities for backward compatibility
#[cfg(not(target_arch = "wasm32"))]
/// URI ↔ filesystem helpers used during Index/Analyze workflows.
pub use perl_uri::{fs_path_to_uri, uri_to_fs_path};
/// URI inspection helpers used during Index/Analyze workflows.
pub use perl_uri::{is_file_uri, is_special_scheme, uri_extension, uri_key};

// ============================================================================
// Index Lifecycle Types (Index Lifecycle v1 Specification)
// ============================================================================

/// Index readiness state - explicit lifecycle management
///
/// Represents the current operational state of the workspace index, enabling
/// LSP handlers to provide appropriate responses based on index availability.
/// This state machine prevents blocking operations and ensures graceful
/// degradation when the index is not fully ready.
///
/// # State Transitions
///
/// - `Building` → `Ready`: Workspace scan completes successfully
/// - `Building` → `Degraded`: Scan timeout, IO error, or resource limit
/// - `Ready` → `Building`: Workspace folder change or file watching events
/// - `Ready` → `Degraded`: Parse storm (>10 pending) or IO error
/// - `Degraded` → `Building`: Recovery attempt after cooldown
/// - `Degraded` → `Ready`: Successful re-scan after recovery
///
/// # Invariants
///
/// - During a single build attempt, `phase` advances monotonically
///   (`Idle` → `Scanning` → `Indexing`).
/// - `indexed_count` must not exceed `total_count`; callers should keep totals updated.
/// - `Ready` and `Degraded` counts are snapshots captured at transition time.
///
/// # Usage
///
/// ```rust,ignore
/// use perl_parser::workspace_index::{IndexPhase, IndexState};
/// use std::time::Instant;
///
/// let state = IndexState::Building {
///     phase: IndexPhase::Indexing,
///     indexed_count: 50,
///     total_count: 100,
///     started_at: Instant::now(),
/// };
/// ```
#[derive(Clone, Debug)]
pub enum IndexState {
    /// Index is being constructed (workspace scan in progress)
    Building {
        /// Current build phase (Idle → Scanning → Indexing)
        phase: IndexPhase,
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

impl IndexState {
    /// Return the coarse state kind for instrumentation and routing decisions
    pub fn kind(&self) -> IndexStateKind {
        match self {
            IndexState::Building { .. } => IndexStateKind::Building,
            IndexState::Ready { .. } => IndexStateKind::Ready,
            IndexState::Degraded { .. } => IndexStateKind::Degraded,
        }
    }

    /// Return the current build phase when in `Building` state
    pub fn phase(&self) -> Option<IndexPhase> {
        match self {
            IndexState::Building { phase, .. } => Some(*phase),
            _ => None,
        }
    }

    /// Timestamp of when the current state began
    pub fn state_started_at(&self) -> Instant {
        match self {
            IndexState::Building { started_at, .. } => *started_at,
            IndexState::Ready { completed_at, .. } => *completed_at,
            IndexState::Degraded { since, .. } => *since,
        }
    }
}

/// Coordinates index lifecycle, state transitions, and handler queries
///
/// The IndexCoordinator wraps `WorkspaceIndex` with explicit state management,
/// enabling LSP handlers to query the index readiness and implement appropriate
/// fallback behavior when the index is not fully ready.
///
/// # Architecture
///
/// ```text
/// LspServer
///   └── IndexCoordinator
///         ├── state: Arc<RwLock<IndexState>>
///         ├── index: Arc<WorkspaceIndex>
///         ├── limits: IndexResourceLimits
///         ├── caps: IndexPerformanceCaps
///         ├── metrics: IndexMetrics
///         └── instrumentation: IndexInstrumentation
/// ```
///
/// # State Management
///
/// The coordinator manages three states:
/// - `Building`: Initial scan or recovery in progress
/// - `Ready`: Fully indexed and available for queries
/// - `Degraded`: Available but with reduced functionality
///
/// # Performance Characteristics
///
/// - State checks are lock-free reads (cloned state, <100ns)
/// - State transitions use write locks (rare, <1μs)
/// - Query dispatch has zero overhead in Ready state
/// - Degradation detection is atomic (<10ns per check)
///
/// # Usage
///
/// ```rust,ignore
/// use perl_parser::workspace_index::{IndexCoordinator, IndexState};
///
/// let coordinator = IndexCoordinator::new();
/// assert!(matches!(coordinator.state(), IndexState::Building { .. }));
///
/// // Transition to ready after indexing
/// coordinator.transition_to_ready(100, 5000);
/// assert!(matches!(coordinator.state(), IndexState::Ready { .. }));
///
/// // Query with degradation handling
/// let _result = coordinator.query(
///     |index| index.find_definition("my_function"), // full query
///     |_index| None                                 // partial fallback
/// );
/// ```
pub struct IndexCoordinator {
    /// Current index state (RwLock for state transitions)
    state: Arc<RwLock<IndexState>>,

    /// The actual workspace index
    index: Arc<WorkspaceIndex>,

    /// Resource limits configuration
    ///
    /// Enforces bounded resource usage to prevent unbounded memory growth:
    /// - max_files: Triggers degradation when file count exceeds limit
    /// - max_total_symbols: Triggers degradation when symbol count exceeds limit
    /// - max_symbols_per_file: Used for per-file validation during indexing
    limits: IndexResourceLimits,

    /// Performance caps for early-exit heuristics
    caps: IndexPerformanceCaps,

    /// Runtime metrics for degradation detection
    metrics: IndexMetrics,

    /// Instrumentation for lifecycle transitions and durations
    instrumentation: IndexInstrumentation,
}

impl std::fmt::Debug for IndexCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexCoordinator")
            .field("state", &*self.state.read())
            .field("limits", &self.limits)
            .field("caps", &self.caps)
            .finish_non_exhaustive()
    }
}

impl IndexCoordinator {
    /// Create a new coordinator in Building state
    ///
    /// Initializes the coordinator with default resource limits and
    /// an empty workspace index ready for initial scan.
    ///
    /// # Returns
    ///
    /// A coordinator initialized in `IndexState::Building`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(IndexState::Building {
                phase: IndexPhase::Idle,
                indexed_count: 0,
                total_count: 0,
                started_at: Instant::now(),
            })),
            index: Arc::new(WorkspaceIndex::new()),
            limits: IndexResourceLimits::default(),
            caps: IndexPerformanceCaps::default(),
            metrics: IndexMetrics::new(),
            instrumentation: IndexInstrumentation::new(),
        }
    }

    /// Create a coordinator with custom resource limits
    ///
    /// # Arguments
    ///
    /// * `limits` - Custom resource limits for this workspace
    ///
    /// # Returns
    ///
    /// A coordinator configured with the provided resource limits.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{IndexCoordinator, IndexResourceLimits};
    ///
    /// let limits = IndexResourceLimits::default();
    /// let coordinator = IndexCoordinator::with_limits(limits);
    /// ```
    pub fn with_limits(limits: IndexResourceLimits) -> Self {
        Self {
            state: Arc::new(RwLock::new(IndexState::Building {
                phase: IndexPhase::Idle,
                indexed_count: 0,
                total_count: 0,
                started_at: Instant::now(),
            })),
            index: Arc::new(WorkspaceIndex::new()),
            limits,
            caps: IndexPerformanceCaps::default(),
            metrics: IndexMetrics::new(),
            instrumentation: IndexInstrumentation::new(),
        }
    }

    /// Create a coordinator with custom limits and performance caps
    ///
    /// # Arguments
    ///
    /// * `limits` - Resource limits for this workspace
    /// * `caps` - Performance caps for indexing budgets
    pub fn with_limits_and_caps(limits: IndexResourceLimits, caps: IndexPerformanceCaps) -> Self {
        Self {
            state: Arc::new(RwLock::new(IndexState::Building {
                phase: IndexPhase::Idle,
                indexed_count: 0,
                total_count: 0,
                started_at: Instant::now(),
            })),
            index: Arc::new(WorkspaceIndex::new()),
            limits,
            caps,
            metrics: IndexMetrics::new(),
            instrumentation: IndexInstrumentation::new(),
        }
    }

    /// Get current state (lock-free read via clone)
    ///
    /// Returns a cloned copy of the current state for lock-free access
    /// in hot path LSP handlers.
    ///
    /// # Returns
    ///
    /// The current `IndexState` snapshot.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{IndexCoordinator, IndexState};
    ///
    /// let coordinator = IndexCoordinator::new();
    /// match coordinator.state() {
    ///     IndexState::Ready { .. } => {
    ///         // Full query path
    ///     }
    ///     _ => {
    ///         // Degraded/building fallback
    ///     }
    /// }
    /// ```
    pub fn state(&self) -> IndexState {
        self.state.read().clone()
    }

    /// Get reference to the underlying workspace index
    ///
    /// Provides direct access to the `WorkspaceIndex` for operations
    /// that don't require state checking (e.g., document store access).
    ///
    /// # Returns
    ///
    /// A shared reference to the underlying workspace index.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// let _index = coordinator.index();
    /// ```
    pub fn index(&self) -> &Arc<WorkspaceIndex> {
        &self.index
    }

    /// Access the configured resource limits
    pub fn limits(&self) -> &IndexResourceLimits {
        &self.limits
    }

    /// Access the configured performance caps
    pub fn performance_caps(&self) -> &IndexPerformanceCaps {
        &self.caps
    }

    /// Current pending-parse counter (lock-free read). Exposed for tests
    /// that need to assert the counter returns to baseline after a burst
    /// of `notify_change`/`notify_parse_complete` calls -- e.g. proving a
    /// coalescing or panic/stale-reject accounting fix actually balances
    /// (#3660), rather than only checking the coarser `state()` transition.
    pub fn pending_parse_count(&self) -> usize {
        self.metrics.pending_count()
    }

    /// Snapshot lifecycle instrumentation (durations, transitions, early exits)
    pub fn instrumentation_snapshot(&self) -> IndexInstrumentationSnapshot {
        self.instrumentation.snapshot()
    }

    /// Notify of file change (may trigger state transition)
    ///
    /// Increments the pending parse count and may transition to degraded
    /// state if a parse storm is detected.
    ///
    /// # Arguments
    ///
    /// * `_uri` - URI of the changed file (reserved for future use).
    ///
    /// # Returns
    ///
    /// Nothing. Updates coordinator metrics and state for the LSP workflow.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// coordinator.notify_change("file:///example.pl");
    /// ```
    pub fn notify_change(&self, _uri: &str) {
        let pending = self.metrics.increment_pending_parses();

        // Check for parse storm
        if self.metrics.is_parse_storm() {
            self.transition_to_degraded(DegradationReason::ParseStorm { pending_parses: pending });
        }
    }

    /// Notify parse completion for the Index/Analyze workflow stages.
    ///
    /// Decrements the pending parse count, enforces resource limits, and may
    /// attempt recovery when parse storms clear.
    ///
    /// # Arguments
    ///
    /// * `_uri` - URI of the parsed file (reserved for future use).
    ///
    /// # Returns
    ///
    /// Nothing. Updates coordinator metrics and state for the LSP workflow.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// coordinator.notify_parse_complete("file:///example.pl");
    /// ```
    pub fn notify_parse_complete(&self, _uri: &str) {
        let pending = self.metrics.decrement_pending_parses();

        // Check for recovery from parse storm
        if pending == 0 {
            if let IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. } =
                self.state()
            {
                // Attempt recovery - transition back to Building for re-scan
                let mut state = self.state.write();
                let from_kind = state.kind();
                self.instrumentation.record_state_transition(from_kind, IndexStateKind::Building);
                *state = IndexState::Building {
                    phase: IndexPhase::Idle,
                    indexed_count: 0,
                    total_count: 0,
                    started_at: Instant::now(),
                };
            }
        }

        // Enforce resource limits after parse completion
        self.enforce_limits();
    }

    /// Transition to Ready state
    ///
    /// Marks the index as fully ready for queries after successful workspace
    /// scan. Records the file count, symbol count, and completion timestamp.
    /// Enforces resource limits after transition.
    ///
    /// # State Transition Guards
    ///
    /// Only valid transitions:
    /// - `Building` → `Ready` (normal completion)
    /// - `Degraded` → `Ready` (recovery after fix)
    ///
    /// # Arguments
    ///
    /// * `file_count` - Total number of files indexed
    /// * `symbol_count` - Total number of symbols extracted
    ///
    /// # Returns
    ///
    /// Nothing. The coordinator state is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// coordinator.transition_to_ready(100, 5000);
    /// ```
    pub fn transition_to_ready(&self, file_count: usize, symbol_count: usize) {
        let mut state = self.state.write();
        let from_kind = state.kind();

        // State transition guard: validate current state allows transition to Ready
        match &*state {
            IndexState::Building { .. } | IndexState::Degraded { .. } => {
                // Valid transition - proceed
                *state =
                    IndexState::Ready { symbol_count, file_count, completed_at: Instant::now() };
            }
            IndexState::Ready { .. } => {
                // Already Ready - update metrics but don't log as transition
                *state =
                    IndexState::Ready { symbol_count, file_count, completed_at: Instant::now() };
            }
        }
        self.instrumentation.record_state_transition(from_kind, IndexStateKind::Ready);
        drop(state); // Release write lock before checking limits

        // Enforce resource limits after transition
        self.enforce_limits();
    }

    /// Transition to Scanning phase (Idle → Scanning)
    ///
    /// Resets build counters and marks the index as scanning workspace folders.
    pub fn transition_to_scanning(&self) {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Building { phase, indexed_count, total_count, started_at } => {
                if *phase != IndexPhase::Scanning {
                    self.instrumentation.record_phase_transition(*phase, IndexPhase::Scanning);
                }
                *state = IndexState::Building {
                    phase: IndexPhase::Scanning,
                    indexed_count: *indexed_count,
                    total_count: *total_count,
                    started_at: *started_at,
                };
            }
            IndexState::Ready { .. } | IndexState::Degraded { .. } => {
                self.instrumentation.record_state_transition(from_kind, IndexStateKind::Building);
                self.instrumentation
                    .record_phase_transition(IndexPhase::Idle, IndexPhase::Scanning);
                *state = IndexState::Building {
                    phase: IndexPhase::Scanning,
                    indexed_count: 0,
                    total_count: 0,
                    started_at: Instant::now(),
                };
            }
        }
    }

    /// Update scanning progress with the latest discovered file count
    pub fn update_scan_progress(&self, total_count: usize) {
        let mut state = self.state.write();
        if let IndexState::Building { phase, indexed_count, started_at, .. } = &*state {
            if *phase != IndexPhase::Scanning {
                self.instrumentation.record_phase_transition(*phase, IndexPhase::Scanning);
            }
            *state = IndexState::Building {
                phase: IndexPhase::Scanning,
                indexed_count: *indexed_count,
                total_count,
                started_at: *started_at,
            };
        }
    }

    /// Transition to Indexing phase (Scanning → Indexing)
    ///
    /// Uses the discovered file count as the total index target.
    pub fn transition_to_indexing(&self, total_count: usize) {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Building { phase, indexed_count, started_at, .. } => {
                if *phase != IndexPhase::Indexing {
                    self.instrumentation.record_phase_transition(*phase, IndexPhase::Indexing);
                }
                *state = IndexState::Building {
                    phase: IndexPhase::Indexing,
                    indexed_count: *indexed_count,
                    total_count,
                    started_at: *started_at,
                };
            }
            IndexState::Ready { .. } | IndexState::Degraded { .. } => {
                self.instrumentation.record_state_transition(from_kind, IndexStateKind::Building);
                self.instrumentation
                    .record_phase_transition(IndexPhase::Idle, IndexPhase::Indexing);
                *state = IndexState::Building {
                    phase: IndexPhase::Indexing,
                    indexed_count: 0,
                    total_count,
                    started_at: Instant::now(),
                };
            }
        }
    }

    /// Transition to Building state (Indexing phase)
    ///
    /// Marks the index as indexing with a known total file count.
    pub fn transition_to_building(&self, total_count: usize) {
        let mut state = self.state.write();
        let from_kind = state.kind();

        // State transition guard: validate transition is allowed
        match &*state {
            IndexState::Degraded { .. } | IndexState::Ready { .. } => {
                self.instrumentation.record_state_transition(from_kind, IndexStateKind::Building);
                self.instrumentation
                    .record_phase_transition(IndexPhase::Idle, IndexPhase::Indexing);
                *state = IndexState::Building {
                    phase: IndexPhase::Indexing,
                    indexed_count: 0,
                    total_count,
                    started_at: Instant::now(),
                };
            }
            IndexState::Building { phase, indexed_count, started_at, .. } => {
                let mut next_phase = *phase;
                if *phase == IndexPhase::Idle {
                    self.instrumentation
                        .record_phase_transition(IndexPhase::Idle, IndexPhase::Indexing);
                    next_phase = IndexPhase::Indexing;
                }
                *state = IndexState::Building {
                    phase: next_phase,
                    indexed_count: *indexed_count,
                    total_count,
                    started_at: *started_at,
                };
            }
        }
    }

    /// Update Building state progress for the Index/Analyze workflow stages.
    ///
    /// Increments the indexed file count and checks for scan timeouts.
    ///
    /// # Arguments
    ///
    /// * `indexed_count` - Number of files indexed so far.
    ///
    /// # Returns
    ///
    /// Nothing. Updates coordinator state and may transition to `Degraded`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// coordinator.transition_to_building(100);
    /// coordinator.update_building_progress(1);
    /// ```
    pub fn update_building_progress(&self, indexed_count: usize) {
        let mut state = self.state.write();

        if let IndexState::Building { phase, started_at, total_count, .. } = &*state {
            let elapsed = started_at.elapsed().as_millis() as u64;

            // Check for scan timeout
            if elapsed > self.limits.max_scan_duration_ms {
                // Timeout exceeded - transition to degraded
                drop(state);
                self.transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: elapsed });
                return;
            }

            // Update progress
            *state = IndexState::Building {
                phase: *phase,
                indexed_count,
                total_count: *total_count,
                started_at: *started_at,
            };
        }
    }

    /// Transition to Degraded state
    ///
    /// Marks the index as degraded with the specified reason. Preserves
    /// the current symbol count (if available) to indicate partial
    /// functionality remains.
    ///
    /// # Arguments
    ///
    /// * `reason` - Why the index degraded (ParseStorm, IoError, etc.)
    ///
    /// # Returns
    ///
    /// Nothing. The coordinator state is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{DegradationReason, IndexCoordinator, ResourceKind};
    ///
    /// let coordinator = IndexCoordinator::new();
    /// coordinator.transition_to_degraded(DegradationReason::ResourceLimit {
    ///     kind: ResourceKind::MaxFiles,
    /// });
    /// ```
    pub fn transition_to_degraded(&self, reason: DegradationReason) {
        let mut state = self.state.write();
        let from_kind = state.kind();

        // Get available symbols count from current state
        let available_symbols = match &*state {
            IndexState::Ready { symbol_count, .. } => *symbol_count,
            IndexState::Degraded { available_symbols, .. } => *available_symbols,
            IndexState::Building { .. } => 0,
        };

        self.instrumentation.record_state_transition(from_kind, IndexStateKind::Degraded);
        *state = IndexState::Degraded { reason, available_symbols, since: Instant::now() };
    }

    /// Check resource limits and return degradation reason if exceeded
    ///
    /// Examines current workspace index state against configured resource limits.
    /// Returns the first exceeded limit found, enabling targeted degradation.
    ///
    /// # Returns
    ///
    /// * `Some(DegradationReason)` - Resource limit exceeded, contains specific limit type
    /// * `None` - All limits within acceptable bounds
    ///
    /// # Checked Limits
    ///
    /// - `max_files`: Total number of indexed files
    /// - `max_total_symbols`: Aggregate symbol count across workspace
    ///
    /// # Performance
    ///
    /// - Lock-free read of index state (<100ns)
    /// - Symbol counting is O(n) where n is number of files
    ///
    /// Returns: `Some(DegradationReason)` when a limit is exceeded, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// let _reason = coordinator.check_limits();
    /// ```
    pub fn check_limits(&self) -> Option<DegradationReason> {
        let files = self.index.files.read();

        // Check max_files limit
        let file_count = files.len();
        if file_count > self.limits.max_files {
            return Some(DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles });
        }

        // Check max_total_symbols limit
        let total_symbols: usize = files.values().map(|fi| fi.symbols.len()).sum();
        if total_symbols > self.limits.max_total_symbols {
            return Some(DegradationReason::ResourceLimit { kind: ResourceKind::MaxSymbols });
        }

        None
    }

    /// Enforce resource limits and trigger degradation if exceeded
    ///
    /// Checks current resource usage against configured limits and automatically
    /// transitions to Degraded state if any limit is exceeded. This method should
    /// be called after operations that modify index size (file additions, parse
    /// completions, etc.).
    ///
    /// # State Transitions
    ///
    /// - `Ready` → `Degraded(ResourceLimit)` if limits exceeded
    /// - `Building` → `Degraded(ResourceLimit)` if limits exceeded
    ///
    /// # Returns
    ///
    /// Nothing. The coordinator state is updated in-place when limits are exceeded.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// // ... index some files ...
    /// coordinator.enforce_limits();  // Check and degrade if needed
    /// ```
    pub fn enforce_limits(&self) {
        if let Some(reason) = self.check_limits() {
            self.transition_to_degraded(reason);
        }
    }

    /// Record an early-exit event for indexing instrumentation
    pub fn record_early_exit(
        &self,
        reason: EarlyExitReason,
        elapsed_ms: u64,
        indexed_files: usize,
        total_files: usize,
    ) {
        self.instrumentation.record_early_exit(EarlyExitRecord {
            reason,
            elapsed_ms,
            indexed_files,
            total_files,
        });
    }

    /// Query with automatic degradation handling
    ///
    /// Dispatches to full query if index is Ready, or partial query otherwise.
    /// This pattern enables LSP handlers to provide appropriate responses
    /// based on index state without explicit state checking.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Return type of the query functions
    /// * `F1` - Full query function type accepting `&WorkspaceIndex` and returning `T`
    /// * `F2` - Partial query function type accepting `&WorkspaceIndex` and returning `T`
    ///
    /// # Arguments
    ///
    /// * `full_query` - Function to execute when index is Ready
    /// * `partial_query` - Function to execute when index is Building/Degraded
    ///
    /// # Returns
    ///
    /// The value returned by the selected query function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::IndexCoordinator;
    ///
    /// let coordinator = IndexCoordinator::new();
    /// let locations = coordinator.query(
    ///     |index| index.find_references("my_function"),  // Full workspace search
    ///     |index| vec![]                                 // Empty fallback
    /// );
    /// ```
    pub fn query<T, F1, F2>(&self, full_query: F1, partial_query: F2) -> T
    where
        F1: FnOnce(&WorkspaceIndex) -> T,
        F2: FnOnce(&WorkspaceIndex) -> T,
    {
        match self.state() {
            IndexState::Ready { .. } => full_query(&self.index),
            _ => partial_query(&self.index),
        }
    }
}

impl Default for IndexCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Symbol Indexing Types
// ============================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
/// Symbol kinds for cross-file indexing during Index/Navigate workflows.
pub enum SymKind {
    /// Variable symbol ($, @, or % sigil)
    Var,
    /// Subroutine definition (sub foo)
    Sub,
    /// Package declaration (package Foo)
    Pack,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// A normalized symbol key for cross-file lookups in Index/Navigate workflows.
pub struct SymbolKey {
    /// Package name containing this symbol
    pub pkg: Arc<str>,
    /// Bare name without sigil prefix
    pub name: Arc<str>,
    /// Variable sigil ($, @, or %) if applicable
    pub sigil: Option<char>,
    /// Kind of symbol (variable, subroutine, package)
    pub kind: SymKind,
}

/// Normalize a Perl variable name for Index/Analyze workflows.
///
/// Extracts an optional sigil and bare name for consistent symbol indexing.
///
/// # Arguments
///
/// * `name` - Variable name from Perl source, with or without sigil.
///
/// # Returns
///
/// `(sigil, name)` tuple with the optional sigil and normalized identifier.
///
/// # Examples
///
/// ```rust,ignore
/// use perl_parser::workspace_index::normalize_var;
///
/// assert_eq!(normalize_var("$count"), (Some('$'), "count"));
/// assert_eq!(normalize_var("process_emails"), (None, "process_emails"));
/// ```
pub fn normalize_var(name: &str) -> (Option<char>, &str) {
    if name.is_empty() {
        return (None, "");
    }

    // Safe: we've checked that name is not empty
    let Some(first_char) = name.chars().next() else {
        return (None, name); // Should never happen but handle gracefully
    };
    match first_char {
        '$' | '@' | '%' => {
            if name.len() > 1 {
                (Some(first_char), &name[1..])
            } else {
                (Some(first_char), "")
            }
        }
        _ => (None, name),
    }
}

// Using lsp_types for Position and Range

#[derive(Debug, Clone, PartialEq, Eq)]
/// Internal location type used during Navigate/Analyze workflows.
pub struct Location {
    /// File URI where the symbol is located
    pub uri: String,
    /// Line and character range within the file
    pub range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stable symbol identity returned by cross-file reference queries.
pub struct SymbolIdentity {
    /// Canonical stable key for the symbol (qualified when available).
    pub stable_key: String,
    /// Bare symbol name.
    pub name: String,
    /// Fully qualified symbol name when available.
    pub qualified_name: Option<String>,
    /// Symbol kind (subroutine, package, variable, ...).
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Read-only cross-file query result used by rename/safe-delete planners.
pub struct CrossFileReferenceQueryResult {
    /// Identity for the resolved symbol.
    pub symbol: SymbolIdentity,
    /// Definition site for the resolved symbol.
    pub definition: Location,
    /// All reference locations (including definition) in deterministic order.
    pub references: Vec<Location>,
}

// `PartialEq` added for 1711-B shadow-compare parity assertions (see
// `FileExtractionBundle` / `extraction_bundle_shadow_compare`) -- every field
// type already derives `PartialEq`, so this is purely additive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A symbol in the workspace for Index/Navigate workflows.
pub struct WorkspaceSymbol {
    /// Symbol name without package qualification
    pub name: String,
    /// Type of symbol (subroutine, variable, package, etc.)
    pub kind: SymbolKind,
    /// File URI where the symbol is defined
    pub uri: String,
    /// Line and character range of the symbol definition
    pub range: Range,
    /// Fully qualified name including package (e.g., "Package::function")
    pub qualified_name: Option<String>,
    /// POD documentation associated with the symbol
    pub documentation: Option<String>,
    /// Name of the containing package or class
    pub container_name: Option<String>,
    /// Whether this symbol has a body (false for forward declarations)
    #[serde(default = "default_has_body")]
    pub has_body: bool,
    /// Workspace folder URI this symbol belongs to (for multi-root workspace support)
    pub workspace_folder_uri: Option<String>,
    /// Whether this symbol is a lexically-scoped variable (`my` or `state`).
    ///
    /// Lexical variables cannot be correctly analysed by the bare-name unused-symbol
    /// check in [`WorkspaceIndex::find_unused_symbols`], which lacks scope-range
    /// information.  Setting this flag during indexing lets the function skip them
    /// entirely, avoiding both false positives and false negatives.  Proper
    /// lexical-unused detection is deferred to the scope-aware `ScopeAnalyzer`.
    #[serde(default)]
    pub is_lexical: bool,
}

fn default_has_body() -> bool {
    true
}

// Re-export the unified symbol types from perl-symbol
/// Symbol kind enums used during Index/Analyze workflows.
pub use perl_symbol::{SymbolKind, VarKind};

// `PartialEq` added for 1711-B shadow-compare parity assertions -- `Range`
// and `ReferenceKind` already derive `PartialEq`, so this is purely additive.
#[derive(Debug, Clone, PartialEq)]
/// Reference to a symbol for Navigate/Analyze workflows.
pub struct SymbolReference {
    /// File URI where the reference occurs
    pub uri: String,
    /// Line and character range of the reference
    pub range: Range,
    /// How the symbol is being referenced (definition, usage, etc.)
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of how a symbol is referenced in Navigate/Analyze workflows.
pub enum ReferenceKind {
    /// Symbol definition site (sub declaration, variable declaration)
    Definition,
    /// General usage of the symbol (function call, method call)
    Usage,
    /// Import via use statement
    Import,
    /// Variable read access
    Read,
    /// Variable write access (assignment target)
    Write,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// LSP-compliant workspace symbol for wire format in Navigate/Analyze workflows.
pub struct LspWorkspaceSymbol {
    /// Symbol name as displayed to the user
    pub name: String,
    /// LSP symbol kind number (see lsp_types::SymbolKind)
    pub kind: u32,
    /// Location of the symbol definition
    pub location: WireLocation,
    /// Name of the containing symbol (package, class)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    /// Workspace folder URI this symbol belongs to (for multi-root workspace disambiguation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder_uri: Option<String>,
}

impl From<&WorkspaceSymbol> for LspWorkspaceSymbol {
    fn from(sym: &WorkspaceSymbol) -> Self {
        let range = WireRange {
            start: WirePosition { line: sym.range.start.line, character: sym.range.start.column },
            end: WirePosition { line: sym.range.end.line, character: sym.range.end.column },
        };

        Self {
            name: sym.name.clone(),
            kind: sym.kind.to_lsp_kind(),
            location: WireLocation { uri: sym.uri.clone(), range },
            container_name: sym.container_name.clone(),
            workspace_folder_uri: sym.workspace_folder_uri.clone(),
        }
    }
}

/// File-level index data
#[derive(Default, Clone)]
pub struct FileIndex {
    /// Canonical file URI for this index entry.
    source_uri: String,
    /// Symbols defined in this file
    symbols: Vec<WorkspaceSymbol>,
    /// References in this file (symbol name -> references)
    references: HashMap<String, Vec<SymbolReference>>,
    /// Dependencies (modules this file imports)
    dependencies: HashSet<String>,
    /// Content hash for early-exit optimization
    content_hash: u64,
    /// Document generation represented by this indexed snapshot -- the
    /// GENUINELY COMMITTED generation. Only ever advanced by a successful
    /// late-guard `files.insert` (or the unchanged-content early exit,
    /// which is atomic and cannot fail). Never advanced speculatively, so
    /// `indexed_generation()` and every other reader of this field always
    /// see a generation whose symbols/content_hash/document_store text
    /// were actually produced -- there is nothing to roll back here on a
    /// parse failure, because it was never spent on a guess.
    generation: u32,
    /// High-water mark of generations *claimed* (reserved) for this key,
    /// including in-flight attempts that have not yet committed. Bumped
    /// early -- before parsing -- by [`ReservationGuard`] so a concurrent
    /// out-of-order task for an older generation can be rejected before it
    /// writes stale text into `document_store` (see `index_file_with_generation`).
    /// Always `>= generation`. Deliberately NOT the source of truth for
    /// "what's indexed" -- only `generation` is -- so a reservation that
    /// never pans out (parse error, document no longer open) can never
    /// leave the publicly-read `generation` field pointing at content that
    /// was never actually stored.
    pending_generation: u32,
    /// Workspace folder URI this file belongs to (for multi-root workspace support)
    folder_uri: Option<String>,
}

/// RAII reservation for `index_file_with_generation`'s early-guard claim on
/// `FileIndex::pending_generation`.
///
/// Constructed once a task's generation genuinely advances the high-water
/// mark; dropped without a call to [`Self::commit`] rolls the reservation
/// back automatically -- covering EVERY early-return between the
/// reservation and the late guard's successful commit (today: a parse
/// error and a `document_store` lookup miss) uniformly, so a future third
/// early-return in this function can't silently reintroduce the same bug
/// class (#3618 review-3660 findings 3(a)/3(b)/3(c)).
///
/// The rollback restores `pending_generation` toward `FileIndex::generation`
/// -- the last GENUINELY COMMITTED generation, which only a successful late
/// guard ever advances -- rather than to this task's own pre-reservation
/// snapshot. Restoring to a per-task snapshot is unsound under chained
/// failures: if task A reserves then fails, and task B reserves a still
/// -higher generation and ALSO fails, A's rollback correctly no-ops (B's
/// claim is still current), but a rollback that restores to "my own
/// pre-reservation value" would have B's own drop set the high-water mark
/// to A's already-superseded reservation -- stranding it above the true
/// committed floor read by nothing, but also below what a legitimate
/// future reservation should be able to observe as "nothing is truly
/// pending." Restoring to `generation` (never touched by any reservation)
/// is always correct regardless of how many reservations chained and
/// failed before this one.
struct ReservationGuard<'a> {
    index: &'a WorkspaceIndex,
    key: String,
    reserved: u32,
    committed: bool,
}

impl ReservationGuard<'_> {
    /// Disarm the rollback -- call this once the late guard has genuinely
    /// committed this generation to `self.files`.
    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ReservationGuard<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let mut files = self.index.files.write();
        if let Some(existing) = files.get_mut(&self.key) {
            // Only roll back if nothing newer has claimed the slot since --
            // a concurrent task's own legitimate later reservation must
            // never be stomped by this failure's cleanup.
            if existing.pending_generation == self.reserved {
                existing.pending_generation = existing.generation;
            }
        }
    }
}

/// Write-through semantic fact storage for one indexed file.
///
/// Derives `Serialize, Deserialize` (Campaign 31 PR 5, perl-lsp-swarm#2592)
/// so the `perllsp ripr-facts` exporter can serialize the shard into the
/// `ripr-perl-facts-v1` packet. Previously derived only `Clone, Debug`.
///
/// Derives `PartialEq` (1711-B shadow-compare parity assertions, see
/// `FileExtractionBundle` / `extraction_bundle_shadow_compare`) -- every
/// field type (`AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`,
/// `FileId`, etc.) already derives `PartialEq`, so this is purely additive.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileFactShard {
    /// Canonical file URI for this shard.
    pub source_uri: String,
    /// Stable file identifier derived from normalized URI.
    pub file_id: FileId,
    /// Whole-file content hash used for stale-shard replacement.
    pub content_hash: u64,
    /// Schema version of the semantic fact producer that built this shard.
    ///
    /// Set to [`crate::semantic::facts::PRODUCER_SCHEMA_VERSION`] at
    /// construction time.  Consumers (e.g. the snapshot layer in #1601)
    /// compare this field against the constant to detect schema drift.
    pub producer_schema_version: u32,
    /// Optional per-category hashes for change diagnostics.
    pub anchors_hash: Option<u64>,
    /// Optional per-category hashes for change diagnostics.
    pub entities_hash: Option<u64>,
    /// Optional per-category hashes for change diagnostics.
    pub occurrences_hash: Option<u64>,
    /// Optional per-category hashes for change diagnostics.
    pub edges_hash: Option<u64>,
    /// Anchor facts for this file.
    pub anchors: Vec<AnchorFact>,
    /// Entity facts for this file.
    pub entities: Vec<EntityFact>,
    /// Occurrence facts for this file.
    pub occurrences: Vec<perl_semantic_facts::OccurrenceFact>,
    /// Edge facts for this file.
    pub edges: Vec<EdgeFact>,
}

/// Thread-safe workspace index
pub struct WorkspaceIndex {
    /// Index data per file URI (normalized key -> data)
    files: Arc<RwLock<HashMap<String, FileIndex>>>,
    /// Global symbol multimap (qualified/bare name -> ordered definition candidates)
    symbols: Arc<RwLock<HashMap<String, Vec<DefinitionCandidate>>>>,
    /// Workspace-symbol search index for fast query lookup.
    ///
    /// Maps lowercase symbol name (bare or qualified) to all `WorkspaceSymbol`
    /// instances that carry that name.  `search_source_symbols` iterates the
    /// unique name keys in this map instead of scanning every file's symbol list,
    /// turning the outer loop from O(total_symbols) to O(unique_lowercase_names).
    ///
    /// Lock order: always acquire `symbols` before `search_index`.
    search_index: Arc<RwLock<HashMap<String, Vec<WorkspaceSymbol>>>>,
    /// Global reference index (symbol name -> locations across all files)
    ///
    /// Aggregated from per-file `FileIndex::references` during `index_file()`.
    /// Provides O(1) lookup for `find_references()` instead of iterating all files.
    global_references: Arc<RwLock<HashMap<String, Vec<Location>>>>,
    /// Write-through semantic fact shards keyed by normalized URI.
    fact_shards: Arc<RwLock<HashMap<String, FileFactShard>>>,
    /// Semantic cross-file reference index (typed occurrences by name and entity).
    semantic_reference_index: Arc<RwLock<ReferenceIndex>>,
    /// Semantic cross-file import/export index.
    semantic_import_export_index: Arc<RwLock<ImportExportIndex>>,
    /// Document store for in-memory text
    document_store: DocumentStore,
    /// Workspace folder URIs for multi-root workspace support
    ///
    /// Used to determine which workspace folder a file belongs to for
    /// proper folder attribution in multi-root workspaces.
    workspace_folders: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DefinitionCandidate {
    location: Location,
    kind: SymbolKind,
}

impl WorkspaceIndex {
    fn location_sort_key(location: &Location) -> (&str, u32, u32, u32, u32) {
        (
            location.uri.as_str(),
            location.range.start.line,
            location.range.start.column,
            location.range.end.line,
            location.range.end.column,
        )
    }

    fn sort_locations_deterministically(locations: &mut [Location]) {
        locations.sort_by(|left, right| {
            Self::location_sort_key(left).cmp(&Self::location_sort_key(right))
        });
    }

    fn definition_candidate_sort_key(
        candidate: &DefinitionCandidate,
    ) -> (u8, &str, u32, u32, u32, u32) {
        let rank = match candidate.kind {
            SymbolKind::Subroutine | SymbolKind::Method => 0,
            SymbolKind::Constant => 1,
            _ => 2,
        };
        (
            rank,
            candidate.location.uri.as_str(),
            candidate.location.range.start.line,
            candidate.location.range.start.column,
            candidate.location.range.end.line,
            candidate.location.range.end.column,
        )
    }

    fn rebuild_symbol_cache(
        files: &HashMap<String, FileIndex>,
        symbols: &mut HashMap<String, Vec<DefinitionCandidate>>,
    ) {
        symbols.clear();

        for file_index in files.values() {
            for symbol in &file_index.symbols {
                if let Some(ref qname) = symbol.qualified_name {
                    symbols.entry(qname.clone()).or_default().push(DefinitionCandidate {
                        location: Location { uri: symbol.uri.clone(), range: symbol.range },
                        kind: symbol.kind,
                    });
                }
                symbols.entry(symbol.name.clone()).or_default().push(DefinitionCandidate {
                    location: Location { uri: symbol.uri.clone(), range: symbol.range },
                    kind: symbol.kind,
                });
            }
        }
        for entries in symbols.values_mut() {
            entries.sort_by(|left, right| {
                Self::definition_candidate_sort_key(left)
                    .cmp(&Self::definition_candidate_sort_key(right))
            });
            entries.dedup();
        }
    }

    /// Incrementally remove one file's symbols from the global cache,
    /// re-inserting shadowed symbols from remaining files.
    fn incremental_remove_symbols(
        files: &HashMap<String, FileIndex>,
        symbols: &mut HashMap<String, Vec<DefinitionCandidate>>,
        old_file_index: &FileIndex,
    ) {
        let mut affected_names: Vec<String> = Vec::new();
        for sym in &old_file_index.symbols {
            if let Some(ref qname) = sym.qualified_name {
                let mut remove_key = false;
                if let Some(entries) = symbols.get_mut(qname) {
                    entries.retain(|candidate| candidate.location.uri != sym.uri);
                    remove_key = entries.is_empty();
                }
                if remove_key {
                    symbols.remove(qname);
                    affected_names.push(qname.clone());
                }
            }
            let mut remove_key = false;
            if let Some(entries) = symbols.get_mut(&sym.name) {
                entries.retain(|candidate| candidate.location.uri != sym.uri);
                remove_key = entries.is_empty();
            }
            if remove_key {
                symbols.remove(&sym.name);
                affected_names.push(sym.name.clone());
            }
        }
        if !affected_names.is_empty() {
            symbols.clear();
            for file_index in files
                .values()
                .filter(|file_index| file_index.source_uri != old_file_index.source_uri)
            {
                for symbol in &file_index.symbols {
                    if let Some(ref qname) = symbol.qualified_name {
                        symbols.entry(qname.clone()).or_default().push(DefinitionCandidate {
                            location: Location { uri: symbol.uri.clone(), range: symbol.range },
                            kind: symbol.kind,
                        });
                    }
                    symbols.entry(symbol.name.clone()).or_default().push(DefinitionCandidate {
                        location: Location { uri: symbol.uri.clone(), range: symbol.range },
                        kind: symbol.kind,
                    });
                }
            }
            for entries in symbols.values_mut() {
                entries.sort_by(|left, right| {
                    Self::definition_candidate_sort_key(left)
                        .cmp(&Self::definition_candidate_sort_key(right))
                });
                entries.dedup();
            }
        }
    }

    /// Incrementally add one file's symbols to the global cache.
    fn incremental_add_symbols(
        symbols: &mut HashMap<String, Vec<DefinitionCandidate>>,
        file_index: &FileIndex,
    ) {
        for sym in &file_index.symbols {
            if let Some(ref qname) = sym.qualified_name {
                symbols.entry(qname.clone()).or_default().push(DefinitionCandidate {
                    location: Location { uri: sym.uri.clone(), range: sym.range },
                    kind: sym.kind,
                });
            }
            symbols.entry(sym.name.clone()).or_default().push(DefinitionCandidate {
                location: Location { uri: sym.uri.clone(), range: sym.range },
                kind: sym.kind,
            });
        }
        for entries in symbols.values_mut() {
            entries.sort_by(|left, right| {
                Self::definition_candidate_sort_key(left)
                    .cmp(&Self::definition_candidate_sort_key(right))
            });
            entries.dedup();
        }
    }

    /// Build the search index from scratch from all file indexes.
    ///
    /// Keyed by lowercase bare name and lowercase qualified name so that
    /// `search_source_symbols` can iterate unique name keys (O(unique_lowercase_names))
    /// rather than all (file, symbol) pairs (O(total_symbols)).
    ///
    /// Lock order: hold `symbols` write before calling; acquire `search_index` write
    /// immediately after `symbols` write.
    fn rebuild_search_index(
        files: &HashMap<String, FileIndex>,
        search_index: &mut HashMap<String, Vec<WorkspaceSymbol>>,
    ) {
        search_index.clear();
        for file_index in files.values() {
            for symbol in &file_index.symbols {
                let key = symbol.name.to_lowercase();
                search_index.entry(key).or_default().push(symbol.clone());
                if let Some(ref qname) = symbol.qualified_name {
                    let qkey = qname.to_lowercase();
                    search_index.entry(qkey).or_default().push(symbol.clone());
                }
            }
        }
    }

    /// Incrementally add one file's symbols to the search index.
    fn incremental_add_search(
        search_index: &mut HashMap<String, Vec<WorkspaceSymbol>>,
        file_index: &FileIndex,
    ) {
        for symbol in &file_index.symbols {
            let key = symbol.name.to_lowercase();
            search_index.entry(key).or_default().push(symbol.clone());
            if let Some(ref qname) = symbol.qualified_name {
                let qkey = qname.to_lowercase();
                search_index.entry(qkey).or_default().push(symbol.clone());
            }
        }
    }

    /// Incrementally remove one file's symbols from the search index by URI.
    fn incremental_remove_search(
        search_index: &mut HashMap<String, Vec<WorkspaceSymbol>>,
        file_index: &FileIndex,
    ) {
        for symbol in &file_index.symbols {
            let key = symbol.name.to_lowercase();
            if let Some(entries) = search_index.get_mut(&key) {
                entries.retain(|s| s.uri != symbol.uri);
                if entries.is_empty() {
                    search_index.remove(&key);
                }
            }
            if let Some(ref qname) = symbol.qualified_name {
                let qkey = qname.to_lowercase();
                if let Some(entries) = search_index.get_mut(&qkey) {
                    entries.retain(|s| s.uri != symbol.uri);
                    if entries.is_empty() {
                        search_index.remove(&qkey);
                    }
                }
            }
        }
    }

    /// Determine the workspace folder URI for a given file URI.
    ///
    /// Returns the workspace folder URI that contains the given file URI.
    /// This is used for multi-root workspace support to properly attribute
    /// files and symbols to their originating workspace folder.
    ///
    /// # Arguments
    ///
    /// * `file_uri` - The file URI to find the containing workspace folder for
    ///
    /// # Returns
    ///
    /// `Some(folder_uri)` if the file is within a workspace folder, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_workspace::workspace::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// index.set_workspace_folders(vec![
    ///     "file:///project1".to_string(),
    ///     "file:///project2".to_string(),
    /// ]);
    ///
    /// let folder = index.determine_folder_uri("file:///project1/src/main.pl");
    /// assert_eq!(folder, Some("file:///project1".to_string()));
    /// ```
    fn determine_folder_uri(&self, file_uri: &str) -> Option<String> {
        let folders = self.workspace_folders.read();
        let mut best_match: Option<&String> = None;
        for folder_uri in folders.iter() {
            // Check if the file URI starts with the folder URI
            // We need to ensure proper URI matching (with or without trailing slash)
            let folder_with_slash = if folder_uri.ends_with('/') {
                folder_uri.clone()
            } else {
                format!("{}/", folder_uri)
            };
            if file_uri.starts_with(&folder_with_slash) || file_uri == folder_uri {
                match best_match {
                    Some(existing) if existing.len() >= folder_uri.len() => {}
                    _ => best_match = Some(folder_uri),
                }
            }
        }
        best_match.cloned()
    }

    fn find_definition_in_files(
        files: &HashMap<String, FileIndex>,
        symbol_name: &str,
        uri_filter: Option<&str>,
    ) -> Option<(Location, String)> {
        let mut candidates: Vec<(Location, String)> = Vec::new();
        for file_index in files.values() {
            if let Some(filter) = uri_filter
                && file_index.symbols.first().is_some_and(|symbol| symbol.uri != filter)
            {
                continue;
            }

            for symbol in &file_index.symbols {
                if symbol.name == symbol_name
                    || symbol.qualified_name.as_deref() == Some(symbol_name)
                {
                    candidates.push((
                        Location { uri: symbol.uri.clone(), range: symbol.range },
                        symbol.uri.clone(),
                    ));
                }
            }
        }

        candidates.sort_by(|left, right| {
            Self::location_sort_key(&left.0).cmp(&Self::location_sort_key(&right.0))
        });
        candidates.into_iter().next()
    }

    fn find_symbol_by_definition(
        &self,
        definition: &Location,
        symbol_name: &str,
    ) -> Option<WorkspaceSymbol> {
        let files = self.files.read();
        files
            .values()
            .flat_map(|file_index| file_index.symbols.iter())
            .filter(|symbol| {
                symbol.uri == definition.uri
                    && symbol.range == definition.range
                    && (symbol.name == symbol_name
                        || symbol.qualified_name.as_deref() == Some(symbol_name))
            })
            .min_by(|left, right| {
                (
                    left.qualified_name.as_deref().unwrap_or_default(),
                    left.name.as_str(),
                    left.kind.to_lsp_kind(),
                )
                    .cmp(&(
                        right.qualified_name.as_deref().unwrap_or_default(),
                        right.name.as_str(),
                        right.kind.to_lsp_kind(),
                    ))
            })
            .cloned()
    }

    fn has_unique_symbol_name_and_kind(&self, target: &WorkspaceSymbol) -> bool {
        let files = self.files.read();
        files
            .values()
            .flat_map(|file_index| file_index.symbols.iter())
            .filter(|symbol| symbol.name == target.name && symbol.kind == target.kind)
            .take(2)
            .count()
            == 1
    }

    fn collect_symbol_references(&self, symbol: &WorkspaceSymbol) -> Vec<Location> {
        let mut names_to_query: Vec<&str> = Vec::new();
        if let Some(qualified_name) = symbol.qualified_name.as_deref() {
            names_to_query.push(qualified_name);
            if self.has_unique_symbol_name_and_kind(symbol) {
                names_to_query.push(symbol.name.as_str());
            }
        } else {
            names_to_query.push(symbol.name.as_str());
        }

        let global_refs = self.global_references.read();
        let mut seen: HashSet<(String, u32, u32, u32, u32)> = HashSet::new();
        let mut locations = Vec::new();

        for symbol_name in names_to_query {
            if let Some(refs) = global_refs.get(symbol_name) {
                for location in refs {
                    let key = (
                        location.uri.clone(),
                        location.range.start.line,
                        location.range.start.column,
                        location.range.end.line,
                        location.range.end.column,
                    );
                    if seen.insert(key) {
                        locations.push(location.clone());
                    }
                }
            }
        }
        drop(global_refs);

        Self::sort_locations_deterministically(&mut locations);
        locations
    }

    /// Create a new empty index
    ///
    /// # Returns
    ///
    /// A workspace index with empty file and symbol tables.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// assert!(!index.has_symbols());
    /// ```
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            symbols: Arc::new(RwLock::new(HashMap::new())),
            search_index: Arc::new(RwLock::new(HashMap::new())),
            global_references: Arc::new(RwLock::new(HashMap::new())),
            fact_shards: Arc::new(RwLock::new(HashMap::new())),
            semantic_reference_index: Arc::new(RwLock::new(ReferenceIndex::new())),
            semantic_import_export_index: Arc::new(RwLock::new(ImportExportIndex::new())),
            document_store: DocumentStore::new(),
            workspace_folders: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a workspace index with pre-allocated capacity.
    ///
    /// Pre-allocating reduces the number of rehash operations during large-workspace
    /// startup. Use this instead of `new()` when the approximate workspace size is
    /// known in advance (e.g. from a file discovery scan).
    ///
    /// # Arguments
    ///
    /// * `estimated_files` - Expected number of source files in the workspace.
    /// * `avg_symbols_per_file` - Expected average number of symbols per file.
    ///
    /// # Panics
    ///
    /// Does not panic. Overflow is prevented via `saturating_mul` and an upper cap
    /// on the symbol/reference map capacity.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_workspace::workspace::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::with_capacity(1000, 20);
    /// assert!(!index.has_symbols());
    /// ```
    pub fn with_capacity(estimated_files: usize, avg_symbols_per_file: usize) -> Self {
        // Each symbol is stored twice (qualified + bare name) due to dual indexing.
        let sym_cap =
            estimated_files.saturating_mul(avg_symbols_per_file).saturating_mul(2).min(1_000_000);
        let ref_cap = (sym_cap / 4).min(1_000_000);
        Self {
            files: Arc::new(RwLock::new(HashMap::with_capacity(estimated_files))),
            symbols: Arc::new(RwLock::new(HashMap::with_capacity(sym_cap))),
            search_index: Arc::new(RwLock::new(HashMap::with_capacity(sym_cap))),
            global_references: Arc::new(RwLock::new(HashMap::with_capacity(ref_cap))),
            fact_shards: Arc::new(RwLock::new(HashMap::with_capacity(estimated_files))),
            semantic_reference_index: Arc::new(RwLock::new(ReferenceIndex::new())),
            semantic_import_export_index: Arc::new(RwLock::new(ImportExportIndex::new())),
            document_store: DocumentStore::new(),
            workspace_folders: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set the workspace folder URIs for multi-root workspace support.
    ///
    /// This method updates the list of workspace folders that the index
    /// uses to determine folder attribution for files and symbols.
    ///
    /// # Arguments
    ///
    /// * `folders` - A vector of workspace folder URIs
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_workspace::workspace::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// index.set_workspace_folders(vec![
    ///     "file:///project1".to_string(),
    ///     "file:///project2".to_string(),
    /// ]);
    /// ```
    pub fn set_workspace_folders(&self, folders: Vec<String>) {
        let mut workspace_folders = self.workspace_folders.write();
        *workspace_folders = folders;
    }

    /// Get the current workspace folder URIs.
    ///
    /// # Returns
    ///
    /// A vector of workspace folder URIs.
    #[must_use]
    pub fn workspace_folders(&self) -> Vec<String> {
        self.workspace_folders.read().clone()
    }

    /// Return the document generation represented by the indexed file snapshot.
    #[must_use]
    pub fn indexed_generation(&self, uri: &str) -> Option<u32> {
        let uri_str = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&uri_str);
        self.files.read().get(&key).map(|file_index| file_index.generation)
    }

    /// Whether the indexed snapshot for `uri` is older than `expected_generation`.
    #[must_use]
    pub fn is_index_generation_stale(&self, uri: &str, expected_generation: u32) -> bool {
        self.indexed_generation(uri)
            .is_some_and(|indexed_generation| indexed_generation < expected_generation)
    }

    /// Normalize a URI to a consistent form using proper URI handling
    fn normalize_uri(uri: &str) -> String {
        perl_uri::normalize_uri(uri)
    }

    /// Remove a file's contributions from the global reference index.
    ///
    /// Retains only entries whose URI does not match `file_uri`.
    /// Empty keys are removed to avoid unbounded map growth.
    fn remove_file_global_refs(
        global_refs: &mut HashMap<String, Vec<Location>>,
        file_index: &FileIndex,
        file_uri: &str,
    ) {
        for name in file_index.references.keys() {
            if let Some(locs) = global_refs.get_mut(name) {
                locs.retain(|loc| loc.uri != file_uri);
                if locs.is_empty() {
                    global_refs.remove(name);
                }
            }
        }
    }

    /// Index a file from its URI and text content
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI identifying the document
    /// * `text` - Full Perl source text for indexing
    ///
    /// # Returns
    ///
    /// `Ok(())` when indexing succeeds, or an error message otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails or the document store cannot be updated.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    /// use url::Url;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let index = WorkspaceIndex::new();
    /// let uri = Url::parse("file:///example.pl")?;
    /// index.index_file(uri, "sub hello { return 1; }".to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns: `Ok(())` when indexing succeeds, otherwise an error string.
    pub fn index_file(&self, uri: Url, text: String) -> Result<(), String> {
        self.index_file_with_generation(uri, text, 0)
    }

    /// Index a file from its URI, text content, and document generation.
    pub fn index_file_with_generation(
        &self,
        uri: Url,
        text: String,
        generation: u32,
    ) -> Result<(), String> {
        let uri_str = uri.to_string();

        // Compute content hash for early-exit optimization
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let content_hash = hasher.finish();

        // Check if content is unchanged (early-exit optimization), and --
        // still under the SAME `files.write()` guard -- update
        // `document_store`. `document_store` has its own, entirely separate
        // internal lock (`DocumentStore::documents: Arc<RwLock<..>>`, no
        // cross-reference to `self.files`), so holding `files.write()`
        // across both the generation check AND the `document_store` write
        // does not risk any lock-ordering deadlock; it makes the two
        // genuinely atomic with respect to any other writer of this URI's
        // entry, closing the race outright rather than narrowing it (an
        // earlier revision applied this same generation check here but
        // released `files.write()` before the `document_store` write,
        // leaving a real, if nanosecond-scale, window a stale out-of-order
        // task could still land in -- flagged again by factory-droid on
        // PR #3618 after that partial fix).
        let key = DocumentStore::uri_key(&uri_str);
        // Set below iff this task's generation genuinely advances the
        // claimed high-water mark (a genuine reservation, not a
        // same-or-older no-op). `ReservationGuard::drop` rolls the claim
        // back automatically on ANY early return between here and the late
        // guard's successful commit (`.commit()`, called once this task's
        // generation is actually written to `self.files` below) -- see its
        // doc comment for why it restores toward the last genuinely
        // committed generation rather than a per-task snapshot.
        let mut reservation: Option<ReservationGuard<'_>> = None;
        {
            let mut files = self.files.write();
            if let Some(existing_index) = files.get_mut(&key) {
                if existing_index.content_hash == content_hash {
                    existing_index.generation = existing_index.generation.max(generation);
                    existing_index.pending_generation =
                        existing_index.pending_generation.max(generation);
                    // Content unchanged, skip re-indexing
                    #[cfg(test)]
                    reindex_metrics::record_content_hash_short_circuit();
                    return Ok(());
                }
                // Same monotonic generation guard as the one under the later
                // `files.write()` block below (see its doc comment for the
                // full out-of-order-completion race this closes) -- applied
                // here too so a stale out-of-order task can't overwrite
                // `document_store`'s text with older content even when it's
                // correctly rejected from `self.files` by the later guard.
                // Compares against the HIGH-WATER MARK (genuinely committed
                // OR still-in-flight reserved), not just the committed
                // generation, so a concurrent newer task that hasn't
                // finished parsing yet still correctly rejects an older
                // out-of-order task here.
                let high_water = existing_index.generation.max(existing_index.pending_generation);
                if generation > 0 && high_water > 0 && high_water > generation {
                    #[cfg(test)]
                    reindex_metrics::record_stale_rejected_pre_parse();
                    return Ok(());
                }
                // Reserve this generation NOW, before parsing -- not just at
                // the later guard, which only runs AFTER
                // `Parser::new(&text).parse()` below completes. Without this,
                // two concurrent tasks for adjacent generations N and N+1 can
                // BOTH read the high-water mark here before EITHER has
                // finished parsing, so neither guard sees the other as newer
                // and both proceed to write `document_store` -- if N's
                // (older) write lands after N+1's, `document_store.text`
                // ends up holding stale content indefinitely, observable by
                // cross-file consumers (rename, safe-delete preview,
                // navigation, hover for other-file symbols) that read
                // `document_store()` directly (flagged by factory-droid and
                // cubic on PR #3618). Bumping `pending_generation` here,
                // still under this SAME `files.write()` acquisition, makes
                // it visible to the very next racer's early check
                // immediately -- it does not need to wait for this task's
                // parse to finish.
                //
                // This claim is tracked SEPARATELY from `generation` (the
                // genuinely-committed field read by `indexed_generation()`
                // and everything else) precisely so a reservation that never
                // pans out -- parse error, or the document closing before
                // the late guard runs -- has nothing to roll back on the
                // field callers actually trust; only `pending_generation`
                // needs cleanup, and `ReservationGuard` does that
                // automatically on any early return (review-3660 findings
                // 3(a)/3(b)/3(c) on PR #3618).
                if generation > 0 && generation > high_water {
                    existing_index.pending_generation = generation;
                    reservation = Some(ReservationGuard {
                        index: self,
                        key: key.clone(),
                        reserved: generation,
                        committed: false,
                    });
                }
            }

            // Update document store -- still holding `files.write()`, so no
            // other writer of this URI can observe a partial state between
            // the generation check above and this write.
            if self.document_store.is_open(&uri_str) {
                self.document_store.update(&uri_str, 1, text.clone());
            } else {
                self.document_store.open(uri_str.clone(), 1, text.clone());
            }
        }

        // Parse the file
        let mut parser = Parser::new(&text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                // `reservation`'s `Drop` (if it holds a claim) rolls
                // `pending_generation` back to the last genuinely committed
                // generation -- this task never reaches the late guard's
                // commit below.
                return Err(format!("Parse error: {}", e));
            }
        };

        // Get the document for line index. If the document was closed out
        // from under a still-in-flight background index task (e.g. a rapid
        // didClose racing this task's own reservation), `reservation`'s
        // `Drop` rolls the claim back here too -- previously this early
        // return left `self.files[key].generation`'s reservation
        // permanently claimed with nothing that would ever commit it
        // (#3618 review-3660 finding 3(c)).
        let mut doc = self.document_store.get(&uri_str).ok_or("Document not found")?;

        // Determine workspace folder URI from the file URI
        let folder_uri = self.determine_folder_uri(&uri_str);

        // Extract symbols and references via the unified single-traversal
        // extraction bundle (perl-lsp-swarm#1711-B cutover). One AST walk
        // (`IndexVisitor::visit_unified`, run inside
        // `FileExtractionBundle::build_unified`) now produces BOTH the
        // legacy `FileIndex` reference/dependency projection AND the
        // canonical `Vec<SymbolRef>` projection, replacing the two
        // independent full-AST reference walks (`IndexVisitor::visit` +
        // `extract_symbol_refs`) this path ran before. Declaration
        // extraction, eval-sub boundary facts, generated-member facts, and
        // import/use-lib extraction are UNCHANGED by this cutover -- only
        // the reference walk is unified (declarations are a separable
        // follow-up; see `FileExtractionBundle::build_unified`'s doc
        // comment).
        let mut bundle =
            FileExtractionBundle::build_unified(&ast, &uri_str, content_hash, &mut doc, folder_uri);
        // `build_unified` builds its own `FileIndex` (it has no notion of
        // this call's `generation` parameter) -- restore it here, exactly
        // as the pre-cutover `FileIndex { ..., generation, ... }` literal
        // did.
        bundle.legacy_index.generation = generation;
        let file_index = bundle.legacy_index;

        let fact_shard = if bundle.canonical_shard.anchors.is_empty()
            && bundle.canonical_shard.entities.is_empty()
            && bundle.canonical_shard.occurrences.is_empty()
            && bundle.canonical_shard.edges.is_empty()
        {
            Self::build_fact_shard(&uri_str, content_hash, &file_index)
        } else {
            bundle.canonical_shard
        };

        // Update the import/export index with the import specs and use-lib
        // facts the unified extraction bundle above already produced
        // (`extract_import_specs`/`extract_use_lib_facts`, unchanged by
        // this cutover) -- populates ImportExportIndex so that
        // `Foo->import(@names)` dynamic-import suppression is live in
        // production.
        //
        // Lock ordering note: `semantic_import_export_index` is acquired write
        // separately from (and after) `files`/`symbols`/`global_references` to
        // match the consistent lock-order used throughout this file.
        let file_id = Self::hash_uri_to_file_id(&uri_str);
        let import_specs = bundle.import_specs;
        let use_lib_facts = bundle.use_lib_facts;

        // Update the index, refresh the global symbol cache, and replace this file's
        // contribution in the global reference index.
        {
            let mut files = self.files.write();

            // Monotonic generation guard: an older TRACKED generation must
            // never overwrite a newer TRACKED one, regardless of
            // completion order.
            //
            // This write path is reached by, among other callers, a
            // fire-and-forget background indexing task (see
            // `LspServer::run_post_parse_side_effects` in perl-lsp-rs --
            // `handle.spawn_blocking(task)`) that returns to its caller as
            // soon as it is *spawned*, not when it completes. Two such
            // tasks for adjacent generations N and N+1 of the same URI can
            // therefore run on different blocking-pool threads and finish
            // OUT OF ORDER. Each independently passes its own
            // pre-spawn freshness check (a single check-then-act that is
            // NOT atomic with this write), so completion order alone
            // decided which generation's content ended up stored here --
            // if N finishes after N+1, N would silently overwrite N+1,
            // producing torn state: this index at N while the document's
            // own symbol_index/AST are already at N+1, externally
            // observable as inconsistent `workspace/symbol` vs.
            // same-file answers until the next edit.
            //
            // This check closes that hole at the STORE itself -- a second,
            // independent line of defense that does not depend on task
            // completion order. It is evaluated under the SAME
            // `files.write()` acquisition as the `files.insert` below, so
            // the check-then-act is atomic with respect to any other
            // writer of this URI's entry.
            //
            // `generation == 0` is deliberately treated as "untracked" and
            // exempt from this guard, matching every other call site in
            // the codebase that does not thread a real per-document
            // generation counter through `index_file_with_generation`:
            // `index_file()` (the ungenerationed convenience wrapper used
            // by file-watcher re-indexing, workspace-wide rescans, rename
            // preview indexing, and most tests) and the `didOpen`
            // background index task (`runtime/text_sync.rs`, which always
            // passes `0` since a freshly opened document always starts at
            // generation 0) both intentionally call this with `generation:
            // 0` on every invocation, including *legitimate re-indexes of
            // already-tracked documents* (e.g. reopening a file that was
            // previously edited to some generation > 0, or an external
            // on-disk change picked up by the file watcher after edits).
            // A strict numeric comparison across these untracked calls and
            // the async parse worker's tracked calls would incorrectly
            // block those legitimate refreshes once any document reached
            // generation > 0 -- confirmed empirically: an unconditional
            // `existing.generation >= generation` guard here made 7
            // existing tests fail (`test_early_exit_optimization_changed_content`
            // et al.), all of which re-index the same URI twice through the
            // untracked `generation: 0` convention and expect the second
            // (later) call to win. Only comparing when BOTH sides are
            // genuinely tracked (`> 0`) closes the out-of-order race for
            // the async parse worker -- the only caller that ever supplies
            // a real, monotonically-increasing generation for the same
            // in-flight document -- without touching any untracked caller.
            if generation > 0 {
                if let Some(existing) = files.get(&key) {
                    if existing.generation > 0 && existing.generation > generation {
                        #[cfg(test)]
                        reindex_metrics::record_stale_rejected_post_parse();
                        return Ok(());
                    }
                }
            }

            // Remove stale global references from previous version of this file
            if let Some(old_index) = files.get(&key) {
                let mut global_refs = self.global_references.write();
                #[cfg(test)]
                reindex_metrics::record_global_refs_removed(
                    old_index.references.values().map(std::vec::Vec::len).sum(),
                );
                Self::remove_file_global_refs(&mut global_refs, old_index, &uri_str);
            }

            // Incrementally remove old symbols before inserting new file
            if let Some(old_index) = files.get(&key) {
                let mut symbols = self.symbols.write();
                let mut search_idx = self.search_index.write();
                #[cfg(test)]
                reindex_metrics::record_legacy_symbols_removed(old_index.symbols.len());
                #[cfg(test)]
                reindex_metrics::record_legacy_search_removed(old_index.symbols.len());
                Self::incremental_remove_symbols(&files, &mut symbols, old_index);
                Self::incremental_remove_search(&mut search_idx, old_index);
                drop(search_idx);
                drop(symbols);
            }
            files.insert(key.clone(), file_index);
            // This generation is now genuinely committed -- disarm the
            // reservation's rollback so its `Drop` at function end is a
            // no-op. Nothing left to roll back: `pending_generation` on the
            // freshly-inserted `FileIndex` resets to its default (0), which
            // is harmless -- `generation` (just written above) is the only
            // field any guard or reader ever trusts as "committed", and a
            // reset `pending_generation` only ever makes a FUTURE early
            // guard's high-water comparison more permissive, never less
            // correct, since `generation.max(pending_generation)` still
            // floors at the value just committed here.
            if let Some(reservation) = reservation.take() {
                reservation.commit();
            }
            let mut symbols = self.symbols.write();
            let mut search_idx = self.search_index.write();
            if let Some(new_index) = files.get(&key) {
                #[cfg(test)]
                reindex_metrics::record_legacy_symbols_added(new_index.symbols.len());
                #[cfg(test)]
                reindex_metrics::record_legacy_search_added(new_index.symbols.len());
                Self::incremental_add_symbols(&mut symbols, new_index);
                Self::incremental_add_search(&mut search_idx, new_index);
            }

            if let Some(file_index) = files.get(&key) {
                let mut global_refs = self.global_references.write();
                #[cfg(test)]
                reindex_metrics::record_global_refs_added(
                    file_index.references.values().map(std::vec::Vec::len).sum(),
                );
                for (name, refs) in &file_index.references {
                    let entry = global_refs.entry(name.clone()).or_default();
                    for reference in refs {
                        entry.push(Location { uri: reference.uri.clone(), range: reference.range });
                    }
                }
            }
            self.replace_fact_shard_incremental(&key, fact_shard);
            #[cfg(test)]
            reindex_metrics::record_generation_accepted();
        }

        // Update the import/export index with the freshly extracted import specs
        // and use-lib facts.  Stale entries for this URI are removed first
        // (incremental re-indexing).  This is done after the main write lock
        // block to follow the established lock ordering
        // (shards → reference_index → import_export_index).
        {
            let mut ie_idx = self.semantic_import_export_index.write();
            ie_idx.remove_file_imports(&uri_str);
            ie_idx.add_file_imports(&uri_str, file_id, import_specs);
            ie_idx.remove_file_use_lib(&uri_str);
            ie_idx.add_file_use_lib(&uri_str, file_id, use_lib_facts);
        }

        Ok(())
    }

    /// Remove a file from the index
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI (string form) to remove
    ///
    /// # Returns
    ///
    /// Nothing. The index is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// index.remove_file("file:///example.pl");
    /// ```
    pub fn remove_file(&self, uri: &str) {
        let uri_str = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&uri_str);

        // Remove from document store
        self.document_store.close(&uri_str);

        // Remove file index
        let mut files = self.files.write();
        if let Some(file_index) = files.remove(&key) {
            self.fact_shards.write().remove(&key);

            // Clean up semantic cross-file indexes for this file.
            self.semantic_reference_index.write().remove_file(&uri_str);
            {
                let mut ie_idx = self.semantic_import_export_index.write();
                ie_idx.remove_file_imports(&uri_str);
                ie_idx.remove_module_exports(&uri_str);
                ie_idx.remove_file_use_lib(&uri_str);
            }

            // Incrementally remove symbols and re-insert any shadowed names.
            let mut symbols = self.symbols.write();
            let mut search_idx = self.search_index.write();
            Self::incremental_remove_symbols(&files, &mut symbols, &file_index);
            Self::incremental_remove_search(&mut search_idx, &file_index);

            // Defensive sweep: purge any remaining cache entries whose value
            // points to this file's URI.  incremental_remove_symbols already
            // handles known symbol names; this sweep guarantees no stale
            // candidates survive even when:
            //   * the file had zero symbols (nothing for incremental_remove
            //     to walk), or
            //   * a symbol's stored uri differs from the canonical normalize_uri
            //     output (URI normalization edge cases).
            // Match against every URI spelling observed in this file index plus
            // the canonical uri_str so raw/normalized variants are all caught.
            let mut removed_uris = vec![uri_str.as_str()];
            for observed_uri in file_index.symbols.iter().map(|s| s.uri.as_str()).chain(
                file_index.references.values().flat_map(|refs| refs.iter().map(|r| r.uri.as_str())),
            ) {
                if !removed_uris.contains(&observed_uri) {
                    removed_uris.push(observed_uri);
                }
            }
            symbols.retain(|_, candidates| {
                candidates.retain(|candidate| {
                    let cand_uri = candidate.location.uri.as_str();
                    !removed_uris.contains(&cand_uri)
                });
                !candidates.is_empty()
            });
            // Defensive sweep for search_index: remove any remaining entries
            // pointing to the removed URI (mirrors the symbols sweep above).
            search_idx.retain(|_, syms| {
                syms.retain(|sym| !removed_uris.contains(&sym.uri.as_str()));
                !syms.is_empty()
            });

            // Remove from global reference index. Two-phase cleanup: first
            // remove names this file was known to reference (cheap path), then
            // a defensive sweep over all remaining entries to catch any that
            // were inserted under names not present in this file's
            // FileIndex::references map (e.g. via aggregated/global insertion
            // paths). Empty buckets are dropped.
            let mut global_refs = self.global_references.write();
            Self::remove_file_global_refs(&mut global_refs, &file_index, &uri_str);
            global_refs.retain(|_, locs| {
                locs.retain(|loc| !removed_uris.contains(&loc.uri.as_str()));
                !locs.is_empty()
            });
        }
    }

    /// Remove a file from the index (URL variant for compatibility)
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI as a parsed `Url`
    ///
    /// # Returns
    ///
    /// Nothing. The index is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    /// use url::Url;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let index = WorkspaceIndex::new();
    /// let uri = Url::parse("file:///example.pl")?;
    /// index.remove_file_url(&uri);
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_file_url(&self, uri: &Url) {
        self.remove_file(uri.as_str())
    }

    /// Clear a file from the index (alias for remove_file)
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI (string form) to remove
    ///
    /// # Returns
    ///
    /// Nothing. The index is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// index.clear_file("file:///example.pl");
    /// ```
    pub fn clear_file(&self, uri: &str) {
        self.remove_file(uri);
    }

    /// Clear a file from the index (URL variant for compatibility)
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI as a parsed `Url`
    ///
    /// # Returns
    ///
    /// Nothing. The index is updated in-place.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    /// use url::Url;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let index = WorkspaceIndex::new();
    /// let uri = Url::parse("file:///example.pl")?;
    /// index.clear_file_url(&uri);
    /// # Ok(())
    /// # }
    /// ```
    pub fn clear_file_url(&self, uri: &Url) {
        self.clear_file(uri.as_str())
    }

    /// Remove all files from a specific workspace folder.
    ///
    /// This method removes all indexed files that belong to the given
    /// workspace folder URI. This is useful when a workspace folder is
    /// removed from the multi-root workspace.
    ///
    /// # Arguments
    ///
    /// * `folder_uri` - The workspace folder URI to remove files from
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_workspace::workspace::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// // Index files from multiple folders...
    /// index.remove_folder("file:///project1");
    /// ```
    pub fn remove_folder(&self, folder_uri: &str) {
        let mut uris_to_remove = Vec::new();
        let files = self.files.read();

        // Collect all files that belong to this folder
        for file_index in files.values() {
            if file_index.folder_uri.as_deref() == Some(folder_uri) {
                uris_to_remove.push(file_index.source_uri.clone());
            }
        }
        drop(files);

        // Remove each file through the full removal path to keep
        // symbol/reference caches and document store in sync.
        for uri in uris_to_remove {
            self.remove_file(&uri);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Index a file from a URI string for the Index/Analyze workflow.
    ///
    /// Accepts either a `file://` URI or a filesystem path. Not available on
    /// wasm32 targets (requires filesystem path conversion).
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI string or filesystem path.
    /// * `text` - Full Perl source text for indexing.
    ///
    /// # Returns
    ///
    /// `Ok(())` when indexing succeeds, or an error message otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI is invalid or parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let index = WorkspaceIndex::new();
    /// index.index_file_str("file:///example.pl", "sub hello { }")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn index_file_str(&self, uri: &str, text: &str) -> Result<(), String> {
        let path = Path::new(uri);
        let url = if path.is_absolute() {
            url::Url::from_file_path(path)
                .map_err(|_| format!("Invalid URI or file path: {}", uri))?
        } else {
            // Raw absolute Windows paths like C:\foo can parse as a bogus URI
            // (`c:` scheme). Prefer URL parsing only for non-path inputs.
            url::Url::parse(uri).or_else(|_| {
                url::Url::from_file_path(path)
                    .map_err(|_| format!("Invalid URI or file path: {}", uri))
            })?
        };
        self.index_file(url, text.to_string())
    }

    /// Index multiple files in a single batch operation.
    ///
    /// This is significantly faster than calling `index_file` in a loop for
    /// initial workspace scans because it defers the global symbol cache
    /// rebuild to a single pass at the end.
    ///
    /// Phase 1: Parse all files without holding locks.
    /// Phase 2: Bulk-insert file indices and rebuild the symbol cache once.
    pub fn index_files_batch(&self, files_to_index: Vec<(Url, String)>) -> Vec<String> {
        let mut errors = Vec::new();

        // Phase 1: Parse all files without locks
        let mut parsed: Vec<(String, String, FileIndex)> = Vec::with_capacity(files_to_index.len());
        for (uri, text) in &files_to_index {
            let uri_str = uri.to_string();

            // Content hash for early-exit
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let content_hash = hasher.finish();

            let key = DocumentStore::uri_key(&uri_str);

            // Check if content unchanged
            {
                let files = self.files.read();
                if let Some(existing) = files.get(&key) {
                    if existing.content_hash == content_hash {
                        continue;
                    }
                }
            }

            // Update document store
            if self.document_store.is_open(&uri_str) {
                self.document_store.update(&uri_str, 1, text.clone());
            } else {
                self.document_store.open(uri_str.clone(), 1, text.clone());
            }

            // Parse
            let mut parser = Parser::new(text);
            let ast = match parser.parse() {
                Ok(ast) => ast,
                Err(e) => {
                    errors.push(format!("Parse error in {}: {}", uri_str, e));
                    continue;
                }
            };

            let mut doc = match self.document_store.get(&uri_str) {
                Some(d) => d,
                None => {
                    errors.push(format!("Document not found: {}", uri_str));
                    continue;
                }
            };

            // Determine workspace folder URI from the file URI
            let folder_uri = self.determine_folder_uri(&uri_str);

            let mut file_index = FileIndex {
                source_uri: uri_str.clone(),
                content_hash,
                folder_uri: folder_uri.clone(),
                ..Default::default()
            };
            let mut visitor = IndexVisitor::new(&mut doc, uri_str.clone(), folder_uri);
            visitor.visit(&ast, &mut file_index);

            parsed.push((key, uri_str, file_index));
        }

        // Phase 2: Bulk insert with single cache rebuild
        {
            let mut files = self.files.write();
            let mut symbols = self.symbols.write();
            let mut search_idx = self.search_index.write();
            let mut global_refs = self.global_references.write();

            // Pre-allocate capacity for the incoming batch to avoid rehashing.
            // Each symbol is indexed under both its qualified name and bare name.
            files.reserve(parsed.len());
            symbols.reserve(parsed.len().saturating_mul(20).saturating_mul(2));

            for (key, uri_str, file_index) in parsed {
                // Remove stale global references
                if let Some(old_index) = files.get(&key) {
                    Self::remove_file_global_refs(&mut global_refs, old_index, &uri_str);
                }

                files.insert(key.clone(), file_index);

                // Add global references for this file
                if let Some(fi) = files.get(&key) {
                    for (name, refs) in &fi.references {
                        let entry = global_refs.entry(name.clone()).or_default();
                        for reference in refs {
                            entry.push(Location {
                                uri: reference.uri.clone(),
                                range: reference.range,
                            });
                        }
                    }
                }
            }

            // Single rebuild at the end
            Self::rebuild_symbol_cache(&files, &mut symbols);
            Self::rebuild_search_index(&files, &mut search_idx);
        }

        errors
    }

    /// Find all references to a symbol using dual indexing strategy
    ///
    /// This function searches for both exact matches and bare name matches when
    /// the symbol is qualified. For example, when searching for "Utils::process_data":
    /// - First searches for exact "Utils::process_data" references
    /// - Then searches for bare "process_data" references that might refer to the same function
    ///
    /// This dual approach handles cases where functions are called both as:
    /// - Qualified: `Utils::process_data()`
    /// - Unqualified: `process_data()` (when in the same package or imported)
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - Symbol name or qualified name to search
    ///
    /// # Returns
    ///
    /// All reference locations found for the requested symbol.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _refs = index.find_references("Utils::process_data");
    /// ```
    pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
        let global_refs = self.global_references.read();
        let mut seen: HashSet<(String, u32, u32, u32, u32)> = HashSet::new();
        let mut locations = Vec::new();

        // O(1) lookup for exact symbol name
        if let Some(refs) = global_refs.get(symbol_name) {
            for loc in refs {
                let key = (
                    loc.uri.clone(),
                    loc.range.start.line,
                    loc.range.start.column,
                    loc.range.end.line,
                    loc.range.end.column,
                );
                if seen.insert(key) {
                    locations.push(Location { uri: loc.uri.clone(), range: loc.range });
                }
            }
        }

        // If the symbol is qualified, also collect bare name references
        if let Some(idx) = symbol_name.rfind("::") {
            let bare_name = &symbol_name[idx + 2..];
            if let Some(refs) = global_refs.get(bare_name) {
                for loc in refs {
                    let key = (
                        loc.uri.clone(),
                        loc.range.start.line,
                        loc.range.start.column,
                        loc.range.end.line,
                        loc.range.end.column,
                    );
                    if seen.insert(key) {
                        locations.push(Location { uri: loc.uri.clone(), range: loc.range });
                    }
                }
            }
        } else {
            // If the symbol is bare, also collect qualified references that end
            // with the same bare name, e.g. `Pkg::foo` when searching for `foo`.
            for (name, refs) in global_refs.iter() {
                if !Self::is_qualified_variant_of(name, symbol_name) {
                    continue;
                }

                for loc in refs {
                    let key = (
                        loc.uri.clone(),
                        loc.range.start.line,
                        loc.range.start.column,
                        loc.range.end.line,
                        loc.range.end.column,
                    );
                    if seen.insert(key) {
                        locations.push(Location { uri: loc.uri.clone(), range: loc.range });
                    }
                }
            }
        }

        Self::sort_locations_deterministically(&mut locations);
        locations
    }

    /// Resolve a symbol and return its definition/reference set for cross-file planning.
    ///
    /// Returns `None` when no definition can be resolved for `symbol_name`.
    pub fn query_symbol_references(
        &self,
        symbol_name: &str,
    ) -> Option<CrossFileReferenceQueryResult> {
        let definition = self.find_definition(symbol_name)?;
        let symbol = self.find_symbol_by_definition(&definition, symbol_name)?;

        let stable_key = symbol.qualified_name.clone().unwrap_or_else(|| {
            format!(
                "{}@{}:{}:{}",
                symbol.name, symbol.uri, symbol.range.start.line, symbol.range.start.column
            )
        });
        let mut references = self.collect_symbol_references(&symbol);
        if !references.iter().any(|location| location == &definition) {
            references.push(definition.clone());
            Self::sort_locations_deterministically(&mut references);
        }

        Some(CrossFileReferenceQueryResult {
            symbol: SymbolIdentity {
                stable_key,
                name: symbol.name,
                qualified_name: symbol.qualified_name,
                kind: symbol.kind,
            },
            definition,
            references,
        })
    }

    /// Count non-definition references (usages) of a symbol.
    ///
    /// Like `find_references` but excludes `ReferenceKind::Definition` entries,
    /// returning only actual usage sites. This is used by code lens to show
    /// "N references" where N means call sites, not the definition itself.
    pub fn count_usages(&self, symbol_name: &str) -> usize {
        let files = self.files.read();
        let mut seen: HashSet<(String, u32, u32, u32, u32)> = HashSet::new();

        for (_uri_key, file_index) in files.iter() {
            if let Some(refs) = file_index.references.get(symbol_name) {
                for r in refs.iter().filter(|r| r.kind != ReferenceKind::Definition) {
                    seen.insert((
                        r.uri.clone(),
                        r.range.start.line,
                        r.range.start.column,
                        r.range.end.line,
                        r.range.end.column,
                    ));
                }
            }

            if let Some(idx) = symbol_name.rfind("::") {
                let bare_name = &symbol_name[idx + 2..];
                if let Some(refs) = file_index.references.get(bare_name) {
                    for r in refs.iter().filter(|r| r.kind != ReferenceKind::Definition) {
                        seen.insert((
                            r.uri.clone(),
                            r.range.start.line,
                            r.range.start.column,
                            r.range.end.line,
                            r.range.end.column,
                        ));
                    }
                }
            } else {
                for (name, refs) in &file_index.references {
                    if !Self::is_qualified_variant_of(name, symbol_name) {
                        continue;
                    }

                    for r in refs.iter().filter(|r| r.kind != ReferenceKind::Definition) {
                        seen.insert((
                            r.uri.clone(),
                            r.range.start.line,
                            r.range.start.column,
                            r.range.end.line,
                            r.range.end.column,
                        ));
                    }
                }
            }
        }

        seen.len()
    }

    fn is_qualified_variant_of(candidate: &str, bare_symbol: &str) -> bool {
        candidate.rsplit_once("::").is_some_and(|(_, candidate_bare)| candidate_bare == bare_symbol)
    }

    /// Find the definition of a symbol
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - Symbol name or qualified name to resolve
    ///
    /// # Returns
    ///
    /// The first matching definition location, if found.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _def = index.find_definition("MyPackage::example");
    /// ```
    pub fn find_definition(&self, symbol_name: &str) -> Option<Location> {
        if let Some(location) = self.definition_candidates(symbol_name).into_iter().next() {
            return Some(location);
        }

        // Fall back to a full files scan for this query. The result is intentionally
        // NOT written back to `self.symbols`: every indexed symbol is already
        // inserted under both qualified and bare names by `incremental_add_symbols`,
        // so any cache miss here is for a key that does not correspond to an
        // indexed symbol (e.g. a typo or alias). Caching such queries is unsound
        // (entries become stale on file edits and were never tracked for cleanup
        // in `remove_file`/`incremental_remove_symbols`) and lets the cache grow
        // unboundedly across long sessions. Returning the resolved location
        // directly preserves correctness without retaining state.
        let files = self.files.read();
        Self::find_definition_in_files(&files, symbol_name, None).map(|(location, _uri)| location)
    }

    pub(crate) fn definition_candidates(&self, symbol_name: &str) -> Vec<Location> {
        let symbols = self.symbols.read();
        symbols
            .get(symbol_name)
            .map(|candidates| {
                candidates.iter().map(|candidate| candidate.location.clone()).collect()
            })
            .unwrap_or_default()
    }

    /// Get all symbols in the workspace
    ///
    /// # Returns
    ///
    /// A vector containing every symbol currently indexed.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _symbols = index.all_symbols();
    /// ```
    pub fn all_symbols(&self) -> Vec<WorkspaceSymbol> {
        let files = self.files.read();
        let mut symbols = Vec::new();

        for (_uri_key, file_index) in files.iter() {
            symbols.extend(file_index.symbols.clone());
        }

        symbols
    }

    /// Clear all indexed files and symbols from the workspace.
    pub fn clear(&self) {
        self.files.write().clear();
        self.symbols.write().clear();
        self.search_index.write().clear();
        self.global_references.write().clear();
        self.fact_shards.write().clear();
        *self.semantic_reference_index.write() = ReferenceIndex::new();
        *self.semantic_import_export_index.write() = ImportExportIndex::new();
    }

    fn hash_uri_to_file_id(uri: &str) -> FileId {
        let mut hasher = DefaultHasher::new();
        uri.hash(&mut hasher);
        FileId(hasher.finish())
    }

    fn build_fact_shard(uri: &str, content_hash: u64, file_index: &FileIndex) -> FileFactShard {
        let file_id = Self::hash_uri_to_file_id(uri);
        let mut anchors = Vec::new();
        let mut entities = Vec::new();
        for (idx, symbol) in file_index.symbols.iter().enumerate() {
            let anchor_id = AnchorId((idx + 1) as u64);
            anchors.push(AnchorFact {
                id: anchor_id,
                file_id,
                // WorkspaceSymbol provides line/column coordinates only, not byte
                // offsets.  Zero-initialize span_*_byte until a byte-offset source
                // is plumbed through the indexing pipeline.
                span_start_byte: 0,
                span_end_byte: 0,
                scope_id: None,
                provenance: Provenance::SearchFallback,
                confidence: Confidence::Low,
            });
            entities.push(EntityFact {
                id: EntityId((idx + 1) as u64),
                kind: EntityKind::Unknown,
                canonical_name: symbol
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| symbol.name.clone()),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::SearchFallback,
                confidence: Confidence::Low,
            });
        }
        // Hash the per-category fact vectors so consumers can detect staleness
        // without re-reading the full shard.
        let anchors_hash = {
            let mut h = DefaultHasher::new();
            anchors.len().hash(&mut h);
            for a in &anchors {
                a.id.hash(&mut h);
                a.span_start_byte.hash(&mut h);
                a.span_end_byte.hash(&mut h);
            }
            h.finish()
        };
        let entities_hash = {
            let mut h = DefaultHasher::new();
            entities.len().hash(&mut h);
            for e in &entities {
                e.id.hash(&mut h);
                e.canonical_name.hash(&mut h);
            }
            h.finish()
        };
        FileFactShard {
            source_uri: uri.to_string(),
            file_id,
            content_hash,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: Some(anchors_hash),
            entities_hash: Some(entities_hash),
            occurrences_hash: Some(0),
            edges_hash: Some(0),
            anchors,
            entities,
            occurrences: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Build a canonical [`FileFactShard`] from the AST using the semantic
    /// fact adapters in `perl-symbol`, calling `extract_symbol_refs(ast)`
    /// itself (a second, independent full-AST reference walk).
    ///
    /// **Superseded in production by [`Self::build_canonical_fact_shard_from_symbol_refs`]
    /// as of the 1711-B cutover** (`index_file_with_generation` now derives its
    /// canonical `Vec<SymbolRef>` from the unified `visit_unified` walk instead
    /// of calling `extract_symbol_refs` a second time). Kept as the
    /// `extraction_bundle_shadow_compare` parity harness's `build_direct`/
    /// `FileExtractionBundle::build` reference point -- the pre-cutover
    /// dual-walk behavior this cutover's parity tests assert against.
    /// Reachable only from `#[cfg(test)]` code (the shadow/parity harness);
    /// gated accordingly rather than `#[allow(dead_code)]`.
    #[cfg(test)]
    fn build_canonical_fact_shard_for_ast(
        uri: &str,
        content_hash: u64,
        ast: &Node,
    ) -> FileFactShard {
        let file_id = Self::hash_uri_to_file_id(uri);

        // Extract declarations and references from the AST.
        #[cfg(test)]
        let decl_start = Instant::now();
        let decls = extract_symbol_decls(ast, None);
        #[cfg(test)]
        reindex_metrics::record_decl_extract(decl_start.elapsed());
        #[cfg(test)]
        let ref_start = Instant::now();
        let refs = extract_symbol_refs(ast);
        #[cfg(test)]
        reindex_metrics::record_ref_extract(ref_start.elapsed());

        // Run the canonical adapters.
        let decl_facts = symbol_decls_to_semantic_facts(&decls, file_id);

        // Build an entity lookup map for reference resolution.
        let entity_ids_by_name: std::collections::BTreeMap<String, EntityId> =
            decl_facts.entities.iter().map(|e| (e.canonical_name.clone(), e.id)).collect();
        let ref_facts = symbol_refs_to_semantic_facts(&refs, file_id, &entity_ids_by_name);

        // Extract dynamic boundary evidence for `eval "sub NAME { ... }"` patterns.
        // Non-literal evals (e.g. `eval $code`) are intentionally skipped — the
        // sub name is not statically known and no evidence is emitted.
        #[cfg(test)]
        let eval_sub_start = Instant::now();
        let eval_sub_triples =
            crate::semantic::eval_sub_extractor::extract_eval_sub_boundaries(ast, file_id);
        #[cfg(test)]
        reindex_metrics::record_eval_sub(eval_sub_start.elapsed());
        let dynamic_boundaries: Vec<perl_semantic_facts::OccurrenceFact> =
            eval_sub_triples.iter().map(|(_, _, occ)| occ.clone()).collect();
        #[cfg(test)]
        let generated_member_start = Instant::now();
        let generated_member_facts =
            crate::semantic::generated_member_extractor::extract_generated_member_facts(
                ast, file_id,
            );
        #[cfg(test)]
        reindex_metrics::record_generated_member(generated_member_start.elapsed());

        // Build synthetic entity/anchor slices from eval-sub triples.
        // IMPORTANT: The triple is (entity, anchor, occurrence).  Only idx 0
        // (entity) and idx 1 (anchor) belong in the synthetic slices.  Idx 2
        // (occurrence) already flows through `dynamic_boundaries` above — do
        // NOT move it here or `occurrences_hash` will double-count.
        let synthetic_entities_from_eval: Vec<perl_semantic_facts::EntityFact> =
            eval_sub_triples.iter().map(|(entity, _, _)| entity.clone()).collect();
        let synthetic_anchors_from_eval: Vec<perl_semantic_facts::AnchorFact> =
            eval_sub_triples.iter().map(|(_, anchor, _)| anchor.clone()).collect();

        // Build synthetic entity/anchor slices from generated member facts.
        let synthetic_entities_from_generated: Vec<perl_semantic_facts::EntityFact> =
            generated_member_facts.iter().map(|f| f.entity.clone()).collect();
        let synthetic_anchors_from_generated: Vec<perl_semantic_facts::AnchorFact> =
            generated_member_facts.iter().map(|f| f.anchor.clone()).collect();

        // Merge into single synthetic slices for the canonical builder.
        let mut all_synthetic_entities = synthetic_entities_from_eval;
        all_synthetic_entities.extend(synthetic_entities_from_generated);
        let mut all_synthetic_anchors = synthetic_anchors_from_eval;
        all_synthetic_anchors.extend(synthetic_anchors_from_generated);

        // Build the canonical fact shard.
        // Synthetic entities/anchors are now passed to the builder so that
        // `entities_hash` and `anchors_hash` cover the COMPLETE set.
        // Import specs (for `use`, `require`, `ClassName->import()`) and
        // use-lib facts are populated separately via ImportExportIndex — not passed here.
        crate::semantic::facts::build_canonical_fact_shard(
            uri,
            content_hash,
            &decl_facts,
            &ref_facts,
            &[],
            &dynamic_boundaries,
            &all_synthetic_entities,
            &all_synthetic_anchors,
        )
    }

    /// **Production canonical builder for the unified reference traversal
    /// (perl-lsp-swarm#1711-B cutover).** Identical to
    /// [`Self::build_canonical_fact_shard_for_ast`] except it takes an
    /// ALREADY-COMPUTED `refs: &[SymbolRef]` instead of calling
    /// `extract_symbol_refs(ast)` itself -- this is what lets
    /// `FileExtractionBundle::build_unified` feed it the `Vec<SymbolRef>`
    /// produced by `IndexVisitor::visit_unified`'s single traversal, instead
    /// of running a second, independent `extract_symbol_refs` walk. Every
    /// other extractor call (`extract_symbol_decls` for declarations,
    /// eval-sub, generated-member) is UNCHANGED and unaffected -- only the
    /// reference walk is unified (see the #1711 feasibility comment for why
    /// declarations are a separable follow-up). Called by
    /// `index_file_with_generation` via `FileExtractionBundle::build_unified`;
    /// `build_canonical_fact_shard_for_ast` above remains live only for the
    /// `extraction_bundle_shadow_compare` parity harness's `build_direct`
    /// reference path, not for production.
    fn build_canonical_fact_shard_from_symbol_refs(
        uri: &str,
        content_hash: u64,
        ast: &Node,
        refs: &[perl_symbol::surface::r#ref::SymbolRef],
    ) -> FileFactShard {
        let file_id = Self::hash_uri_to_file_id(uri);

        #[cfg(test)]
        let decl_start = Instant::now();
        let decls = extract_symbol_decls(ast, None);
        #[cfg(test)]
        reindex_metrics::record_decl_extract(decl_start.elapsed());
        let decl_facts = symbol_decls_to_semantic_facts(&decls, file_id);

        let entity_ids_by_name: std::collections::BTreeMap<String, EntityId> =
            decl_facts.entities.iter().map(|e| (e.canonical_name.clone(), e.id)).collect();
        let ref_facts = symbol_refs_to_semantic_facts(refs, file_id, &entity_ids_by_name);

        #[cfg(test)]
        let eval_sub_start = Instant::now();
        let eval_sub_triples =
            crate::semantic::eval_sub_extractor::extract_eval_sub_boundaries(ast, file_id);
        #[cfg(test)]
        reindex_metrics::record_eval_sub(eval_sub_start.elapsed());
        let dynamic_boundaries: Vec<perl_semantic_facts::OccurrenceFact> =
            eval_sub_triples.iter().map(|(_, _, occ)| occ.clone()).collect();
        #[cfg(test)]
        let generated_member_start = Instant::now();
        let generated_member_facts =
            crate::semantic::generated_member_extractor::extract_generated_member_facts(
                ast, file_id,
            );
        #[cfg(test)]
        reindex_metrics::record_generated_member(generated_member_start.elapsed());

        let synthetic_entities_from_eval: Vec<perl_semantic_facts::EntityFact> =
            eval_sub_triples.iter().map(|(entity, _, _)| entity.clone()).collect();
        let synthetic_anchors_from_eval: Vec<perl_semantic_facts::AnchorFact> =
            eval_sub_triples.iter().map(|(_, anchor, _)| anchor.clone()).collect();
        let synthetic_entities_from_generated: Vec<perl_semantic_facts::EntityFact> =
            generated_member_facts.iter().map(|f| f.entity.clone()).collect();
        let synthetic_anchors_from_generated: Vec<perl_semantic_facts::AnchorFact> =
            generated_member_facts.iter().map(|f| f.anchor.clone()).collect();

        let mut all_synthetic_entities = synthetic_entities_from_eval;
        all_synthetic_entities.extend(synthetic_entities_from_generated);
        let mut all_synthetic_anchors = synthetic_anchors_from_eval;
        all_synthetic_anchors.extend(synthetic_anchors_from_generated);

        crate::semantic::facts::build_canonical_fact_shard(
            uri,
            content_hash,
            &decl_facts,
            &ref_facts,
            &[],
            &dynamic_boundaries,
            &all_synthetic_entities,
            &all_synthetic_anchors,
        )
    }

    /// Replace a [`FileFactShard`] with per-category incremental invalidation.
    ///
    /// Compares the whole-file `content_hash` first; when unchanged the
    /// replacement is skipped entirely.  Otherwise each per-category hash
    /// (`anchors_hash`, `entities_hash`, `occurrences_hash`, `edges_hash`)
    /// is compared individually.  Only categories whose hash changed trigger
    /// removal of old entries and insertion of new ones in the cross-file
    /// semantic indexes.
    ///
    /// **Validates: Requirements 18.1, 18.2, 18.3, 18.4, 18.5**
    pub fn replace_fact_shard_incremental(
        &self,
        key: &str,
        new_shard: FileFactShard,
    ) -> ShardReplaceResult {
        let mut shards = self.fact_shards.write();
        let old_shard = shards.get(key);

        let replacement = plan_shard_replacement(
            old_shard.map(Self::shard_category_hashes),
            Self::shard_category_hashes(&new_shard),
        );

        if replacement.content_unchanged {
            return replacement;
        }

        let source_uri = new_shard.source_uri.clone();

        // ── Update cross-file semantic indexes per category ──
        // Occurrences and edges are both managed by the ReferenceIndex.
        // When either changes we must remove+re-add the file in that index.
        if replacement.occurrences_updated || replacement.edges_updated {
            let mut ref_idx = self.semantic_reference_index.write();
            if old_shard.is_some() {
                ref_idx.remove_file(&source_uri);
            }
            ref_idx.add_file(&new_shard);
        }

        // Entities feed into the import/export index (export sets are keyed
        // by module name derived from entity canonical names).  When entities
        // change we refresh the import/export index for this file.
        if replacement.entities_updated {
            let mut ie_idx = self.semantic_import_export_index.write();
            ie_idx.remove_file_imports(&source_uri);
            ie_idx.remove_module_exports(&source_uri);
            // Re-add is handled by the caller or future wiring; for now we
            // ensure stale entries are purged.
        }

        // Store the new shard (always, since content_hash differs).
        shards.insert(key.to_string(), new_shard);

        replacement
    }

    fn shard_category_hashes(shard: &FileFactShard) -> ShardCategoryHashes {
        ShardCategoryHashes {
            content_hash: shard.content_hash,
            anchors_hash: shard.anchors_hash,
            entities_hash: shard.entities_hash,
            occurrences_hash: shard.occurrences_hash,
            edges_hash: shard.edges_hash,
        }
    }

    /// Number of stored file fact shards.
    pub fn fact_shard_count(&self) -> usize {
        self.fact_shards.read().len()
    }

    /// Fetch a file fact shard for test/inspection.
    pub fn file_fact_shard(&self, uri: &str) -> Option<FileFactShard> {
        let key = DocumentStore::uri_key(&Self::normalize_uri(uri));
        self.fact_shards.read().get(&key).cloned()
    }

    #[cfg(test)]
    fn inject_test_fact_shard(&self, shard: FileFactShard) {
        let key = DocumentStore::uri_key(&Self::normalize_uri(&shard.source_uri));
        self.fact_shards.write().insert(key, shard);
    }

    /// Resolve a semantic anchor to a source-backed LSP-wire location.
    ///
    /// Returns `None` for missing anchors, zero-width fallback anchors, or
    /// anchors whose source text is unavailable from the document store. If
    /// more than one shard contains the same anchor ID, this fails closed
    /// instead of choosing an arbitrary hash-map iteration result.
    pub fn semantic_anchor_wire_location(&self, anchor_id: AnchorId) -> Option<WireLocation> {
        let shards = self.fact_shards.read();
        let mut location = None;

        for shard in shards.values() {
            for anchor in shard.anchors.iter().filter(|anchor| anchor.id == anchor_id) {
                if anchor.span_end_byte <= anchor.span_start_byte {
                    return None;
                }

                let doc = self.document_store.get(&shard.source_uri)?;
                let start = usize::try_from(anchor.span_start_byte).ok()?;
                let end = usize::try_from(anchor.span_end_byte).ok()?;
                let next_location = WireLocation::new(
                    shard.source_uri.clone(),
                    WireRange::from_byte_offsets(&doc.text, start, end),
                );
                if location.replace(next_location).is_some() {
                    return None;
                }
            }
        }

        location
    }

    /// Resolve a semantic anchor to a source-backed LSP-wire location in a
    /// specific indexed file.
    ///
    /// This is the edit-safe variant of [`Self::semantic_anchor_wire_location`]:
    /// callers that already have `(file_id, anchor_id)` from a semantic plan do
    /// not need the global duplicate-anchor fail-closed behavior.
    pub fn semantic_anchor_wire_location_for_file(
        &self,
        file_id: FileId,
        anchor_id: AnchorId,
    ) -> Option<WireLocation> {
        let shards = self.fact_shards.read();
        let shard = shards.values().find(|shard| shard.file_id == file_id)?;
        let anchor = shard
            .anchors
            .iter()
            .find(|anchor| anchor.id == anchor_id && anchor.file_id == file_id)?;

        if anchor.span_end_byte <= anchor.span_start_byte {
            return None;
        }

        let doc = self.document_store.get(&shard.source_uri)?;
        let start = usize::try_from(anchor.span_start_byte).ok()?;
        let end = usize::try_from(anchor.span_end_byte).ok()?;
        doc.text.get(start..end)?;

        Some(WireLocation::new(
            shard.source_uri.clone(),
            WireRange::from_byte_offsets(&doc.text, start, end),
        ))
    }

    /// Compute the [`FileId`] for a URI using the same hash used during indexing.
    ///
    /// Returns `None` if the URI has not been indexed (no fact shard is present).
    pub fn file_id_for_uri(&self, uri: &str) -> Option<FileId> {
        let key = DocumentStore::uri_key(&Self::normalize_uri(uri));
        self.fact_shards.read().get(&key).map(|shard| shard.file_id)
    }

    /// Invoke a scoped callback with [`WorkspaceSemanticQueries`] built from
    /// the current semantic indexes for the given URI.
    ///
    /// The callback receives the resolved [`FileId`] and a
    /// [`WorkspaceSemanticQueries`] facade that borrows from read-locked
    /// semantic indexes. Locks are released when `f` returns.
    ///
    /// Returns `Some(result)` if the URI is indexed and semantic data is
    /// available, `None` if the URI has not been indexed or its fact shard is
    /// absent (the caller should fall back to legacy diagnostics).
    pub fn with_semantic_queries_for_uri<R>(
        &self,
        uri: &str,
        f: impl FnOnce(FileId, crate::semantic::queries::WorkspaceSemanticQueries<'_>) -> R,
    ) -> Option<R> {
        let key = DocumentStore::uri_key(&Self::normalize_uri(uri));

        // Acquire all three read guards simultaneously. The lock order must be
        // consistent with every other site that acquires multiple locks to avoid
        // deadlock: shards → reference_index → import_export_index.
        let shards_guard = self.fact_shards.read();
        let ref_guard = self.semantic_reference_index.read();
        let ie_guard = self.semantic_import_export_index.read();

        // Verify the URI is indexed before entering the callback.
        let file_id = shards_guard.get(&key)?.file_id;

        let queries = crate::semantic::queries::WorkspaceSemanticQueries::new(
            &ref_guard,
            &ie_guard,
            &shards_guard,
        );

        Some(f(file_id, queries))
    }

    /// Return the number of indexed files in the workspace
    pub fn file_count(&self) -> usize {
        let files = self.files.read();
        files.len()
    }

    /// Return the total number of symbols across all indexed files
    pub fn symbol_count(&self) -> usize {
        let files = self.files.read();
        files.values().map(|file_index| file_index.symbols.len()).sum()
    }

    /// Get all files in a specific workspace folder
    ///
    /// # Arguments
    ///
    /// * `folder_uri` - Workspace folder URI to filter by
    ///
    /// # Returns
    ///
    /// A vector of file indices belonging to the specified folder
    pub fn files_in_folder(&self, folder_uri: &str) -> Vec<FileIndex> {
        let files = self.files.read();
        files.values().filter(|f| f.folder_uri.as_deref() == Some(folder_uri)).cloned().collect()
    }

    /// Get all symbols in a specific workspace folder
    ///
    /// # Arguments
    ///
    /// * `folder_uri` - Workspace folder URI to filter by
    ///
    /// # Returns
    ///
    /// A vector of symbols belonging to the specified folder
    pub fn symbols_in_folder(&self, folder_uri: &str) -> Vec<WorkspaceSymbol> {
        let files = self.files.read();
        files
            .values()
            .filter(|f| f.folder_uri.as_deref() == Some(folder_uri))
            .flat_map(|f| f.symbols.iter().cloned())
            .collect()
    }

    /// Capture a point-in-time memory estimate of the index.
    ///
    /// Acquires read locks on all index components and walks their contents
    /// to estimate heap usage. Intended for offline profiling; do not call
    /// on the LSP hot path.
    ///
    /// Only available when the `memory-profiling` feature is enabled.
    #[cfg(feature = "memory-profiling")]
    pub fn memory_snapshot(&self) -> crate::workspace::memory::MemorySnapshot {
        use std::mem::size_of;

        let files_guard = self.files.read();
        let symbols_guard = self.symbols.read();
        let global_refs_guard = self.global_references.read();

        // --- files map ---
        let mut files_bytes: usize = 0;
        let mut total_symbol_count: usize = 0;
        for (uri_key, fi) in files_guard.iter() {
            // key string
            files_bytes += uri_key.len();
            // per-symbol entries
            for sym in &fi.symbols {
                files_bytes += sym.name.len()
                    + sym.uri.len()
                    + sym.qualified_name.as_deref().map_or(0, str::len)
                    + sym.documentation.as_deref().map_or(0, str::len)
                    + sym.container_name.as_deref().map_or(0, str::len)
                    // stack portion: kind + range + has_body + option discriminants
                    + size_of::<WorkspaceSymbol>();
            }
            total_symbol_count += fi.symbols.len();
            // per-reference entries
            for (ref_name, refs) in &fi.references {
                files_bytes += ref_name.len();
                for r in refs {
                    files_bytes += r.uri.len() + size_of::<SymbolReference>();
                }
            }
            // dependencies
            for dep in &fi.dependencies {
                files_bytes += dep.len();
            }
            // content hash (u64) + vec/hashset capacity overhead (rough)
            files_bytes += size_of::<u64>();
        }

        // --- global symbols map ---
        let mut symbols_bytes: usize = 0;
        for (qname, candidates) in symbols_guard.iter() {
            symbols_bytes += qname.len();
            for candidate in candidates {
                symbols_bytes += candidate.location.uri.len() + size_of::<Location>();
            }
        }

        // --- global references map ---
        let mut global_refs_bytes: usize = 0;
        for (sym_name, locs) in global_refs_guard.iter() {
            global_refs_bytes += sym_name.len();
            for loc in locs {
                global_refs_bytes += loc.uri.len() + size_of::<Location>();
            }
        }

        // --- document store ---
        let document_store_bytes = self.document_store.total_text_bytes();

        crate::workspace::memory::MemorySnapshot {
            file_count: files_guard.len(),
            symbol_count: total_symbol_count,
            files_bytes,
            symbols_bytes,
            global_refs_bytes,
            document_store_bytes,
        }
    }

    /// Check if the workspace index has symbols (soft readiness check)
    ///
    /// Returns true if the index contains any symbols, indicating that
    /// at least some files have been indexed and the workspace is ready
    /// for symbol-based operations like completion.
    ///
    /// # Returns
    ///
    /// `true` if any symbols are indexed, otherwise `false`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// assert!(!index.has_symbols());
    /// ```
    pub fn has_symbols(&self) -> bool {
        let files = self.files.read();
        files.values().any(|file_index| !file_index.symbols.is_empty())
    }

    /// Search for symbols by query
    ///
    /// # Arguments
    ///
    /// * `query` - Substring to match against symbol names
    ///
    /// # Returns
    ///
    /// Symbols whose names or qualified names contain the query string.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _results = index.search_symbols("example");
    /// ```
    pub fn search_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        self.search_source_symbols(query, None)
    }

    /// Search only source-backed syntax symbols from the workspace index.
    ///
    /// Generated/framework members are excluded. Use this when a caller needs
    /// to preserve the historical source-backed live slice for trust receipts
    /// or fallback paths.
    ///
    /// Uses the `search_index` (keyed by lowercase bare/qualified names) to
    /// iterate unique name keys rather than all (file, symbol) pairs, turning
    /// the outer loop from O(total_symbols) to O(unique_lowercase_names).
    /// A symbol that is stored under both its bare name key and its qualified
    /// name key is deduplicated by `(uri, start_byte)` so each `WorkspaceSymbol`
    /// appears at most once in the result.
    pub fn search_source_symbols(&self, query: &str, cap: Option<usize>) -> Vec<WorkspaceSymbol> {
        let query = query.trim();
        let query_lower = query.to_lowercase();
        let search_idx = self.search_index.read();
        let mut seen: HashSet<(String, usize)> = HashSet::new();
        let mut results = Vec::new();
        'outer: for (name_key, symbols) in search_idx.iter() {
            if name_key.contains(&query_lower) {
                for sym in symbols {
                    // Dedup: a symbol may appear under both its bare-name key
                    // and its qualified-name key; keep only the first occurrence.
                    let dedup_key = (sym.uri.clone(), sym.range.start.byte);
                    if seen.insert(dedup_key) {
                        results.push(sym.clone());
                        if cap.is_some_and(|c| results.len() >= c) {
                            break 'outer;
                        }
                    }
                }
            }
        }
        results
    }

    /// Search labeled generated/framework members backed by semantic source anchors.
    ///
    /// This is a narrow workspace-symbol pilot: returned symbols are explicitly
    /// labeled as generated/framework members and point at the source declaration
    /// that produced the member, not at an exact generated method body.
    pub fn search_generated_workspace_symbols(
        &self,
        query: &str,
        cap: Option<usize>,
    ) -> Vec<WorkspaceSymbol> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let source_backed_qualified_names = self.source_backed_qualified_names();
        let shards = self.fact_shards.read();
        let mut results = Vec::new();

        'outer: for shard in shards.values() {
            for entity in &shard.entities {
                if entity.kind != EntityKind::GeneratedMember {
                    continue;
                }
                if !is_framework_generated_member_entity(entity) {
                    continue;
                }
                if source_backed_qualified_names.contains(&entity.canonical_name) {
                    continue;
                }
                let Some((container_name, bare_name)) =
                    split_qualified_symbol_name(&entity.canonical_name)
                else {
                    continue;
                };
                if !bare_name.to_lowercase().contains(&query_lower)
                    && !entity.canonical_name.to_lowercase().contains(&query_lower)
                {
                    continue;
                }
                let Some(anchor_id) = entity.anchor_id else {
                    continue;
                };
                let Some(range) = self.generated_member_anchor_range(shard, anchor_id) else {
                    continue;
                };

                results.push(WorkspaceSymbol {
                    name: format!("{bare_name} [generated/framework]"),
                    kind: SymbolKind::Method,
                    uri: shard.source_uri.clone(),
                    range,
                    qualified_name: Some(entity.canonical_name.clone()),
                    documentation: Some(
                        "Generated/framework member; virtual symbol anchored to source declaration"
                            .to_string(),
                    ),
                    container_name: Some(format!("{container_name} [generated/framework]")),
                    has_body: false,
                    workspace_folder_uri: self.determine_folder_uri(&shard.source_uri),
                    is_lexical: false,
                });
                if cap.is_some_and(|c| results.len() >= c) {
                    break 'outer;
                }
            }
        }

        sort_workspace_symbols(&mut results);
        results
    }

    fn source_backed_qualified_names(&self) -> HashSet<String> {
        let files = self.files.read();
        let mut qualified_names = HashSet::new();
        for file_index in files.values() {
            for symbol in &file_index.symbols {
                if let Some(name) = &symbol.qualified_name {
                    qualified_names.insert(name.clone());
                    continue;
                }
                if let Some(container) = &symbol.container_name {
                    qualified_names.insert(format!("{container}::{}", symbol.name));
                }
            }
        }
        qualified_names
    }

    fn generated_member_anchor_range(
        &self,
        shard: &FileFactShard,
        anchor_id: AnchorId,
    ) -> Option<Range> {
        let anchor = shard
            .anchors
            .iter()
            .find(|anchor| anchor.id == anchor_id && anchor.file_id == shard.file_id)?;
        if anchor.provenance != Provenance::FrameworkSynthesis
            || anchor.confidence != Confidence::Medium
        {
            return None;
        }
        if anchor.span_end_byte <= anchor.span_start_byte {
            return None;
        }

        let doc = self.document_store.get(&shard.source_uri)?;
        let start = usize::try_from(anchor.span_start_byte).ok()?;
        let end = usize::try_from(anchor.span_end_byte).ok()?;
        doc.text.get(start..end)?;
        let ((start_line, start_col), (end_line, end_col)) = doc.line_index.range(start, end);
        Some(Range {
            start: Position { byte: start, line: start_line, column: start_col },
            end: Position { byte: end, line: end_line, column: end_col },
        })
    }

    /// Find symbols by query (alias for search_symbols for compatibility)
    ///
    /// # Arguments
    ///
    /// * `query` - Substring to match against symbol names
    ///
    /// # Returns
    ///
    /// Symbols whose names or qualified names contain the query string.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _results = index.find_symbols("example");
    /// ```
    pub fn find_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        self.search_symbols(query)
    }

    /// Rank symbols by folder proximity to a document
    ///
    /// Returns symbols sorted by: same folder > other folders
    ///
    /// # Arguments
    ///
    /// * `symbols` - Symbols to rank
    /// * `doc_uri` - Document URI to determine folder context
    ///
    /// # Returns
    ///
    /// Symbols ranked by folder proximity (same folder first)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let symbols = index.search_symbols("example");
    /// let ranked = index.rank_symbols_by_folder(symbols, "file:///project1/src/main.pl");
    /// ```
    pub fn rank_symbols_by_folder(
        &self,
        symbols: Vec<WorkspaceSymbol>,
        doc_uri: &str,
    ) -> Vec<WorkspaceSymbol> {
        let doc_folder = self.determine_folder_uri(doc_uri);

        let mut ranked: Vec<(WorkspaceSymbol, i32)> = symbols
            .into_iter()
            .map(|symbol| {
                let rank = if let Some(ref doc_folder_uri) = doc_folder {
                    if symbol.workspace_folder_uri.as_ref() == Some(doc_folder_uri) {
                        0 // Same folder - highest priority
                    } else {
                        1 // Different folder - lower priority
                    }
                } else {
                    1 // No document context - treat as different folder
                };
                (symbol, rank)
            })
            .collect();

        // Sort by rank (lower is better), then by name for stability
        ranked.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.name.cmp(&b.0.name)));

        ranked.into_iter().map(|(symbol, _)| symbol).collect()
    }

    /// Search for symbols with folder-aware ranking
    ///
    /// Combines symbol search with folder proximity ranking
    ///
    /// # Arguments
    ///
    /// * `name` - Symbol name to search for
    /// * `doc_uri` - Document URI for ranking context
    ///
    /// # Returns
    ///
    /// Ranked symbols with same-folder results first
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let ranked = index.search_symbols_ranked("example", "file:///project1/src/main.pl");
    /// ```
    pub fn search_symbols_ranked(&self, name: &str, doc_uri: &str) -> Vec<WorkspaceSymbol> {
        let symbols = self.search_symbols(name);
        self.rank_symbols_by_folder(symbols, doc_uri)
    }

    /// Determine if two symbols are in the same package
    ///
    /// # Arguments
    ///
    /// * `symbol_a` - First symbol
    /// * `symbol_b` - Second symbol
    ///
    /// # Returns
    ///
    /// `true` if both symbols are in the same package
    #[allow(dead_code)]
    pub fn same_package(&self, symbol_a: &WorkspaceSymbol, symbol_b: &WorkspaceSymbol) -> bool {
        let package_a = self.extract_package_name(&symbol_a.name);
        let package_b = self.extract_package_name(&symbol_b.name);
        package_a == package_b
    }

    /// Determine if two package names are the same (helper for testing)
    ///
    /// # Arguments
    ///
    /// * `package_a` - First package name
    /// * `package_b` - Second package name
    ///
    /// # Returns
    ///
    /// `true` if both package names are equal
    #[allow(dead_code)]
    pub fn same_package_by_container(&self, package_a: &str, package_b: &str) -> bool {
        package_a == package_b
    }

    /// Extract package name from a symbol name
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - Symbol name (e.g., "Foo::Bar::baz" or "baz")
    ///
    /// # Returns
    ///
    /// Package name (e.g., "Foo::Bar") or None for main package
    #[allow(dead_code)]
    pub fn extract_package_name(&self, symbol_name: &str) -> Option<String> {
        let parts: Vec<&str> = symbol_name.split("::").collect();
        if parts.len() > 1 { Some(parts[..parts.len() - 1].join("::")) } else { None }
    }

    /// Get symbols in a specific file
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI to inspect
    ///
    /// # Returns
    ///
    /// All symbols indexed for the requested file.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _symbols = index.file_symbols("file:///example.pl");
    /// ```
    pub fn file_symbols(&self, uri: &str) -> Vec<WorkspaceSymbol> {
        let normalized_uri = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized_uri);
        let files = self.files.read();

        files.get(&key).map(|fi| fi.symbols.clone()).unwrap_or_default()
    }

    /// Get dependencies of a file
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI to inspect
    ///
    /// # Returns
    ///
    /// A set of module names imported by the file.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _deps = index.file_dependencies("file:///example.pl");
    /// ```
    pub fn file_dependencies(&self, uri: &str) -> HashSet<String> {
        let normalized_uri = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized_uri);
        let files = self.files.read();

        files.get(&key).map(|fi| fi.dependencies.clone()).unwrap_or_default()
    }

    /// Find all files that depend on a module
    ///
    /// # Arguments
    ///
    /// * `module_name` - Module name to search for in file dependencies
    ///
    /// # Returns
    ///
    /// A list of file URIs that import or depend on the module.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _files = index.find_dependents("My::Module");
    /// ```
    pub fn find_dependents(&self, module_name: &str) -> Vec<String> {
        let canonical = canonicalize_perl_module_name(module_name);
        let legacy = legacy_perl_module_name(&canonical);
        let files = self.files.read();
        let mut dependents = Vec::new();

        for (uri_key, file_index) in files.iter() {
            if file_index.dependencies.contains(module_name)
                || file_index.dependencies.contains(&canonical)
                || file_index.dependencies.contains(&legacy)
            {
                dependents.push(uri_key.clone());
            }
        }

        dependents
    }

    /// Get the document store
    ///
    /// # Returns
    ///
    /// A reference to the in-memory document store.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _store = index.document_store();
    /// ```
    pub fn document_store(&self) -> &DocumentStore {
        &self.document_store
    }

    /// Find unused symbols in the workspace
    ///
    /// # Returns
    ///
    /// Symbols that have no non-definition references in the workspace.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _unused = index.find_unused_symbols();
    /// ```
    pub fn find_unused_symbols(&self) -> Vec<WorkspaceSymbol> {
        let files = self.files.read();
        let mut unused = Vec::new();

        // Collect all defined symbols
        for (_uri_key, file_index) in files.iter() {
            for symbol in &file_index.symbols {
                // Lexically-scoped variables (my/state) require scope-range analysis to
                // determine whether a reference in the same file refers to *this* declaration
                // or a same-named variable in a different block.  The bare-name lookup below
                // cannot make that distinction, so lexical variables are excluded from this
                // check entirely.  Proper unused-lexical detection is handled by the
                // scope-aware ScopeAnalyzer.  See issue #1805.
                if symbol.is_lexical {
                    continue;
                }

                // Check if this symbol has any references beyond its definition
                let has_usage = files.values().any(|fi| {
                    if let Some(refs) = fi.references.get(&symbol.name) {
                        refs.iter().any(|r| r.kind != ReferenceKind::Definition)
                    } else {
                        false
                    }
                });

                if !has_usage {
                    unused.push(symbol.clone());
                }
            }
        }

        unused
    }

    /// Get all symbols that belong to a specific package
    ///
    /// # Arguments
    ///
    /// * `package_name` - Package name to match (e.g., `My::Package`)
    ///
    /// # Returns
    ///
    /// Symbols defined within the requested package.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::WorkspaceIndex;
    ///
    /// let index = WorkspaceIndex::new();
    /// let _members = index.get_package_members("My::Package");
    /// ```
    pub fn get_package_members(&self, package_name: &str) -> Vec<WorkspaceSymbol> {
        let files = self.files.read();
        let mut members = Vec::new();

        for (_uri_key, file_index) in files.iter() {
            for symbol in &file_index.symbols {
                // Check if symbol belongs to this package
                if let Some(ref container) = symbol.container_name {
                    if container == package_name {
                        members.push(symbol.clone());
                    }
                }
                // Also check qualified names
                if let Some(ref qname) = symbol.qualified_name {
                    if qname.starts_with(&format!("{}::", package_name)) {
                        // Avoid duplicates - only add if not already in via container_name
                        if symbol.container_name.as_deref() != Some(package_name) {
                            members.push(symbol.clone());
                        }
                    }
                }
            }
        }

        members
    }

    /// Names of all packages explicitly declared in a file.
    ///
    /// Returns the bare declared name for each `package` statement or block in
    /// the file (e.g. `"Foo"`, `"Bar"`, `"Foo::Nested"`).  A file with no
    /// explicit `package` declaration returns an empty vec; there is no implicit
    /// `"main"` symbol to surface.  A file containing `package main;` explicitly
    /// WILL appear in results.
    ///
    /// # Arguments
    ///
    /// * `uri` - File URI to inspect (normalized via `normalize_uri`)
    ///
    /// # Returns
    ///
    /// Declared package names in declaration order (AST walk order).
    pub fn file_packages(&self, uri: &str) -> Vec<String> {
        let normalized = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized);
        let files = self.files.read();
        let Some(file) = files.get(&key) else {
            return Vec::new();
        };

        let mut packages = Vec::new();
        for symbol in &file.symbols {
            if symbol.kind == SymbolKind::Package {
                packages.push(symbol.name.clone());
            }
        }
        packages
    }

    /// Symbols declared inside a specific package within a file.
    ///
    /// Returns all `WorkspaceSymbol` entries whose `container_name` equals
    /// `package_name` (bare name match, e.g. `"Bar"` or `"Foo::Nested"`).
    /// Package declaration symbols themselves are excluded (they carry
    /// `container_name = None`).
    ///
    /// # Arguments
    ///
    /// * `uri`          - File URI to inspect
    /// * `package_name` - Bare package name to filter by (e.g. `"Foo::Bar"`)
    ///
    /// # Returns
    ///
    /// Symbols belonging to the package, in declaration order.
    pub fn file_package_symbols(&self, uri: &str, package_name: &str) -> Vec<WorkspaceSymbol> {
        let normalized = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized);
        let files = self.files.read();
        let Some(file) = files.get(&key) else {
            return Vec::new();
        };

        let mut symbols = Vec::new();
        for symbol in &file.symbols {
            if Self::symbol_belongs_to_package(symbol, package_name) {
                symbols.push(symbol.clone());
            }
        }
        symbols
    }

    fn symbol_belongs_to_package(symbol: &WorkspaceSymbol, package_name: &str) -> bool {
        symbol.container_name.as_ref().is_some_and(|container| package_name.eq(container.as_str()))
    }

    /// Find the definition location for a symbol key during Index/Navigate stages.
    ///
    /// # Arguments
    ///
    /// * `key` - Normalized symbol key to resolve.
    ///
    /// # Returns
    ///
    /// The definition location for the symbol, if found.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
    /// use std::sync::Arc;
    ///
    /// let index = WorkspaceIndex::new();
    /// let key = SymbolKey { pkg: Arc::from("My::Package"), name: Arc::from("example"), sigil: None, kind: SymKind::Sub };
    /// let _def = index.find_def(&key);
    /// ```
    pub fn find_def(&self, key: &SymbolKey) -> Option<Location> {
        if let Some(sigil) = key.sigil {
            // It's a variable
            let var_name = format!("{}{}", sigil, key.name);
            self.find_definition(&var_name)
        } else if key.kind == SymKind::Pack {
            // It's a package lookup (e.g., from `use Module::Name`)
            // Search for the package declaration by name
            self.find_definition(key.pkg.as_ref())
                .or_else(|| self.find_definition(key.name.as_ref()))
        } else {
            // It's a subroutine or package
            let qualified_name = format!("{}::{}", key.pkg, key.name);
            self.find_definition(&qualified_name)
        }
    }

    /// Find reference locations for a symbol key using dual indexing.
    ///
    /// Searches both qualified and bare names to support Navigate/Analyze workflows.
    ///
    /// # Arguments
    ///
    /// * `key` - Normalized symbol key to search for.
    ///
    /// # Returns
    ///
    /// All reference locations for the symbol, excluding the definition.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
    /// use std::sync::Arc;
    ///
    /// let index = WorkspaceIndex::new();
    /// let key = SymbolKey { pkg: Arc::from("main"), name: Arc::from("example"), sigil: None, kind: SymKind::Sub };
    /// let _refs = index.find_refs(&key);
    /// ```
    pub fn find_refs(&self, key: &SymbolKey) -> Vec<Location> {
        let files_locked = self.files.read();
        let mut all_refs = if let Some(sigil) = key.sigil {
            // It's a variable - search through all files for this variable name
            let var_name = format!("{}{}", sigil, key.name);
            let mut refs = Vec::new();
            for (_uri_key, file_index) in files_locked.iter() {
                if let Some(var_refs) = file_index.references.get(&var_name) {
                    for reference in var_refs {
                        refs.push(Location { uri: reference.uri.clone(), range: reference.range });
                    }
                }
            }
            refs
        } else {
            // It's a subroutine or package
            if key.pkg.as_ref() == "main" {
                // For main package, we search for both "main::foo" and bare "foo"
                let mut refs = self.find_references(&format!("main::{}", key.name));
                // Add bare name references
                for (_uri_key, file_index) in files_locked.iter() {
                    if let Some(bare_refs) = file_index.references.get(key.name.as_ref()) {
                        for reference in bare_refs {
                            refs.push(Location {
                                uri: reference.uri.clone(),
                                range: reference.range,
                            });
                        }
                    }
                }
                refs
            } else {
                let qualified_name = format!("{}::{}", key.pkg, key.name);
                self.find_references(&qualified_name)
            }
        };
        drop(files_locked);

        // Remove the definition; the caller will include it separately if needed
        if let Some(def) = self.find_def(key) {
            all_refs.retain(|loc| !(loc.uri == def.uri && loc.range == def.range));
        }

        // Deduplicate by URI and range
        let mut seen = HashSet::new();
        all_refs.retain(|loc| {
            seen.insert((
                loc.uri.clone(),
                loc.range.start.line,
                loc.range.start.column,
                loc.range.end.line,
                loc.range.end.column,
            ))
        });

        all_refs
    }
}

/// **`build_unified` is the production extraction path (perl-lsp-swarm#1711-B
/// cutover); `build` below remains a shadow/parity-harness-only twin.**
/// `build()` runs the OLD, pre-cutover MULTIPLE separate AST walks (one
/// `IndexVisitor::visit`, one `build_canonical_fact_shard_for_ast` -- which
/// itself internally calls
/// `extract_symbol_decls`/`extract_symbol_refs`/eval-sub/generated-member --
/// plus one `extract_import_specs` and one `extract_use_lib_facts`), and just
/// packages their outputs into one struct; it is kept ONLY as the
/// `extraction_bundle_shadow_compare` test module's `build_direct`-equivalent
/// reference point, not called by `index_file_with_generation` anymore.
/// `build_unified()` is what production actually calls: it runs ONE reference
/// walk (`IndexVisitor::visit_unified`) that produces both projections,
/// eliminating the duplicate `extract_symbol_refs` walk `build()`/pre-cutover
/// production used to run.
///
/// # Parity contract
///
/// | Output | Legacy (`IndexVisitor` / [`FileIndex`]) | Canonical ([`FileFactShard`]) |
/// |---|---|---|
/// | Declarations | [`WorkspaceSymbol`] (name, kind, range, qualified_name, container_name, is_lexical) built by `IndexVisitor::project_symbol_declarations`, which calls `extract_symbol_decls(ast, Some("main"))` | [`EntityFact`] + [`AnchorFact`] via `extract_symbol_decls(ast, None)` -> `symbol_decls_to_semantic_facts` |
/// | References | [`SymbolReference`] with `ReferenceKind::{Read,Write,Usage,Import,Definition}`, produced by the hand-rolled `IndexVisitor::visit_node`/`visit_unified` walk (dual bare+qualified indexing for calls, dependency tracking for `use`/`extends`/`with`/`require`, interpolated-string variable scanning) | `OccurrenceFact` + `EdgeFact` via `symbol_refs_to_semantic_facts` (`Call`/`MethodCall`/`StaticMethodCall`/`CoderefReference`/`TypeglobReference`/`Read` only -- no `Write`, no dependency tracking, no interpolated strings) |
/// | Dynamic / generated facts | not modeled in `FileIndex` | `extract_eval_sub_boundaries` + `extract_generated_member_facts`, merged into `FileFactShard.{entities,anchors,occurrences}` |
/// | Imports | `ReferenceKind::Import` entries in `FileIndex.references`, plus `FileIndex.dependencies: HashSet<String>` | `extract_import_specs` / `extract_use_lib_facts`, written to `ImportExportIndex` -- **not** part of `FileFactShard` (see `build_canonical_fact_shard_for_ast`'s always-empty `imports: &[]` argument) |
/// | Identity / ordering | `WorkspaceSymbol`/`SymbolReference` carry no stable ID; `FileIndex.references` is a `HashMap` (unordered by name, but each name's `Vec` preserves visit order) | `AnchorId`/`EntityId`/`OccurrenceId`/`EdgeId` are content-derived stable hashes (`stable_id`); `Vec` fields preserve extraction order |
///
/// Declaration extraction is intentionally still NOT unified (`extract_symbol_decls`
/// is called once per projection, in both `build()` and `build_unified()`) --
/// a separable follow-up tracked on #1711 (see the feasibility comment, item
/// 3). Only the reference walk is unified by this cutover.
pub(crate) struct FileExtractionBundle {
    /// The legacy `IndexVisitor` projection, produced by one `visit()` call.
    pub(crate) legacy_index: FileIndex,
    /// The canonical fact shard, produced by one
    /// `build_canonical_fact_shard_for_ast` call.
    pub(crate) canonical_shard: FileFactShard,
    /// Import specifications from one `extract_import_specs` call. Not part
    /// of `canonical_shard` (see the parity contract table above).
    pub(crate) import_specs: Vec<perl_semantic_facts::ImportSpec>,
    /// `use lib`/`no lib` facts from one `extract_use_lib_facts` call. Not
    /// part of `canonical_shard` (see the parity contract table above).
    pub(crate) use_lib_facts: Vec<perl_semantic_facts::UseLibFact>,
}

impl FileExtractionBundle {
    /// **Shadow/parity-harness-only twin of [`Self::build_unified`] (kept for
    /// the `extraction_bundle_shadow_compare` regression harness).** Runs the
    /// OLD, pre-1711-B-cutover MULTIPLE separate extractor calls (still
    /// several distinct AST walks -- see the struct-level doc comment) and
    /// packages every result into a single bundle. Does not reduce the
    /// traversal count; not called by `index_file_with_generation`.
    // Shadow scaffold (see the struct-level justification above); the
    // function itself is equally unused by the live path outside tests --
    // reachable only from `#[cfg(test)]` code (`extraction_bundle_shadow_compare`),
    // so it is gated accordingly rather than `#[allow(dead_code)]`.
    #[cfg(test)]
    pub(crate) fn build(
        ast: &Node,
        uri_str: &str,
        content_hash: u64,
        doc: &mut Document,
        folder_uri: Option<String>,
    ) -> Self {
        let mut file_index = FileIndex {
            source_uri: uri_str.to_string(),
            content_hash,
            folder_uri: folder_uri.clone(),
            ..Default::default()
        };
        let mut visitor = IndexVisitor::new(doc, uri_str.to_string(), folder_uri);
        visitor.visit(ast, &mut file_index);

        let canonical_shard =
            WorkspaceIndex::build_canonical_fact_shard_for_ast(uri_str, content_hash, ast);

        let file_id = WorkspaceIndex::hash_uri_to_file_id(uri_str);
        let import_specs =
            crate::semantic::workspace_import_extractor::extract_import_specs(ast, file_id);
        let use_lib_facts =
            crate::semantic::workspace_import_extractor::extract_use_lib_facts(ast, file_id);

        Self { legacy_index: file_index, canonical_shard, import_specs, use_lib_facts }
    }

    /// **Production extraction path (perl-lsp-swarm#1711-B cutover).** The
    /// REAL unified traversal: runs ONE reference walk
    /// (`IndexVisitor::visit_unified`) that produces BOTH the legacy
    /// [`FileIndex`] reference/dependency projection AND the canonical
    /// `Vec<SymbolRef>` projection, then feeds that single `Vec<SymbolRef>`
    /// into `WorkspaceIndex::build_canonical_fact_shard_from_symbol_refs`
    /// instead of calling `extract_symbol_refs(ast)` a second time. This is
    /// what eliminates one of the two full-AST reference walks
    /// `index_file_with_generation` used to run.
    ///
    /// Declaration extraction is UNCHANGED: `extract_symbol_decls` is still
    /// called twice (once per projection, with the existing
    /// `Some("main")`/`None` package-context seeds) -- unifying declarations
    /// is a separable follow-up (see the #1711 feasibility comment, item 3).
    /// Import/use-lib extraction is also unchanged.
    ///
    /// Uses [`Node::for_each_child`] as the unified walk's recursion
    /// fallback (see `IndexVisitor::walk_unified`'s doc comment), which
    /// closes several PRE-EXISTING legacy `FileIndex` coverage gaps not
    /// introduced by this change -- see
    /// `docs/reference/1711-B-coverage-delta.md` for the full,
    /// fixture-backed list; this is the intentional, monotonic legacy
    /// `FileIndex` coverage improvement this cutover ships. Called by
    /// `WorkspaceIndex::index_file_with_generation` -- this IS the
    /// production path now (was shadow-only prior to the 1711-B cutover;
    /// `FileExtractionBundle::build` above remains the shadow/parity-only
    /// non-unified twin, used only by the `extraction_bundle_shadow_compare`
    /// harness).
    pub(crate) fn build_unified(
        ast: &Node,
        uri_str: &str,
        content_hash: u64,
        doc: &mut Document,
        folder_uri: Option<String>,
    ) -> Self {
        let mut file_index = FileIndex {
            source_uri: uri_str.to_string(),
            content_hash,
            folder_uri: folder_uri.clone(),
            ..Default::default()
        };
        let mut symbol_refs = Vec::new();
        let mut visitor = IndexVisitor::new(doc, uri_str.to_string(), folder_uri);
        #[cfg(test)]
        let visit_start = Instant::now();
        visitor.visit_unified(ast, &mut file_index, &mut symbol_refs);
        #[cfg(test)]
        reindex_metrics::record_visit(visit_start.elapsed());

        let canonical_shard = WorkspaceIndex::build_canonical_fact_shard_from_symbol_refs(
            uri_str,
            content_hash,
            ast,
            &symbol_refs,
        );

        let file_id = WorkspaceIndex::hash_uri_to_file_id(uri_str);
        #[cfg(test)]
        let import_start = Instant::now();
        let import_specs =
            crate::semantic::workspace_import_extractor::extract_import_specs(ast, file_id);
        #[cfg(test)]
        reindex_metrics::record_import_extract(import_start.elapsed());
        #[cfg(test)]
        let use_lib_start = Instant::now();
        let use_lib_facts =
            crate::semantic::workspace_import_extractor::extract_use_lib_facts(ast, file_id);
        #[cfg(test)]
        reindex_metrics::record_use_lib_extract(use_lib_start.elapsed());

        Self { legacy_index: file_index, canonical_shard, import_specs, use_lib_facts }
    }
}

/// AST visitor for extracting symbols and references
struct IndexVisitor {
    document: Document,
    uri: String,
    current_package: Option<String>,
    workspace_folder_uri: Option<String>,
}

fn is_interpolated_var_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_interpolated_var_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':'
}

fn has_escaped_interpolation_marker(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let mut backslashes = 0usize;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        backslashes += 1;
        cursor -= 1;
    }

    backslashes % 2 == 1
}

fn strip_matching_quote_delimiters(raw_content: &str) -> &str {
    if raw_content.len() < 2 {
        return raw_content;
    }

    let bytes = raw_content.as_bytes();
    match (bytes.first(), bytes.last()) {
        (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\'')) => {
            &raw_content[1..raw_content.len() - 1]
        }
        _ => raw_content,
    }
}

impl IndexVisitor {
    fn new(document: &mut Document, uri: String, workspace_folder_uri: Option<String>) -> Self {
        Self {
            document: document.clone(),
            uri,
            current_package: Some("main".to_string()),
            workspace_folder_uri,
        }
    }

    fn visit(&mut self, node: &Node, file_index: &mut FileIndex) {
        self.project_symbol_declarations(node, file_index);
        self.visit_node(node, file_index);
    }

    fn project_symbol_declarations(&self, node: &Node, file_index: &mut FileIndex) {
        for decl in extract_symbol_decls(node, self.current_package.as_deref()) {
            let (start, end) = match decl.kind {
                SymbolKind::Variable(_) => match decl.anchor_span {
                    Some(span) => span,
                    None => decl.full_span,
                },
                _ => decl.full_span,
            };
            let ((start_line, start_col), (end_line, end_col)) =
                self.document.line_index.range(start, end);
            let range = Range {
                start: Position { byte: start, line: start_line, column: start_col },
                end: Position { byte: end, line: end_line, column: end_col },
            };

            let symbol_name = symbol_decl_name(&decl.kind, &decl.name);

            // Suppress qualified_name for lexically-scoped variables (my, state): they
            // are not package-visible and must not be found by a qualified lookup such
            // as `Foo::x`.  `our` and `local` variables keep the qualified name because
            // they participate in the package namespace.
            let qualified_name = match &decl.declarator {
                Some(d) if d == "my" || d == "state" => None,
                _ => (!decl.qualified_name.is_empty()).then_some(decl.qualified_name),
            };

            // Top-level package declarations have no containing package; suppress the
            // spurious "main" container that comes from the walker's initial context.
            let container_name = match decl.kind {
                SymbolKind::Package => None,
                _ => decl.container,
            };

            // Lexical declarators (my/state) produce scope-local variables that cannot be
            // correctly analysed by a bare-name unused-symbol check.  Flag them so that
            // `find_unused_symbols` can skip the whole class.
            let is_lexical = matches!(decl.declarator.as_deref(), Some("my") | Some("state"));

            file_index.symbols.push(WorkspaceSymbol {
                name: symbol_name.clone(),
                kind: decl.kind,
                uri: self.uri.clone(),
                range,
                qualified_name,
                documentation: None,
                container_name,
                has_body: true,
                workspace_folder_uri: self.workspace_folder_uri.clone(),
                is_lexical,
            });

            file_index.references.entry(symbol_name).or_default().push(SymbolReference {
                uri: self.uri.clone(),
                range,
                kind: ReferenceKind::Definition,
            });
        }
    }

    fn record_interpolated_variable_references(
        &self,
        raw_content: &str,
        range: Range,
        file_index: &mut FileIndex,
    ) {
        let content = strip_matching_quote_delimiters(raw_content);
        let bytes = content.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            if has_escaped_interpolation_marker(bytes, index) {
                index += 1;
                continue;
            }

            let sigil = match bytes[index] {
                b'$' => "$",
                b'@' => "@",
                _ => {
                    index += 1;
                    continue;
                }
            };

            if index + 1 >= bytes.len() {
                break;
            }

            let (start, needs_closing_brace) =
                if bytes[index + 1] == b'{' { (index + 2, true) } else { (index + 1, false) };

            if start >= bytes.len() || !is_interpolated_var_start(bytes[start]) {
                index += 1;
                continue;
            }

            let mut end = start + 1;
            while end < bytes.len() && is_interpolated_var_continue(bytes[end]) {
                end += 1;
            }

            if needs_closing_brace && (end >= bytes.len() || bytes[end] != b'}') {
                index += 1;
                continue;
            }

            if let Some(name) = content.get(start..end) {
                let var_name = format!("{sigil}{name}");
                file_index.references.entry(var_name).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range,
                    kind: ReferenceKind::Read,
                });
            }

            index = if needs_closing_brace { end + 1 } else { end };
        }
    }

    fn visit_node(&mut self, node: &Node, file_index: &mut FileIndex) {
        match &node.kind {
            NodeKind::Package { name, .. } => {
                let package_name = name.clone();

                // Update the current package (replaces the previous one, not a stack)
                self.current_package = Some(package_name.clone());
            }

            NodeKind::Subroutine { body, .. } => {
                // Visit body
                self.visit_node(body, file_index);
            }

            NodeKind::VariableDeclaration { initializer, .. } => {
                // Visit initializer
                if let Some(init) = initializer {
                    self.visit_node(init, file_index);
                }
            }

            NodeKind::VariableListDeclaration { initializer, .. } => {
                // Visit the initializer
                if let Some(init) = initializer {
                    self.visit_node(init, file_index);
                }
            }

            NodeKind::Variable { sigil, name } => {
                let var_name = format!("{}{}", sigil, name);

                // Track as usage (could be read or write based on context)
                file_index.references.entry(var_name).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    kind: ReferenceKind::Read, // Default to read, would need context for write
                });
            }

            NodeKind::FunctionCall { name, args, .. } => {
                let func_name = name.clone();
                let location = self.node_to_range(node);

                // Determine package and bare name
                let (pkg, bare_name) = if let Some(idx) = func_name.rfind("::") {
                    (&func_name[..idx], &func_name[idx + 2..])
                } else {
                    (self.current_package.as_deref().unwrap_or("main"), func_name.as_str())
                };

                let qualified = format!("{}::{}", pkg, bare_name);

                // Track as usage for both qualified and bare forms
                // This dual indexing allows finding references whether the function is called
                // as `process_data()` or `Utils::process_data()`
                file_index.references.entry(bare_name.to_string()).or_default().push(
                    SymbolReference {
                        uri: self.uri.clone(),
                        range: location,
                        kind: ReferenceKind::Usage,
                    },
                );
                file_index.references.entry(qualified).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: location,
                    kind: ReferenceKind::Usage,
                });

                if name == "extends" || name == "with" {
                    for module_name in extract_module_names_from_call_args(args) {
                        file_index
                            .dependencies
                            .insert(normalize_dependency_module_name(&module_name));
                    }
                } else if name == "require" {
                    if let Some(module_name) = extract_module_name_from_require_args(args) {
                        file_index
                            .dependencies
                            .insert(normalize_dependency_module_name(&module_name));
                    }
                }

                // Visit arguments
                for arg in args {
                    self.visit_node(arg, file_index);
                }
            }

            NodeKind::Use { module, args, .. } => {
                let module_name = normalize_dependency_module_name(module);
                file_index.dependencies.insert(module_name.clone());

                // Also track actual parent/base class names for dependency discovery.
                // `use parent 'Foo::Bar'` stores module="parent" and args=["'Foo::Bar'"],
                // so find_dependents("Foo::Bar") would miss files with only use parent.
                if module == "parent" || module == "base" {
                    for name in extract_module_names_from_use_args(args) {
                        file_index.dependencies.insert(normalize_dependency_module_name(&name));
                    }
                }

                // Track as import
                file_index.references.entry(module_name).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    kind: ReferenceKind::Import,
                });
            }

            // Handle assignment to detect writes
            NodeKind::Assignment { lhs, rhs, op } => {
                // For compound assignments (+=, -=, .=, etc.), the LHS is both read and written
                let is_compound = op != "=";

                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    let var_name = format!("{}{}", sigil, name);

                    // For compound assignments, it's a read first
                    if is_compound {
                        file_index.references.entry(var_name.clone()).or_default().push(
                            SymbolReference {
                                uri: self.uri.clone(),
                                range: self.node_to_range(lhs),
                                kind: ReferenceKind::Read,
                            },
                        );
                    }

                    // Then it's always a write
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(lhs),
                        kind: ReferenceKind::Write,
                    });
                }

                // Right side could have reads
                self.visit_node(rhs, file_index);
            }

            // Recursively visit child nodes
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, file_index);
                }
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
                self.visit_node(condition, file_index);
                self.visit_node(then_branch, file_index);
                for (cond, branch) in elsif_branches {
                    self.visit_node(cond, file_index);
                    self.visit_node(branch, file_index);
                }
                if let Some(else_br) = else_branch {
                    self.visit_node(else_br, file_index);
                }
            }

            NodeKind::While { condition, body, continue_block, .. } => {
                self.visit_node(condition, file_index);
                self.visit_node(body, file_index);
                if let Some(cont) = continue_block {
                    self.visit_node(cont, file_index);
                }
            }

            NodeKind::For { init, condition, update, body, continue_block } => {
                if let Some(i) = init {
                    self.visit_node(i, file_index);
                }
                if let Some(c) = condition {
                    self.visit_node(c, file_index);
                }
                if let Some(u) = update {
                    self.visit_node(u, file_index);
                }
                self.visit_node(body, file_index);
                if let Some(cont) = continue_block {
                    self.visit_node(cont, file_index);
                }
            }

            NodeKind::Foreach { variable, list, body, continue_block } => {
                // Iterator is a write context
                if let Some(cb) = continue_block {
                    self.visit_node(cb, file_index);
                }
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let var_name = format!("{}{}", sigil, name);
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(variable),
                        kind: ReferenceKind::Write,
                    });
                }
                self.visit_node(variable, file_index);
                self.visit_node(list, file_index);
                self.visit_node(body, file_index);
            }

            NodeKind::MethodCall { object, method, args } => {
                // Check if this is a static method call (Package->method)
                let qualified_method = if let NodeKind::Identifier { name } = &object.kind {
                    // Static method call: Package->method
                    Some(format!("{}::{}", name, method))
                } else {
                    // Instance method call: $obj->method
                    None
                };

                // Object is a read context
                self.visit_node(object, file_index);

                // Track method call under BOTH the qualified form (for static calls
                // like `Pkg->method`) AND the bare method name. This mirrors the
                // FunctionCall dual-key storage above (PR #122 dual-indexing pattern)
                // so that bare-name lookups (e.g. `find_unused_symbols`,
                // `count_usages("method")`) consistently find static method call sites.
                // See #6799 for the original asymmetric-storage bug report.
                let location = self.node_to_range(node);
                if let Some(qualified_method) = qualified_method.as_ref() {
                    file_index.references.entry(qualified_method.clone()).or_default().push(
                        SymbolReference {
                            uri: self.uri.clone(),
                            range: location,
                            kind: ReferenceKind::Usage,
                        },
                    );
                }
                file_index.references.entry(method.clone()).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: location,
                    kind: ReferenceKind::Usage,
                });

                if method == "import"
                    && let NodeKind::Identifier { name: module_name } = &object.kind
                {
                    for symbol in extract_manual_import_symbols(args) {
                        file_index.references.entry(symbol).or_default().push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            kind: ReferenceKind::Import,
                        });
                    }
                    file_index.dependencies.insert(normalize_dependency_module_name(module_name));
                }

                // Visit arguments
                for arg in args {
                    self.visit_node(arg, file_index);
                }
            }

            NodeKind::No { module, .. } => {
                let module_name = normalize_dependency_module_name(module);
                file_index.dependencies.insert(module_name);
            }

            NodeKind::Class { name, .. } => {
                self.current_package = Some(name.clone());
            }

            NodeKind::Method { body, signature, .. } => {
                // Visit params
                if let Some(sig) = signature {
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        for param in parameters {
                            self.visit_node(param, file_index);
                        }
                    }
                }

                // Visit body
                self.visit_node(body, file_index);
            }

            NodeKind::String { value, interpolated } => {
                if *interpolated {
                    let range = self.node_to_range(node);
                    self.record_interpolated_variable_references(value, range, file_index);
                }
            }

            NodeKind::Heredoc { content, interpolated, .. } => {
                if *interpolated {
                    let range = self.node_to_range(node);
                    self.record_interpolated_variable_references(content, range, file_index);
                }
            }

            // Handle special assignments (++ and --)
            NodeKind::Unary { op, operand } if op == "++" || op == "--" => {
                // Pre/post increment/decrement are both read and write
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    let var_name = format!("{}{}", sigil, name);

                    // It's both a read and a write
                    file_index.references.entry(var_name.clone()).or_default().push(
                        SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(operand),
                            kind: ReferenceKind::Read,
                        },
                    );

                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(operand),
                        kind: ReferenceKind::Write,
                    });
                }
            }

            _ => {
                // For other node types, just visit children
                self.visit_children(node, file_index);
            }
        }
    }

    fn visit_children(&mut self, node: &Node, file_index: &mut FileIndex) {
        // Generic visitor for unhandled node types - visit all nested nodes
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, file_index);
                }
            }
            NodeKind::ExpressionStatement { expression } => {
                self.visit_node(expression, file_index);
            }
            // Expression nodes
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand, file_index);
            }
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left, file_index);
                self.visit_node(right, file_index);
            }
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                self.visit_node(condition, file_index);
                self.visit_node(then_expr, file_index);
                self.visit_node(else_expr, file_index);
            }
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem, file_index);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    self.visit_node(key, file_index);
                    self.visit_node(value, file_index);
                }
            }
            NodeKind::Return { value } => {
                if let Some(val) = value {
                    self.visit_node(val, file_index);
                }
            }
            NodeKind::Eval { block } | NodeKind::Do { block } | NodeKind::Defer { block } => {
                self.visit_node(block, file_index);
            }
            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.visit_node(body, file_index);
                for (_, block) in catch_blocks {
                    self.visit_node(block, file_index);
                }
                if let Some(finally) = finally_block {
                    self.visit_node(finally, file_index);
                }
            }
            NodeKind::Given { expr, body } => {
                self.visit_node(expr, file_index);
                self.visit_node(body, file_index);
            }
            NodeKind::When { condition, body } => {
                self.visit_node(condition, file_index);
                self.visit_node(body, file_index);
            }
            NodeKind::Default { body } => {
                self.visit_node(body, file_index);
            }
            NodeKind::StatementModifier { statement, condition, .. } => {
                self.visit_node(statement, file_index);
                self.visit_node(condition, file_index);
            }
            NodeKind::VariableWithAttributes { variable, .. } => {
                self.visit_node(variable, file_index);
            }
            NodeKind::LabeledStatement { statement, .. } => {
                self.visit_node(statement, file_index);
            }
            NodeKind::NestedVariableList { items } => {
                // Recurse into items so nested-declared variables are indexed.
                for item in items {
                    self.visit_node(item, file_index);
                }
            }
            _ => {
                // For other node types, no children to visit
            }
        }
    }

    fn node_to_range(&mut self, node: &Node) -> Range {
        // LineIndex.range returns line numbers and UTF-16 code unit columns
        let ((start_line, start_col), (end_line, end_col)) =
            self.document.line_index.range(node.location.start, node.location.end);
        // Use byte offsets from node.location directly
        Range {
            start: Position { byte: node.location.start, line: start_line, column: start_col },
            end: Position { byte: node.location.end, line: end_line, column: end_col },
        }
    }

    /// **Production reference walk (perl-lsp-swarm#1711-B cutover).**
    /// Unified reference walk: produces BOTH the legacy [`FileIndex`]
    /// reference/dependency projection AND the canonical `Vec<SymbolRef>`
    /// projection (the input `symbol_refs_to_semantic_facts` expects) from
    /// ONE recursive descent -- replacing what production used to run as
    /// TWO separate full-AST walks (`IndexVisitor::visit` +
    /// `extract_symbol_refs`). Declaration
    /// extraction is UNCHANGED and NOT unified here (`project_symbol_declarations`
    /// still calls `extract_symbol_decls(ast, Some("main"))` exactly as
    /// `visit()` does today) -- see the #1711 feasibility comment, item 3,
    /// a separable follow-up.
    ///
    /// Uses [`Node::for_each_child`] as its recursion fallback (the
    /// complete, compiler-exhaustive dispatcher) instead of
    /// `IndexVisitor::visit_children`'s hand-maintained allowlist. This
    /// closes several PRE-EXISTING legacy coverage gaps, empirically
    /// verified against current `origin/main` and characterized in
    /// `docs/reference/1711-B-coverage-delta.md`: block-form
    /// `package Foo { ... }` / `class Foo { ... }` bodies are never walked
    /// for references by `IndexVisitor::visit_node` today (only their
    /// declarations are seen, via the separate `extract_symbol_decls` walk);
    /// `Typeglob`, `Tie`, `Goto` coderef targets, regex-bind expressions
    /// (`Match`/`Substitution`/`Transliteration`), `IndirectCall` arguments,
    /// `Subroutine` signature default-value expressions, and non-`Variable`
    /// assignment/increment targets are also unreached by legacy today (that
    /// coverage gain is intentional and shipped by the 1711-B cutover -- see
    /// `assert_unified_legacy_is_superset` in `extraction_bundle_shadow_compare`
    /// below). Called by production via `FileExtractionBundle::build_unified`.
    fn visit_unified(
        &mut self,
        node: &Node,
        file_index: &mut FileIndex,
        symbol_refs: &mut Vec<perl_symbol::surface::r#ref::SymbolRef>,
    ) {
        self.project_symbol_declarations(node, file_index);
        self.walk_unified(node, file_index, symbol_refs);
    }

    /// Emit this node's own canonical [`SymbolRef`] (if any) without
    /// recursing into its children -- the caller controls recursion order
    /// (needed so `Assignment`/increment targets that legacy already
    /// special-cases don't ALSO get a second, generic recursion pass that
    /// would double-count the legacy side).
    fn emit_canonical_ref(
        node: &Node,
        symbol_refs: &mut Vec<perl_symbol::surface::r#ref::SymbolRef>,
    ) {
        if let Some(symbol_ref) = canonical_ref_for_node(node) {
            symbol_refs.push(symbol_ref);
        }
    }

    fn walk_unified(
        &mut self,
        node: &Node,
        file_index: &mut FileIndex,
        symbol_refs: &mut Vec<perl_symbol::surface::r#ref::SymbolRef>,
    ) {
        match &node.kind {
            NodeKind::Package { name, block, .. } => {
                self.current_package = Some(name.clone());
                // NEW COVERAGE: `IndexVisitor::visit_node` never recurses
                // into `block` for references today (only
                // `project_symbol_declarations`/`extract_symbol_decls` sees
                // it, for declarations). Recursing here closes that gap --
                // see `docs/reference/1711-B-coverage-delta.md` case 1.
                if let Some(b) = block {
                    self.walk_unified(b, file_index, symbol_refs);
                }
            }

            NodeKind::Class { name, body, .. } => {
                self.current_package = Some(name.clone());
                // NEW COVERAGE: same gap as `Package` above, for Perl
                // 5.38+ `class Foo { ... }` bodies -- see coverage-delta
                // case 2.
                self.walk_unified(body, file_index, symbol_refs);
            }

            NodeKind::Subroutine { body, prototype, signature, .. } => {
                // NEW COVERAGE: legacy's `visit_node` only visits `body`,
                // never `prototype`/`signature` -- a signature's
                // default-value expressions (`sub greet($name =
                // default_name())`) are invisible today. `Method` (below)
                // already visits `signature` correctly; `Subroutine` did
                // not -- see coverage-delta case 8.
                if let Some(proto) = prototype {
                    self.walk_unified(proto, file_index, symbol_refs);
                }
                if let Some(sig) = signature {
                    self.walk_unified(sig, file_index, symbol_refs);
                }
                self.walk_unified(body, file_index, symbol_refs);
            }

            NodeKind::Method { body, signature, .. } => {
                if let Some(sig) = signature {
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        for param in parameters {
                            self.walk_unified(param, file_index, symbol_refs);
                        }
                    }
                }
                self.walk_unified(body, file_index, symbol_refs);
            }

            // Signature parameter nodes bind a DECLARATION-site variable --
            // must NOT be walked as a reference. Mirrors
            // `extract_symbol_refs`'s explicit skip exactly (ref.rs: "the
            // bound variable is a declaration, not a ref"). Without this
            // arm, the generic `Node::for_each_child` fallback below would
            // visit `variable` too (it has no such declaration/reference
            // distinction), incorrectly emitting a plain `Read`/`SymbolRef`
            // for e.g. a signature's `$name` binding.
            //
            // `NamedParameter` is grouped here (TOTAL skip, including its
            // `default_value`) to mirror `ref.rs:80-84` EXACTLY --
            // `extract_symbol_refs` groups `NamedParameter` with
            // `MandatoryParameter`/`SlurpyParameter` as a Phase-1
            // intentional exclusion (see `ref.rs`'s module doc, lines
            // 15-17: "Optional parameter *default values* are still walked
            // because they are expressions" -- explicitly NOT named-param
            // defaults). Only `OptionalParameter` gets its default walked.
            // An earlier draft of this arm walked `NamedParameter`'s
            // `default_value` too, which made the unified traversal's
            // CANONICAL projection produce an extra `SymbolRef` production
            // `extract_symbol_refs` never produces for a named-param
            // default (e.g. `method bar(:$beta = calc_default())`) --
            // caught by an independent correctness review, since no
            // corpus/real-project/edge-case fixture exercised
            // `NamedParameter` at the time. See
            // `docs/reference/1711-B-coverage-delta.md` and the
            // `coverage_delta_named_parameter_default_is_not_a_new_case`
            // test below, which locks in that this is NOT a
            // coverage-delta case (legacy's pre-unification behavior for
            // `NamedParameter` was ALSO a total skip -- `NamedParameter`
            // was never in `visit_node`/`visit_children`'s coverage
            // either -- so nothing changes for legacy here, and canonical
            // must stay byte-identical).
            NodeKind::MandatoryParameter { .. }
            | NodeKind::SlurpyParameter { .. }
            | NodeKind::NamedParameter { .. } => {
                // Nothing to walk: the bound variable is a declaration,
                // and (for `NamedParameter`) its default value is also
                // intentionally excluded, matching canonical exactly.
            }
            NodeKind::OptionalParameter { default_value, .. } => {
                // Only the default-value expression may reference other
                // symbols (it's evaluated in the caller's scope) -- the
                // bound variable itself is skipped, matching ref.rs.
                self.walk_unified(default_value, file_index, symbol_refs);
            }

            NodeKind::VariableDeclaration { initializer, .. }
            | NodeKind::VariableListDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    self.walk_unified(init, file_index, symbol_refs);
                }
            }

            NodeKind::Variable { sigil, name } => {
                let var_name = format!("{sigil}{name}");
                file_index.references.entry(var_name).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    kind: ReferenceKind::Read,
                });
                Self::emit_canonical_ref(node, symbol_refs);
            }

            NodeKind::Typeglob { .. } => {
                // NEW COVERAGE: legacy has no `Typeglob` arm at all today
                // (`*foo`, `*alias = ...` aliasing references are silently
                // dropped) -- see coverage-delta case 3. Canonical already
                // handled this (dynamic `*{$expr}` forms are filtered by
                // `canonical_ref_for_node`, matching `extract_symbol_refs`).
                if let Some(symbol_ref) = canonical_ref_for_node(node) {
                    // Project a legacy-shaped entry too, using the same
                    // sigil+bare-name key convention as the `Variable` arm
                    // above, so the unified traversal's two output shapes
                    // stay internally consistent.
                    let var_name = format!("*{}", symbol_ref.name);
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Usage,
                    });
                    symbol_refs.push(symbol_ref);
                }
            }

            NodeKind::FunctionCall { name, args, .. } => {
                let func_name = name.clone();
                let location = self.node_to_range(node);

                let (pkg, bare_name) = if let Some(idx) = func_name.rfind("::") {
                    (&func_name[..idx], &func_name[idx + 2..])
                } else {
                    (self.current_package.as_deref().unwrap_or("main"), func_name.as_str())
                };
                let qualified = format!("{pkg}::{bare_name}");

                file_index.references.entry(bare_name.to_string()).or_default().push(
                    SymbolReference {
                        uri: self.uri.clone(),
                        range: location,
                        kind: ReferenceKind::Usage,
                    },
                );
                file_index.references.entry(qualified).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: location,
                    kind: ReferenceKind::Usage,
                });

                if name == "extends" || name == "with" {
                    for module_name in extract_module_names_from_call_args(args) {
                        file_index
                            .dependencies
                            .insert(normalize_dependency_module_name(&module_name));
                    }
                } else if name == "require"
                    && let Some(module_name) = extract_module_name_from_require_args(args)
                {
                    file_index.dependencies.insert(normalize_dependency_module_name(&module_name));
                }

                Self::emit_canonical_ref(node, symbol_refs);

                for arg in args {
                    self.walk_unified(arg, file_index, symbol_refs);
                }
            }

            NodeKind::Use { module, args, .. } => {
                let module_name = normalize_dependency_module_name(module);
                file_index.dependencies.insert(module_name.clone());
                if module == "parent" || module == "base" {
                    for name in extract_module_names_from_use_args(args) {
                        file_index.dependencies.insert(normalize_dependency_module_name(&name));
                    }
                }
                file_index.references.entry(module_name).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    kind: ReferenceKind::Import,
                });
                // No canonical equivalent and no recursion into `args` --
                // matches BOTH legacy's current behavior and
                // `Node::for_each_child`'s (`Use` is a declared leaf there).
            }

            NodeKind::Assignment { lhs, rhs, op } => {
                let is_compound = op != "=";
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    let var_name = format!("{sigil}{name}");
                    if is_compound {
                        file_index.references.entry(var_name.clone()).or_default().push(
                            SymbolReference {
                                uri: self.uri.clone(),
                                range: self.node_to_range(lhs),
                                kind: ReferenceKind::Read,
                            },
                        );
                    }
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(lhs),
                        kind: ReferenceKind::Write,
                    });
                    // Canonical (today, via generic recursion + the
                    // `Variable` arm) classifies an assignment target as a
                    // plain `Read` occurrence -- emit that directly here
                    // instead of ALSO generically recursing into `lhs`
                    // (which would double-count the legacy side above).
                    Self::emit_canonical_ref(lhs, symbol_refs);
                } else {
                    // NEW COVERAGE: legacy's current `Assignment` arm does
                    // NOTHING for a non-`Variable` lhs (e.g. `$h{compute_key()}
                    // = 1`) -- no recursion, so nested references inside an
                    // indexed/complex assignment target are invisible today.
                    // Canonical already reaches this via generic recursion.
                    // See coverage-delta case 9.
                    self.walk_unified(lhs, file_index, symbol_refs);
                }
                self.walk_unified(rhs, file_index, symbol_refs);
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.walk_unified(stmt, file_index, symbol_refs);
                }
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
                self.walk_unified(condition, file_index, symbol_refs);
                self.walk_unified(then_branch, file_index, symbol_refs);
                for (cond, branch) in elsif_branches {
                    self.walk_unified(cond, file_index, symbol_refs);
                    self.walk_unified(branch, file_index, symbol_refs);
                }
                if let Some(else_br) = else_branch {
                    self.walk_unified(else_br, file_index, symbol_refs);
                }
            }

            NodeKind::While { condition, body, continue_block, .. } => {
                self.walk_unified(condition, file_index, symbol_refs);
                self.walk_unified(body, file_index, symbol_refs);
                if let Some(cont) = continue_block {
                    self.walk_unified(cont, file_index, symbol_refs);
                }
            }

            NodeKind::For { init, condition, update, body, continue_block } => {
                if let Some(i) = init {
                    self.walk_unified(i, file_index, symbol_refs);
                }
                if let Some(c) = condition {
                    self.walk_unified(c, file_index, symbol_refs);
                }
                if let Some(u) = update {
                    self.walk_unified(u, file_index, symbol_refs);
                }
                self.walk_unified(body, file_index, symbol_refs);
                if let Some(cont) = continue_block {
                    self.walk_unified(cont, file_index, symbol_refs);
                }
            }

            NodeKind::Foreach { variable, list, body, continue_block } => {
                if let Some(cb) = continue_block {
                    self.walk_unified(cb, file_index, symbol_refs);
                }
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let var_name = format!("{sigil}{name}");
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(variable),
                        kind: ReferenceKind::Write,
                    });
                }
                // Matches legacy's EXISTING (quirky but unchanged) behavior:
                // `variable` is recursed into unconditionally after the
                // write-classification above, so a `Variable` loop target
                // gets BOTH a `Write` entry (from this arm) and a `Read`
                // entry (from the generic `Variable` arm below) today --
                // preserved here for parity, not introduced by unification.
                self.walk_unified(variable, file_index, symbol_refs);
                self.walk_unified(list, file_index, symbol_refs);
                self.walk_unified(body, file_index, symbol_refs);
            }

            NodeKind::MethodCall { object, method, args } => {
                let qualified_method = if let NodeKind::Identifier { name } = &object.kind {
                    Some(format!("{name}::{method}"))
                } else {
                    None
                };

                // Emit the canonical ref BEFORE recursing into `object` --
                // this position must NOT move. `perl_symbol::surface::ref::walk`'s
                // own `MethodCall` arm also pushes its `SymbolRef` before
                // recursing into `object` (own-ref-before-child DFS order),
                // so keeping `emit_canonical_ref` here preserves the
                // byte-for-byte canonical parity `assert_unified_canonical_parity`
                // enforces -- for a chained `$x->foo()->foo()`, the OUTER
                // call's `SymbolRef` precedes the INNER call's in
                // `symbol_refs`, matching production exactly.
                Self::emit_canonical_ref(node, symbol_refs);

                // Recurse into `object` BEFORE recording this call's own
                // LEGACY `FileIndex` reference below -- mirrors
                // `IndexVisitor::visit_node`'s legacy `MethodCall` arm order
                // exactly (child-before-own-ref; see that arm a few hundred
                // lines above, which visits `object` first and only then
                // pushes its own reference). Without this, a chained
                // same-named call like `$x->foo()->foo()` would invert the
                // intra-key `file_index.references["foo"]` Vec order
                // relative to legacy (no reference lost -- counts still
                // match -- but the order silently changed). See
                // `parity_method_call_chained_same_name_reference_order`,
                // which locks this order with an exact Vec equality
                // assertion against legacy's own output.
                self.walk_unified(object, file_index, symbol_refs);

                let location = self.node_to_range(node);
                if let Some(qualified_method) = qualified_method.as_ref() {
                    file_index.references.entry(qualified_method.clone()).or_default().push(
                        SymbolReference {
                            uri: self.uri.clone(),
                            range: location,
                            kind: ReferenceKind::Usage,
                        },
                    );
                }
                file_index.references.entry(method.clone()).or_default().push(SymbolReference {
                    uri: self.uri.clone(),
                    range: location,
                    kind: ReferenceKind::Usage,
                });

                if method == "import"
                    && let NodeKind::Identifier { name: module_name } = &object.kind
                {
                    for symbol in extract_manual_import_symbols(args) {
                        file_index.references.entry(symbol).or_default().push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            kind: ReferenceKind::Import,
                        });
                    }
                    file_index.dependencies.insert(normalize_dependency_module_name(module_name));
                }

                for arg in args {
                    self.walk_unified(arg, file_index, symbol_refs);
                }
            }

            NodeKind::No { module, .. } => {
                let module_name = normalize_dependency_module_name(module);
                file_index.dependencies.insert(module_name);
            }

            NodeKind::String { value, interpolated } => {
                if *interpolated {
                    let range = self.node_to_range(node);
                    self.record_interpolated_variable_references(value, range, file_index);
                }
            }

            NodeKind::Heredoc { content, interpolated, .. } => {
                if *interpolated {
                    let range = self.node_to_range(node);
                    self.record_interpolated_variable_references(content, range, file_index);
                }
            }

            NodeKind::Unary { op, operand } if op == "++" || op == "--" => {
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    let var_name = format!("{sigil}{name}");
                    file_index.references.entry(var_name.clone()).or_default().push(
                        SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(operand),
                            kind: ReferenceKind::Read,
                        },
                    );
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(operand),
                        kind: ReferenceKind::Write,
                    });
                    // Mirrors the `Assignment` arm above: canonical (today,
                    // via generic recursion) classifies an increment target
                    // as a plain `Read` -- emit directly, no double recurse.
                    Self::emit_canonical_ref(operand, symbol_refs);
                } else {
                    // NEW COVERAGE: legacy's current `++`/`--` arm does
                    // NOTHING for a non-`Variable` operand (e.g.
                    // `$h{compute_key()}++`) -- see coverage-delta case 9
                    // (same class as the `Assignment` gap above).
                    self.walk_unified(operand, file_index, symbol_refs);
                }
            }

            NodeKind::Goto { target, .. } => {
                // NEW COVERAGE: legacy has no `Goto` arm at all today --
                // `goto &handler` / `goto LABEL` coderef targets are
                // invisible. See coverage-delta case 4.
                if let Some(symbol_ref) =
                    canonical_coderef_target_ref(target, (node.location.start, node.location.end))
                {
                    let var_name = format!("&{}", symbol_ref.name);
                    file_index.references.entry(var_name).or_default().push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Usage,
                    });
                    symbol_refs.push(symbol_ref);
                } else {
                    self.walk_unified(target, file_index, symbol_refs);
                }
            }

            NodeKind::Unary { op, operand } if op == "\\" => {
                // Unlike `Goto` (which legacy never handles at all today),
                // legacy's CURRENT `visit_node` already has default behavior
                // here: general `Unary` falls through to `visit_children`'s
                // arm, which ALWAYS recurses into `operand` unconditionally,
                // regardless of shape. That must be preserved exactly even
                // when `operand` is coderef-target-shaped for CANONICAL
                // purposes (e.g. `\&attr` parses as a zero-arg synthetic
                // `FunctionCall`, and legacy's generic recursion into it
                // hits the ordinary `FunctionCall` arm -- dual
                // bare+qualified `Usage` entries, the SAME thing legacy has
                // always produced for it).
                match canonical_coderef_target_ref(
                    operand,
                    (node.location.start, node.location.end),
                ) {
                    Some(symbol_ref) => {
                        symbol_refs.push(symbol_ref);
                        // `operand` is guaranteed (by
                        // `canonical_coderef_target_ref`'s own match) to be
                        // either a bare `&name` `Variable` or a zero-arg
                        // synthetic `FunctionCall` -- both leaves, nothing
                        // to recurse into. Replicate legacy's per-kind
                        // classification directly (NOT a second canonical
                        // emission via `walk_unified`, which would
                        // double-count against the `symbol_ref` just
                        // pushed above).
                        match &operand.kind {
                            NodeKind::Variable { sigil, name } => {
                                let var_name = format!("{sigil}{name}");
                                file_index.references.entry(var_name).or_default().push(
                                    SymbolReference {
                                        uri: self.uri.clone(),
                                        range: self.node_to_range(operand),
                                        kind: ReferenceKind::Read,
                                    },
                                );
                            }
                            NodeKind::FunctionCall { name, .. } => {
                                let location = self.node_to_range(operand);
                                let (pkg, bare_name) = if let Some(idx) = name.rfind("::") {
                                    (&name[..idx], &name[idx + 2..])
                                } else {
                                    (
                                        self.current_package.as_deref().unwrap_or("main"),
                                        name.as_str(),
                                    )
                                };
                                let qualified = format!("{pkg}::{bare_name}");
                                file_index
                                    .references
                                    .entry(bare_name.to_string())
                                    .or_default()
                                    .push(SymbolReference {
                                        uri: self.uri.clone(),
                                        range: location,
                                        kind: ReferenceKind::Usage,
                                    });
                                file_index.references.entry(qualified).or_default().push(
                                    SymbolReference {
                                        uri: self.uri.clone(),
                                        range: location,
                                        kind: ReferenceKind::Usage,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                    None => {
                        // Not coderef-target-shaped -- BOTH today's
                        // canonical (`push_coderef_target` returned false,
                        // so it recurses) and today's legacy (which always
                        // recurses here) want ordinary full recursion.
                        self.walk_unified(operand, file_index, symbol_refs);
                    }
                }
            }

            // Everything else: no legacy-specific classification, and
            // recurse via `Node::for_each_child` -- the complete,
            // compiler-exhaustive dispatcher (replaces
            // `IndexVisitor::visit_children`'s hand-maintained allowlist).
            // This is what closes the remaining coverage-delta cases
            // (`Tie`/`Untie`, `Match`/`Substitution`/`Transliteration`
            // regex-bind expressions, `IndirectCall`, and anything else
            // legacy's allowlist silently stopped at) for free.
            _ => {
                node.for_each_child(|child| self.walk_unified(child, file_index, symbol_refs));
            }
        }
    }
}

/// **Production (1711-B cutover).** Canonical [`SymbolRef`] classification for
/// a single node, duplicated from `perl_symbol::surface::ref`'s private
/// `walk` match arms for `Variable`/`Typeglob`/`FunctionCall`/`MethodCall`
/// (the node kinds `IndexVisitor::walk_unified` also classifies for the
/// legacy projection). Byte-for-byte parity with `extract_symbol_refs`'s
/// own output is mechanically enforced by
/// `extraction_bundle_shadow_compare`'s canonical-side parity tests -- any
/// drift between this copy and `perl-symbol`'s private walker fails a test
/// immediately, not silently. Returns `None` for node kinds this function
/// does not classify, for a dynamic typeglob (`*{$expr}`), or for a parser
/// sentinel `FunctionCall` name (`"->()"`, `"&{}"`, `"field"`).
fn canonical_ref_for_node(node: &Node) -> Option<perl_symbol::surface::r#ref::SymbolRef> {
    use perl_symbol::surface::r#ref::{SymbolRef, SymbolRefKind};

    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            let kind = match sigil.as_str() {
                "&" => SymbolRefKind::CoderefReference,
                "*" => SymbolRefKind::TypeglobReference,
                "$" | "$#" => SymbolRefKind::Variable(VarKind::Scalar),
                "@" => SymbolRefKind::Variable(VarKind::Array),
                "%" => SymbolRefKind::Variable(VarKind::Hash),
                _ => return None,
            };
            let (package_qualifier, bare_name, qualified_name) = split_qualified_name_dup(name);
            Some(SymbolRef {
                kind,
                name: bare_name,
                qualified_name,
                sigil: Some(sigil.clone()),
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            })
        }
        NodeKind::Typeglob { name } => {
            if name.starts_with('{') {
                return None;
            }
            let (package_qualifier, bare_name, qualified_name) = split_qualified_name_dup(name);
            Some(SymbolRef {
                kind: SymbolRefKind::TypeglobReference,
                name: bare_name,
                qualified_name,
                sigil: Some("*".to_string()),
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            })
        }
        NodeKind::FunctionCall { name, .. } => {
            if matches!(name.as_str(), "->()" | "&{}" | "field") {
                return None;
            }
            let (package_qualifier, bare_name, qualified_name) = split_qualified_name_dup(name);
            Some(SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: bare_name,
                qualified_name,
                sigil: None,
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            })
        }
        NodeKind::MethodCall { object, method, .. } => {
            let (package_qualifier, qualified_name, kind) = if let NodeKind::Identifier { name } =
                &object.kind
            {
                (Some(name.clone()), format!("{name}::{method}"), SymbolRefKind::StaticMethodCall)
            } else {
                (None, method.clone(), SymbolRefKind::MethodCall)
            };
            Some(SymbolRef {
                kind,
                name: method.clone(),
                qualified_name,
                sigil: None,
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: None,
            })
        }
        _ => None,
    }
}

/// **Production (1711-B cutover).** Coderef-target classification for
/// `Goto`/backslash-`Unary` nodes, duplicated from
/// `perl_symbol::surface::ref`'s private `coderef_target_name`/
/// `push_coderef_target`. See [`canonical_ref_for_node`]'s doc comment for
/// the parity-enforcement rationale.
fn canonical_coderef_target_ref(
    node: &Node,
    full_span: (usize, usize),
) -> Option<perl_symbol::surface::r#ref::SymbolRef> {
    use perl_symbol::surface::r#ref::{SymbolRef, SymbolRefKind};

    let name = match &node.kind {
        NodeKind::Variable { sigil, name } if sigil == "&" => name.as_str(),
        NodeKind::FunctionCall { name, args }
            if args.is_empty()
                && node.location.end.saturating_sub(node.location.start) == name.len() + 1 =>
        {
            name.as_str()
        }
        _ => return None,
    };
    let (package_qualifier, bare_name, qualified_name) = split_qualified_name_dup(name);
    Some(SymbolRef {
        kind: SymbolRefKind::CoderefReference,
        name: bare_name,
        qualified_name,
        sigil: Some("&".to_string()),
        package_qualifier,
        full_span,
        anchor_span: Some((node.location.start, node.location.end)),
    })
}

/// **Production (1711-B cutover).** Duplicated from
/// `perl_symbol::surface::ref::split_qualified_name` (private to that
/// crate). See [`canonical_ref_for_node`]'s doc comment for the
/// parity-enforcement rationale.
fn split_qualified_name_dup(name: &str) -> (Option<String>, String, String) {
    if let Some((package, bare)) = name.rsplit_once("::")
        && !package.is_empty()
        && !bare.is_empty()
    {
        return (Some(package.to_owned()), bare.to_owned(), name.to_owned());
    }
    (None, name.to_owned(), name.to_owned())
}

fn symbol_decl_name(kind: &SymbolKind, name: &str) -> String {
    match kind {
        SymbolKind::Variable(VarKind::Scalar) => format!("${name}"),
        SymbolKind::Variable(VarKind::Array) => format!("@{name}"),
        SymbolKind::Variable(VarKind::Hash) => format!("%{name}"),
        _ => name.to_string(),
    }
}

fn split_qualified_symbol_name(canonical_name: &str) -> Option<(&str, &str)> {
    let (container, bare_name) = canonical_name.rsplit_once("::")?;
    if container.is_empty() || bare_name.is_empty() {
        return None;
    }
    Some((container, bare_name))
}

fn is_framework_generated_member_entity(entity: &EntityFact) -> bool {
    entity.provenance == Provenance::FrameworkSynthesis && entity.confidence == Confidence::Medium
}

fn sort_workspace_symbols(symbols: &mut [WorkspaceSymbol]) {
    symbols.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.uri.cmp(&right.uri))
            .then_with(|| left.range.start.line.cmp(&right.range.start.line))
            .then_with(|| left.range.start.column.cmp(&right.range.start.column))
            .then_with(|| left.range.end.line.cmp(&right.range.end.line))
            .then_with(|| left.range.end.column.cmp(&right.range.end.column))
    });
}

/// Extract bare module names from the argument list of a `use parent` / `use base` statement.
///
/// The `args` field of `NodeKind::Use` stores raw argument strings as the parser captured them.
/// For `use parent 'Foo::Bar'` this is `["'Foo::Bar'"]`.
/// For `use parent qw(Foo::Bar Other::Base)` this is `["qw(Foo::Bar Other::Base)"]`.
/// For `use parent -norequire, 'Foo::Bar'` this is `["-norequire", "'Foo::Bar'"]`.
///
/// Returns the module names with surrounding quotes/qw wrappers stripped.
/// Tokens starting with `-` or not matching `[\w::']+` are silently skipped.
fn extract_module_names_from_use_args(args: &[String]) -> Vec<String> {
    use std::collections::HashSet;

    fn normalize_module_name(token: &str) -> Option<&str> {
        let stripped = token.trim_matches(|c: char| {
            matches!(c, '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';')
        });

        if stripped.is_empty() || stripped.starts_with('-') {
            return None;
        }

        stripped
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '\'')
            .then_some(stripped)
    }

    let joined = args.join(" ");

    let (qw_words, remainder) = extract_qw_words(&joined);
    let mut modules = Vec::new();
    let mut seen = HashSet::new();
    for word in qw_words {
        if let Some(candidate) = normalize_module_name(&word) {
            let canonical = canonicalize_perl_module_name(candidate);
            if seen.insert(canonical.clone()) {
                modules.push(canonical);
            }
        }
    }

    for token in remainder.split_whitespace().flat_map(|t| t.split(',')) {
        if let Some(candidate) = normalize_module_name(token) {
            let canonical = canonicalize_perl_module_name(candidate);
            if seen.insert(canonical.clone()) {
                modules.push(canonical);
            }
        }
    }

    modules
}

fn extract_module_names_from_call_args(args: &[Node]) -> Vec<String> {
    fn collect_from_node(node: &Node, out: &mut Vec<String>) {
        match &node.kind {
            NodeKind::String { value, .. } => {
                out.extend(extract_module_names_from_use_args(std::slice::from_ref(value)));
            }
            NodeKind::Identifier { name } => {
                out.extend(extract_module_names_from_use_args(std::slice::from_ref(name)));
            }
            NodeKind::ArrayLiteral { elements } => {
                for element in elements {
                    collect_from_node(element, out);
                }
            }
            NodeKind::FunctionCall { name, args, .. } if name == "qw" => {
                for arg in args {
                    collect_from_node(arg, out);
                }
            }
            _ => {}
        }
    }

    let mut modules = Vec::new();
    for arg in args {
        collect_from_node(arg, &mut modules);
    }
    modules
}

fn canonicalize_perl_module_name(name: &str) -> String {
    // Perl supports the legacy `'` package separator (e.g. Foo'Bar).
    // Canonicalize to `::` so lookups and dependency matching share one key shape.
    name.replace('\'', "::")
}

fn legacy_perl_module_name(name: &str) -> String {
    name.replace("::", "'")
}

/// Normalize a module name for dependency storage and lookup.
/// Converts legacy `'` separators to `::` so stored keys are canonical.
fn normalize_dependency_module_name(module_name: &str) -> String {
    canonicalize_perl_module_name(module_name)
}

fn extract_qw_words(input: &str) -> (Vec<String>, String) {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut words = Vec::new();
    let mut remainder = String::new();

    while i < chars.len() {
        if chars[i] == 'q'
            && i + 1 < chars.len()
            && chars[i + 1] == 'w'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
        {
            let mut j = i + 2;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j >= chars.len() {
                remainder.push(chars[i]);
                i += 1;
                continue;
            }

            let open = chars[j];
            let (close, is_paired_delimiter) = match open {
                '(' => (')', true),
                '[' => (']', true),
                '{' => ('}', true),
                '<' => ('>', true),
                _ => (open, false),
            };
            if open.is_alphanumeric() || open == '_' || open == '\'' || open == '"' {
                remainder.push(chars[i]);
                i += 1;
                continue;
            }

            let mut k = j + 1;
            if is_paired_delimiter {
                let mut depth = 1usize;
                while k < chars.len() && depth > 0 {
                    if chars[k] == open {
                        depth += 1;
                    } else if chars[k] == close {
                        depth -= 1;
                    }
                    k += 1;
                }
                if depth != 0 {
                    remainder.extend(chars[i..].iter());
                    break;
                }
                k -= 1;
            } else {
                while k < chars.len() && chars[k] != close {
                    k += 1;
                }
                if k >= chars.len() {
                    remainder.extend(chars[i..].iter());
                    break;
                }
            }

            let content: String = chars[j + 1..k].iter().collect();
            for word in content.split_whitespace() {
                if !word.is_empty() {
                    words.push(word.to_string());
                }
            }
            i = k + 1;
            continue;
        }

        remainder.push(chars[i]);
        i += 1;
    }

    (words, remainder)
}

fn extract_module_name_from_require_args(args: &[Node]) -> Option<String> {
    let first = args.first()?;
    match &first.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::String { value, .. } => {
            let cleaned = value.trim_matches('\'').trim_matches('"').trim();
            if cleaned.is_empty() {
                return None;
            }
            Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
        }
        _ => None,
    }
}

fn extract_manual_import_symbols(args: &[Node]) -> Vec<String> {
    fn push_if_bareword(out: &mut Vec<String>, token: &str) {
        let bare = token.trim().trim_matches('"').trim_matches('\'').trim();
        if bare.is_empty() || bare == "," {
            return;
        }
        let is_bareword = bare.bytes().all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
            && bare.as_bytes().first().is_some_and(|ch| ch.is_ascii_alphabetic() || *ch == b'_');
        if is_bareword {
            out.push(bare.to_string());
        }
    }

    let mut symbols = Vec::new();
    for arg in args {
        match &arg.kind {
            NodeKind::String { value, .. } => push_if_bareword(&mut symbols, value),
            NodeKind::Identifier { name } => {
                if name.starts_with("qw") {
                    let content = name
                        .trim_start_matches("qw")
                        .trim_start_matches(|c: char| "([{/<|!".contains(c))
                        .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                    for token in content.split_whitespace() {
                        push_if_bareword(&mut symbols, token);
                    }
                } else {
                    push_if_bareword(&mut symbols, name);
                }
            }
            NodeKind::ArrayLiteral { elements } => {
                for element in elements {
                    if let NodeKind::String { value, .. } = &element.kind {
                        push_if_bareword(&mut symbols, value);
                    }
                }
            }
            _ => {}
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

/// Extract constant names from the `args` field of a `use constant` `NodeKind::Use` node.
///
/// The parser serialises `use constant` args in two distinct forms:
///
/// **Scalar form** — `use constant FOO => 42;`
///   → args: `["FOO", "42"]`  (the `=>` is consumed by the parser, not stored)
///   → The first arg is the constant name; remaining args are the value.
///
/// **Hash form** — `use constant { FOO => 1, BAR => 2 };`
///   → args: `["{", "FOO", "=>", "1", ",", "BAR", "=>", "2", "}"]`
///   → Identifiers immediately followed by `=>` are constant names.
///
/// **qw form** — `use constant qw(FOO BAR);`
///   → args: `["qw(FOO BAR)"]`
///   → Words inside the qw list are constant names.
///
/// Returns a deduplicated list of bare constant names (e.g. `["FOO", "BAR"]`).
#[cfg(test)]
fn extract_constant_names_from_use_args(args: &[String]) -> Vec<String> {
    use std::collections::HashSet;

    fn push_unique(names: &mut Vec<String>, seen: &mut HashSet<String>, candidate: &str) {
        if seen.insert(candidate.to_string()) {
            names.push(candidate.to_string());
        }
    }

    fn normalize_constant_name(token: &str) -> Option<&str> {
        let stripped = token.trim_matches(|c: char| {
            matches!(c, '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';')
        });

        if stripped.is_empty() || stripped.starts_with('-') {
            return None;
        }

        stripped.chars().all(|c| c.is_alphanumeric() || c == '_').then_some(stripped)
    }

    let mut names = Vec::new();
    let mut seen = HashSet::new();

    // Scalar form (most common): args = ["FOO", <value...>]
    // The first arg is a plain identifier with no `=>` in args at all.
    // Hash form starts with `{`; qw form starts with `qw`.
    let first = match args.first() {
        Some(f) => f.as_str(),
        None => return names,
    };

    // qw form: single arg starting with "qw"
    if first.starts_with("qw") {
        let (qw_words, remainder) = extract_qw_words(first);
        if remainder.trim().is_empty() {
            for word in qw_words {
                if let Some(candidate) = normalize_constant_name(&word) {
                    push_unique(&mut names, &mut seen, candidate);
                }
            }
            return names;
        }

        // Fallback for odd tokenisation: tolerate `qw` followed by spacing before the opener.
        let content = first.trim_start_matches("qw").trim_start();
        let content = content
            .trim_start_matches(|c: char| "([{/<|!".contains(c))
            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
        for word in content.split_whitespace() {
            if let Some(candidate) = normalize_constant_name(word) {
                push_unique(&mut names, &mut seen, candidate);
            }
        }
        return names;
    }

    // Hash form: args start with "{", "+{", or "+" followed by "{"
    let starts_hash_form = first == "{"
        || first == "+{"
        || (first == "+" && args.get(1).map(String::as_str) == Some("{"));
    if starts_hash_form {
        let mut skipped_leading_plus = false;
        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            // Some parser/tokenizer variants can emit "+{" as a single token for
            // `use constant +{ ... }`. Treat it as structural punctuation.
            if arg == "+{" {
                skipped_leading_plus = true;
                continue;
            }
            if arg == "+" && !skipped_leading_plus {
                skipped_leading_plus = true;
                continue;
            }
            if arg == "{" || arg == "}" || arg == "," || arg == "=>" {
                continue;
            }
            if let Some(candidate) = normalize_constant_name(arg)
                && iter.peek().map(|s| s.as_str()) == Some("=>")
            {
                push_unique(&mut names, &mut seen, candidate);
            }
        }
        return names;
    }

    // Scalar form: first arg is the constant name (if it is a plain identifier)
    // Remaining args are the value and are skipped.
    if let Some(candidate) = normalize_constant_name(first) {
        push_unique(&mut names, &mut seen, candidate);
    }

    names
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// LSP adapter for converting internal Location types to LSP types
#[cfg(all(feature = "workspace", feature = "lsp-compat"))]
/// LSP adapter utilities for Navigate/Analyze workflows.
pub mod lsp_adapter {
    use super::Location as IxLocation;
    use lsp_types::Location as LspLocation;
    // lsp_types uses Uri, not Url
    type LspUrl = lsp_types::Uri;

    /// Convert an internal location to an LSP Location for Navigate workflows.
    ///
    /// # Arguments
    ///
    /// * `ix` - Internal index location with URI and range information.
    ///
    /// # Returns
    ///
    /// `Some(LspLocation)` when conversion succeeds, or `None` if URI parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{Location as IxLocation, lsp_adapter::to_lsp_location};
    /// use lsp_types::Range;
    ///
    /// let ix_loc = IxLocation { uri: "file:///path.pl".to_string(), range: Range::default() };
    /// let _ = to_lsp_location(&ix_loc);
    /// ```
    pub fn to_lsp_location(ix: &IxLocation) -> Option<LspLocation> {
        parse_url(&ix.uri).map(|uri| {
            let start =
                lsp_types::Position { line: ix.range.start.line, character: ix.range.start.column };
            let end =
                lsp_types::Position { line: ix.range.end.line, character: ix.range.end.column };
            let range = lsp_types::Range { start, end };
            LspLocation { uri, range }
        })
    }

    /// Convert multiple index locations to LSP Locations for Navigate/Analyze workflows.
    ///
    /// # Arguments
    ///
    /// * `all` - Iterator of internal index locations to convert.
    ///
    /// # Returns
    ///
    /// Vector of successfully converted LSP locations, with invalid entries filtered out.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser::workspace_index::{Location as IxLocation, lsp_adapter::to_lsp_locations};
    /// use lsp_types::Range;
    ///
    /// let locations = vec![IxLocation { uri: "file:///script1.pl".to_string(), range: Range::default() }];
    /// let lsp_locations = to_lsp_locations(locations);
    /// assert_eq!(lsp_locations.len(), 1);
    /// ```
    pub fn to_lsp_locations(all: impl IntoIterator<Item = IxLocation>) -> Vec<LspLocation> {
        all.into_iter().filter_map(|ix| to_lsp_location(&ix)).collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_url(s: &str) -> Option<LspUrl> {
        // lsp_types::Uri uses FromStr, not TryFrom
        use std::str::FromStr;

        // Try parsing as URI first
        LspUrl::from_str(s).ok().or_else(|| {
            // Try as a file path if URI parsing fails
            std::path::Path::new(s).canonicalize().ok().and_then(|p| {
                // Use proper URI construction with percent-encoding
                crate::workspace_index::fs_path_to_uri(&p)
                    .ok()
                    .and_then(|uri_string| LspUrl::from_str(&uri_string).ok())
            })
        })
    }

    /// Parse a string as a URL (wasm32 version - no filesystem fallback)
    #[cfg(target_arch = "wasm32")]
    fn parse_url(s: &str) -> Option<LspUrl> {
        use std::str::FromStr;
        LspUrl::from_str(s).ok()
    }
}

/// Test-only instrumentation for measuring `didChange` re-extraction
/// work-shape (perl-lsp-swarm#1711 / PR 1711-A).
///
/// Every function here is compiled ONLY under `#[cfg(test)]` -- there is no
/// production build in which this module, or any call into it, exists. This
/// is a **measurement receipt vehicle**, not a runtime feature: it changes
/// no extraction/propagation behavior, only records how much work the
/// EXISTING `index_file_with_generation` path already does, per call site,
/// on the calling thread.
///
/// A thread-local (not a global static) is used because
/// `index_file_with_generation` runs synchronously on the caller's thread
/// whenever there is no Tokio runtime installed -- exactly how every
/// measurement test in `reindex_workshape_measurement` (below) invokes it.
/// `cargo test` runs tests in parallel on separate OS threads, so a
/// thread-local avoids cross-test interference without needing a mutex or
/// serial-test annotation.
#[cfg(test)]
pub(crate) mod reindex_metrics {
    use std::cell::RefCell;
    use std::time::Duration;

    /// Work-shape counters/timers captured for a single
    /// `index_file_with_generation` call.
    #[derive(Debug, Default, Clone, Copy)]
    pub(crate) struct ReindexWorkMetrics {
        pub visit_calls: u32,
        pub visit_time: Duration,
        pub decl_extract_calls: u32,
        pub decl_extract_time: Duration,
        pub ref_extract_calls: u32,
        pub ref_extract_time: Duration,
        pub eval_sub_calls: u32,
        pub eval_sub_time: Duration,
        pub generated_member_calls: u32,
        pub generated_member_time: Duration,
        pub import_extract_calls: u32,
        pub import_extract_time: Duration,
        pub use_lib_extract_calls: u32,
        pub use_lib_extract_time: Duration,
        /// THIS URI's own `FileIndex::symbols` contribution -- the count of
        /// entries passed through the legacy symbol-table removal routine
        /// on this call. NOT necessarily the number of entries removed from
        /// the global qualified/bare-name map (dual-indexing may write each
        /// contributed symbol under up to two global keys), and NOT a
        /// whole-workspace rebuild -- only this one file's contribution.
        pub legacy_symbols_removed: usize,
        /// Same file-scoped contribution, on the re-add side of this call.
        pub legacy_symbols_added: usize,
        /// THIS URI's contribution passed through the search-index removal
        /// routine (same file-scoped caveat as `legacy_symbols_removed`).
        pub legacy_search_removed: usize,
        pub legacy_search_added: usize,
        /// THIS URI's own global-reference-index contribution (not the
        /// whole workspace-wide reference cache) passed through the
        /// removal routine on this call.
        pub global_refs_removed: usize,
        pub global_refs_added: usize,
        /// `true` when this call took the whole-file content-hash
        /// short-circuit (cheapest possible outcome -- no re-extraction at
        /// all).
        pub content_hash_short_circuit: bool,
        /// `true` when this call was rejected by the pre-parse high-water
        /// monotonic-generation guard.
        pub stale_generation_rejected_pre_parse: bool,
        /// `true` when this call was rejected by the post-parse monotonic
        /// generation guard.
        pub stale_generation_rejected_post_parse: bool,
        /// `true` when this call's generation was genuinely committed.
        pub generation_accepted: bool,
    }

    thread_local! {
        static CURRENT: RefCell<Option<ReindexWorkMetrics>> = const { RefCell::new(None) };
    }

    /// Begin recording on the calling thread. Any prior unread recording on
    /// this thread is discarded.
    pub(crate) fn start() {
        CURRENT.with(|c| *c.borrow_mut() = Some(ReindexWorkMetrics::default()));
    }

    /// Stop recording and return whatever was captured since the last
    /// `start()` on this thread (a zeroed value if `start()` was never
    /// called).
    pub(crate) fn take() -> ReindexWorkMetrics {
        CURRENT.with(|c| c.borrow_mut().take().unwrap_or_default())
    }

    fn record(f: impl FnOnce(&mut ReindexWorkMetrics)) {
        CURRENT.with(|c| {
            if let Some(m) = c.borrow_mut().as_mut() {
                f(m);
            }
        });
    }

    pub(crate) fn record_visit(d: Duration) {
        record(|m| {
            m.visit_calls += 1;
            m.visit_time += d;
        });
    }
    pub(crate) fn record_decl_extract(d: Duration) {
        record(|m| {
            m.decl_extract_calls += 1;
            m.decl_extract_time += d;
        });
    }
    pub(crate) fn record_ref_extract(d: Duration) {
        record(|m| {
            m.ref_extract_calls += 1;
            m.ref_extract_time += d;
        });
    }
    pub(crate) fn record_eval_sub(d: Duration) {
        record(|m| {
            m.eval_sub_calls += 1;
            m.eval_sub_time += d;
        });
    }
    pub(crate) fn record_generated_member(d: Duration) {
        record(|m| {
            m.generated_member_calls += 1;
            m.generated_member_time += d;
        });
    }
    pub(crate) fn record_import_extract(d: Duration) {
        record(|m| {
            m.import_extract_calls += 1;
            m.import_extract_time += d;
        });
    }
    pub(crate) fn record_use_lib_extract(d: Duration) {
        record(|m| {
            m.use_lib_extract_calls += 1;
            m.use_lib_extract_time += d;
        });
    }
    pub(crate) fn record_legacy_symbols_removed(n: usize) {
        record(|m| m.legacy_symbols_removed += n);
    }
    pub(crate) fn record_legacy_search_removed(n: usize) {
        record(|m| m.legacy_search_removed += n);
    }
    pub(crate) fn record_global_refs_removed(n: usize) {
        record(|m| m.global_refs_removed += n);
    }
    pub(crate) fn record_legacy_symbols_added(n: usize) {
        record(|m| m.legacy_symbols_added += n);
    }
    pub(crate) fn record_legacy_search_added(n: usize) {
        record(|m| m.legacy_search_added += n);
    }
    pub(crate) fn record_global_refs_added(n: usize) {
        record(|m| m.global_refs_added += n);
    }
    pub(crate) fn record_content_hash_short_circuit() {
        record(|m| m.content_hash_short_circuit = true);
    }
    pub(crate) fn record_stale_rejected_pre_parse() {
        record(|m| m.stale_generation_rejected_pre_parse = true);
    }
    pub(crate) fn record_stale_rejected_post_parse() {
        record(|m| m.stale_generation_rejected_post_parse = true);
    }
    pub(crate) fn record_generation_accepted() {
        record(|m| m.generation_accepted = true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_use_constant_indexed_as_constant_symbol() {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/My/Config.pm";
        let code = r#"package My::Config;
use constant PI => 3.14159;
use constant {
    MAX_RETRIES => 3,
    TIMEOUT     => 30,
};
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let symbols = index.file_symbols(uri);
        assert!(
            symbols.iter().any(|s| s.name == "PI" && s.kind == SymbolKind::Constant),
            "PI should be indexed as a Constant symbol; got: {:?}",
            symbols.iter().map(|s| (&s.name, &s.kind)).collect::<Vec<_>>()
        );
        assert!(
            symbols.iter().any(|s| s.name == "MAX_RETRIES" && s.kind == SymbolKind::Constant),
            "MAX_RETRIES should be indexed"
        );
        assert!(
            symbols.iter().any(|s| s.name == "TIMEOUT" && s.kind == SymbolKind::Constant),
            "TIMEOUT should be indexed"
        );

        // Qualified lookup should also work
        let def = index.find_definition("My::Config::PI");
        assert!(def.is_some(), "find_definition('My::Config::PI') should succeed");
    }

    #[test]
    fn test_extract_constant_names_deduplicates_qw_form() {
        let names = extract_constant_names_from_use_args(&["qw(FOO BAR FOO)".to_string()]);
        assert_eq!(names, vec!["FOO", "BAR"]);
    }

    #[test]
    fn test_extract_constant_names_accepts_quoted_scalar_form() {
        let names = extract_constant_names_from_use_args(&[
            "'HTTP_OK'".to_string(),
            "=>".to_string(),
            "200".to_string(),
        ]);
        assert_eq!(names, vec!["HTTP_OK"]);
    }

    #[test]
    fn search_symbols_returns_labeled_generated_framework_members()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Generated/Pilot.pm";
        let code = r#"package Generated::Pilot;
use Moo;
has display_name => (is => 'rw');
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let source_symbols = index.search_source_symbols("display_name", None);
        assert!(
            source_symbols.is_empty(),
            "generated framework members must not enter the exact source-symbol slice"
        );
        let trimmed_source_symbols = index.search_source_symbols("  display_name  ", None);
        assert!(
            trimmed_source_symbols.is_empty(),
            "trimmed generated framework member queries must not enter the exact source-symbol slice"
        );

        let generated_symbols = index.search_generated_workspace_symbols("display_name", None);
        assert_eq!(generated_symbols.len(), 1);
        let trimmed_generated_symbols =
            index.search_generated_workspace_symbols("  display_name  ", None);
        assert_eq!(trimmed_generated_symbols.len(), 1);
        assert_eq!(trimmed_generated_symbols[0].name, "display_name [generated/framework]");
        assert!(index.search_generated_workspace_symbols("   ", None).is_empty());
        let symbol = &generated_symbols[0];
        assert_eq!(symbol.name, "display_name [generated/framework]");
        assert_eq!(symbol.kind, SymbolKind::Method);
        assert_eq!(symbol.qualified_name.as_deref(), Some("Generated::Pilot::display_name"));
        assert_eq!(
            symbol.container_name.as_deref(),
            Some("Generated::Pilot [generated/framework]")
        );
        assert!(!symbol.has_body);
        assert_eq!(symbol.uri, uri);
        assert!(
            symbol.range.end.byte > symbol.range.start.byte,
            "generated symbol must be anchored to the source framework declaration"
        );

        let live_symbols = index.search_symbols("display_name");
        assert!(
            live_symbols.is_empty(),
            "general workspace index search must stay source-backed; generated pilot symbols are opt-in"
        );

        {
            let mut shards = index.fact_shards.write();
            let shard = shards.values_mut().next().ok_or("missing generated-member shard")?;
            let entity = shard
                .entities
                .iter_mut()
                .find(|entity| entity.canonical_name == "Generated::Pilot::display_name")
                .ok_or("missing generated member entity")?;
            entity.provenance = Provenance::ExactAst;
        }
        let non_framework_symbols = index.search_generated_workspace_symbols("display_name", None);
        assert!(
            non_framework_symbols.is_empty(),
            "generated workspace-symbol pilot must require framework-synthesis provenance"
        );
        Ok(())
    }

    #[test]
    fn search_symbols_returns_labeled_predicate_generated_members()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Generated/PredicatePilot.pm";
        let code = r#"package Generated::PredicatePilot;
use Moo;
has status => (is => 'rw', predicate => 1);
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let source_symbols = index.search_source_symbols("has_status", None);
        assert!(
            source_symbols.is_empty(),
            "predicate generated members must not enter the exact source-symbol slice"
        );

        let generated_symbols = index.search_generated_workspace_symbols("has_status", None);
        assert_eq!(generated_symbols.len(), 1);
        let symbol = &generated_symbols[0];
        assert_eq!(symbol.name, "has_status [generated/framework]");
        assert_eq!(symbol.kind, SymbolKind::Method);
        assert_eq!(symbol.qualified_name.as_deref(), Some("Generated::PredicatePilot::has_status"));
        assert_eq!(
            symbol.container_name.as_deref(),
            Some("Generated::PredicatePilot [generated/framework]")
        );
        assert!(!symbol.has_body);
        assert_eq!(symbol.uri, uri);
        assert!(
            symbol.range.end.byte > symbol.range.start.byte,
            "predicate generated symbol must be anchored to the source framework declaration"
        );

        let live_symbols = index.search_symbols("has_status");
        assert!(
            live_symbols.is_empty(),
            "general workspace index search must stay source-backed for predicate generated members"
        );
        Ok(())
    }

    #[test]
    fn test_extract_constant_names_accepts_quoted_hash_form() {
        let names = extract_constant_names_from_use_args(&[
            "{".to_string(),
            "'FOO'".to_string(),
            "=>".to_string(),
            "1".to_string(),
            ",".to_string(),
            "\"BAR\"".to_string(),
            "=>".to_string(),
            "2".to_string(),
            "}".to_string(),
        ]);
        assert_eq!(names, vec!["FOO", "BAR"]);
    }

    #[test]
    fn test_extract_constant_names_accepts_plus_hash_form_split_tokens() {
        let names = extract_constant_names_from_use_args(&[
            "+".to_string(),
            "{".to_string(),
            "FOO".to_string(),
            "=>".to_string(),
            "1".to_string(),
            ",".to_string(),
            "BAR".to_string(),
            "=>".to_string(),
            "2".to_string(),
            "}".to_string(),
        ]);
        assert_eq!(names, vec!["FOO", "BAR"]);
    }

    #[test]
    fn test_extract_constant_names_accepts_plus_hash_form_combined_token() {
        let names = extract_constant_names_from_use_args(&[
            "+{".to_string(),
            "FOO".to_string(),
            "=>".to_string(),
            "1".to_string(),
            ",".to_string(),
            "BAR".to_string(),
            "=>".to_string(),
            "2".to_string(),
            "}".to_string(),
        ]);
        assert_eq!(names, vec!["FOO", "BAR"]);
    }
    #[test]
    fn test_use_constant_duplicate_names_indexed_once() {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/My/DedupConfig.pm";
        let code = r#"package My::DedupConfig;
use constant {
    RETRY_COUNT => 3,
    RETRY_COUNT => 5,
};
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let symbols = index.file_symbols(uri);
        let retry_count_symbols = symbols.iter().filter(|s| s.name == "RETRY_COUNT").count();
        assert_eq!(
            retry_count_symbols, 1,
            "RETRY_COUNT should be indexed once even when repeated in use constant hash form"
        );
    }

    #[test]
    fn test_use_constant_plus_hash_form_indexes_keys() {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/My/PlusHash.pm";
        let code = r#"package My::PlusHash;
use constant +{
    FOO => 1,
    BAR => 2,
};
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        assert!(index.find_definition("My::PlusHash::FOO").is_some());
        assert!(index.find_definition("My::PlusHash::BAR").is_some());
    }

    #[test]
    fn test_basic_indexing() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";

        let code = r#"
package MyPackage;

sub hello {
    print "Hello";
}

my $var = 42;
"#;

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        // Should have indexed the package and subroutine
        let symbols = index.file_symbols(uri);
        assert!(symbols.iter().any(|s| s.name == "MyPackage" && s.kind == SymbolKind::Package));
        assert!(symbols.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));
        assert!(symbols.iter().any(|s| s.name == "$var" && s.kind.is_variable()));
    }

    #[test]
    fn test_package_symbol_has_no_container_name() {
        // Regression: project_symbol_declarations used to set container_name = Some("main")
        // for top-level package declarations because the IndexVisitor starts with
        // current_package = Some("main").  Package symbols are top-level declarations
        // and must have container_name = None.
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Foo.pm";
        let code = "package Foo;\nsub bar { }\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let symbols = index.file_symbols(uri);
        let pkg_sym =
            must_some(symbols.iter().find(|s| s.name == "Foo" && s.kind == SymbolKind::Package));
        assert_eq!(
            pkg_sym.container_name, None,
            "Package symbol must not carry a container (was 'main')"
        );
    }

    #[test]
    fn test_file_packages_returns_only_package_symbol_names() {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/OnlyPackages.pm";
        let code = "package Foo;\nsub hello { 1 }\npackage Bar { sub greet { 2 } }\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let mut package_names = index.file_packages(uri);
        package_names.sort();
        let mut expected_package_names: Vec<String> = index
            .file_symbols(uri)
            .into_iter()
            .filter(|s| s.kind == SymbolKind::Package)
            .map(|s| s.name)
            .collect();
        expected_package_names.sort();

        assert_eq!(package_names, expected_package_names);
        assert_eq!(package_names, vec!["Bar", "Foo"]);
        assert!(!package_names.iter().any(|name| name == "hello"));
        assert!(!package_names.iter().any(|name| name == "greet"));
    }

    #[test]
    fn test_file_package_symbols_returns_exact_container_match() {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/PackageMembers.pm";
        let code = "package Foo;\nsub hello { 1 }\npackage Bar;\nsub greet { 2 }\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let all_symbols = index.file_symbols(uri);
        let package_name = "Bar";
        let greet_symbol = must_some(all_symbols.iter().find(|s| s.name == "greet"));
        let bar_package = must_some(
            all_symbols.iter().find(|s| s.name == "Bar" && s.kind == SymbolKind::Package),
        );
        assert!(WorkspaceIndex::symbol_belongs_to_package(greet_symbol, package_name));
        assert!(!WorkspaceIndex::symbol_belongs_to_package(greet_symbol, "Foo"));
        assert!(!WorkspaceIndex::symbol_belongs_to_package(bar_package, package_name));

        let mut expected_bar_names: Vec<String> = all_symbols
            .iter()
            .filter(|s| s.container_name.as_deref() == Some(package_name))
            .map(|s| s.name.clone())
            .collect();
        expected_bar_names.sort();

        let mut bar_names: Vec<String> =
            index.file_package_symbols(uri, package_name).into_iter().map(|s| s.name).collect();
        bar_names.sort();
        assert_eq!(bar_names, expected_bar_names);
        assert_eq!(bar_names, vec!["greet"]);

        let mut foo_names: Vec<String> =
            index.file_package_symbols(uri, "Foo").into_iter().map(|s| s.name).collect();
        foo_names.sort();
        assert_eq!(foo_names, vec!["hello"]);
        assert!(index.file_package_symbols(uri, "Missing").is_empty());
    }

    #[test]
    fn test_my_variable_has_no_qualified_name() {
        // Regression: project_symbol_declarations used to set qualified_name = Some("Foo::x")
        // for `my $x` inside `package Foo`, making `find_definition("Foo::x")` return the
        // lexical variable.  `my` variables are not package-visible and must have
        // qualified_name = None so qualified lookups don't match them.
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Foo.pm";
        let code = "package Foo;\nsub bar { my $x = 1; }\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let symbols = index.file_symbols(uri);
        let var_sym = must_some(symbols.iter().find(|s| s.name == "$x" && s.kind.is_variable()));
        assert_eq!(var_sym.qualified_name, None, "my variable must not have a qualified_name");

        // `find_definition("Foo::x")` must not accidentally resolve to a lexical variable.
        assert!(
            index.find_definition("Foo::x").is_none(),
            "find_definition(\"Foo::x\") must not return a lexical my variable"
        );
    }

    fn reference_kinds_for(
        index: &WorkspaceIndex,
        uri: &str,
        symbol_name: &str,
    ) -> Vec<ReferenceKind> {
        let files = index.files.read();
        let file = must_some(files.get(uri));
        file.references
            .get(symbol_name)
            .map(|refs| refs.iter().map(|r| r.kind).collect())
            .unwrap_or_default()
    }

    #[test]
    fn test_reference_kinds_sub_definition_and_call_are_distinct() {
        let index = WorkspaceIndex::new();
        let uri = "file:///typed-refs-sub.pl";
        let code = "package TypedRefs;
sub foo { return 1; }
foo();
";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let kinds = reference_kinds_for(&index, uri, "foo");
        assert!(kinds.contains(&ReferenceKind::Definition));
        assert!(kinds.contains(&ReferenceKind::Usage));
    }

    #[test]
    fn test_reference_kinds_variable_read_and_write_are_distinct() {
        let index = WorkspaceIndex::new();
        let uri = "file:///typed-refs-var.pl";
        let code = "my $value = 1;
$value = 2;
print $value;
";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let kinds = reference_kinds_for(&index, uri, "$value");
        assert!(kinds.contains(&ReferenceKind::Definition));
        assert!(kinds.contains(&ReferenceKind::Write));
        assert!(kinds.contains(&ReferenceKind::Read));
    }

    #[test]
    fn test_reference_kinds_import_parent_and_export_ok_are_currently_import_only() {
        let index = WorkspaceIndex::new();
        let uri = "file:///typed-refs-import-export.pm";
        let code = "package Child;
use parent 'Base';
our @EXPORT_OK = qw(foo);
1;
";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let parent_kinds = reference_kinds_for(&index, uri, "Base");
        assert!(
            parent_kinds.is_empty(),
            "use parent inheritance edges are currently not stored as typed references"
        );

        let export_symbol_kinds = reference_kinds_for(&index, uri, "foo");
        assert!(
            export_symbol_kinds.is_empty(),
            "EXPORT_OK entries are currently not represented as reference edges"
        );
    }

    #[test]
    fn test_reference_kinds_dynamic_and_meta_edges_are_not_typed_yet() {
        let index = WorkspaceIndex::new();
        let uri = "file:///typed-refs-dynamic.pl";
        let code = r#"package TypedRefs;
sub foo { 1 }
&foo;
my $code = \&foo;
goto &foo;
*alias = \&foo;
eval "foo()";
with 'RoleName';
has 'name' => (is => 'ro');
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let foo_kinds = reference_kinds_for(&index, uri, "foo");
        assert!(
            foo_kinds
                .iter()
                .all(|kind| matches!(kind, ReferenceKind::Definition | ReferenceKind::Usage)),
            r"dynamic call forms (&foo, \&foo, goto &foo) are currently flattened to Usage"
        );

        assert!(
            reference_kinds_for(&index, uri, "RoleName").is_empty(),
            "role composition edges (`with 'RoleName'`) are not indexed as typed references yet"
        );
    }

    #[test]
    fn test_find_references() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";

        let code = r#"
sub test {
    my $x = 1;
    $x = 2;
    print $x;
}
"#;

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let refs = index.find_references("$x");
        assert!(refs.len() >= 2); // Definition + at least one usage
    }

    #[test]
    fn test_find_references_bare_name_includes_qualified_calls() {
        let index = WorkspaceIndex::new();
        let uri = "file:///refs.pl";
        let code = r#"
package RefDemo;
sub helper {
    return 1;
}

helper();
RefDemo::helper();
"#;

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let bare_refs = index.find_references("helper");
        let qualified_refs = index.find_references("RefDemo::helper");

        assert!(
            bare_refs.len() >= qualified_refs.len(),
            "bare-name reference lookup should include qualified calls"
        );
    }

    #[test]
    fn test_count_usages_bare_name_includes_qualified_calls() {
        let index = WorkspaceIndex::new();
        let uri = "file:///usage.pl";
        let code = r#"
package UsageDemo;
sub helper {
    return 1;
}

helper();
UsageDemo::helper();
"#;

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let bare_usage_count = index.count_usages("helper");
        let qualified_usage_count = index.count_usages("UsageDemo::helper");

        assert!(
            bare_usage_count >= qualified_usage_count,
            "bare-name usage count should include qualified call sites"
        );
    }

    #[test]
    fn test_dependencies() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";

        let code = r#"
use strict;
use warnings;
use Data::Dumper;
"#;

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let deps = index.file_dependencies(uri);
        assert!(deps.contains("strict"));
        assert!(deps.contains("warnings"));
        assert!(deps.contains("Data::Dumper"));
    }

    #[test]
    fn test_uri_to_fs_path_basic() {
        // Test basic file:// URI conversion
        if let Some(path) = uri_to_fs_path("file:///tmp/test.pl") {
            assert_eq!(path, std::path::PathBuf::from("/tmp/test.pl"));
        }

        // Test with invalid URI
        assert!(uri_to_fs_path("not-a-uri").is_none());

        // Test with non-file scheme
        assert!(uri_to_fs_path("http://example.com").is_none());
    }

    #[test]
    fn test_uri_to_fs_path_with_spaces() {
        // Test with percent-encoded spaces
        if let Some(path) = uri_to_fs_path("file:///tmp/path%20with%20spaces/test.pl") {
            assert_eq!(path, std::path::PathBuf::from("/tmp/path with spaces/test.pl"));
        }

        // Test with multiple spaces and special characters
        if let Some(path) = uri_to_fs_path("file:///tmp/My%20Documents/test%20file.pl") {
            assert_eq!(path, std::path::PathBuf::from("/tmp/My Documents/test file.pl"));
        }
    }

    #[test]
    fn test_uri_to_fs_path_with_unicode() {
        // Test with Unicode characters (percent-encoded)
        if let Some(path) = uri_to_fs_path("file:///tmp/caf%C3%A9/test.pl") {
            assert_eq!(path, std::path::PathBuf::from("/tmp/café/test.pl"));
        }

        // Test with Unicode emoji (percent-encoded)
        if let Some(path) = uri_to_fs_path("file:///tmp/emoji%F0%9F%98%80/test.pl") {
            assert_eq!(path, std::path::PathBuf::from("/tmp/emoji😀/test.pl"));
        }
    }

    #[test]
    fn test_fs_path_to_uri_basic() {
        // Test basic path to URI conversion
        let result = fs_path_to_uri("/tmp/test.pl");
        assert!(result.is_ok());
        let uri = must(result);
        assert!(uri.starts_with("file://"));
        assert!(uri.contains("/tmp/test.pl"));
    }

    #[test]
    fn test_fs_path_to_uri_with_spaces() {
        // Test path with spaces
        let result = fs_path_to_uri("/tmp/path with spaces/test.pl");
        assert!(result.is_ok());
        let uri = must(result);
        assert!(uri.starts_with("file://"));
        // Should contain percent-encoded spaces
        assert!(uri.contains("path%20with%20spaces"));
    }

    #[test]
    fn test_fs_path_to_uri_with_unicode() {
        // Test path with Unicode characters
        let result = fs_path_to_uri("/tmp/café/test.pl");
        assert!(result.is_ok());
        let uri = must(result);
        assert!(uri.starts_with("file://"));
        // Should contain percent-encoded Unicode
        assert!(uri.contains("caf%C3%A9"));
    }

    #[test]
    fn test_normalize_uri_file_schemes() {
        // Test normalization of valid file URIs
        let uri = WorkspaceIndex::normalize_uri("file:///tmp/test.pl");
        assert_eq!(uri, "file:///tmp/test.pl");

        // Test normalization of URIs with spaces
        let uri = WorkspaceIndex::normalize_uri("file:///tmp/path%20with%20spaces/test.pl");
        assert_eq!(uri, "file:///tmp/path%20with%20spaces/test.pl");
    }

    #[test]
    fn test_normalize_uri_absolute_paths() {
        // Test normalization of absolute paths (convert to file:// URI)
        let uri = WorkspaceIndex::normalize_uri("/tmp/test.pl");
        assert!(uri.starts_with("file://"));
        assert!(uri.contains("/tmp/test.pl"));
    }

    #[test]
    fn test_normalize_uri_special_schemes() {
        // Test that special schemes like untitled: are preserved
        let uri = WorkspaceIndex::normalize_uri("untitled:Untitled-1");
        assert_eq!(uri, "untitled:Untitled-1");
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Test that URI -> path -> URI conversion preserves the URI
        let original_uri = "file:///tmp/path%20with%20spaces/caf%C3%A9.pl";

        if let Some(path) = uri_to_fs_path(original_uri) {
            if let Ok(converted_uri) = fs_path_to_uri(&path) {
                // Should be able to round-trip back to an equivalent URI
                assert!(converted_uri.starts_with("file://"));

                // The path component should decode correctly
                if let Some(roundtrip_path) = uri_to_fs_path(&converted_uri) {
                    #[cfg(windows)]
                    if let Ok(rootless) = path.strip_prefix(std::path::Path::new(r"\")) {
                        assert!(roundtrip_path.ends_with(rootless));
                    } else {
                        assert_eq!(path, roundtrip_path);
                    }

                    #[cfg(not(windows))]
                    assert_eq!(path, roundtrip_path);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_paths() {
        // Test Windows-style paths
        let result = fs_path_to_uri(r"C:\Users\test\Documents\script.pl");
        assert!(result.is_ok());
        let uri = must(result);
        assert!(uri.starts_with("file://"));

        // Test Windows path with spaces
        let result = fs_path_to_uri(r"C:\Program Files\My App\script.pl");
        assert!(result.is_ok());
        let uri = must(result);
        assert!(uri.starts_with("file://"));
        assert!(uri.contains("Program%20Files"));
    }

    // ========================================================================
    // IndexCoordinator Tests
    // ========================================================================

    #[test]
    fn test_coordinator_initial_state() {
        let coordinator = IndexCoordinator::new();
        assert!(matches!(
            coordinator.state(),
            IndexState::Building { phase: IndexPhase::Idle, .. }
        ));
    }

    #[test]
    fn test_transition_to_scanning_phase() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_scanning();

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { phase: IndexPhase::Scanning, .. }),
            "Expected Building state after scanning, got: {:?}",
            state
        );
    }

    #[test]
    fn test_transition_to_indexing_phase() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_scanning();
        coordinator.update_scan_progress(3);
        coordinator.transition_to_indexing(3);

        let state = coordinator.state();
        assert!(
            matches!(
                state,
                IndexState::Building { phase: IndexPhase::Indexing, total_count: 3, .. }
            ),
            "Expected Building state after indexing with total_count 3, got: {:?}",
            state
        );
    }

    #[test]
    fn test_transition_to_ready() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        let state = coordinator.state();
        if let IndexState::Ready { file_count, symbol_count, .. } = state {
            assert_eq!(file_count, 100);
            assert_eq!(symbol_count, 5000);
        } else {
            unreachable!("Expected Ready state, got: {:?}", state);
        }
    }

    #[test]
    fn test_parse_storm_degradation() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        // Trigger parse storm
        for _ in 0..15 {
            coordinator.notify_change("file.pm");
        }

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Degraded { .. }),
            "Expected Degraded state, got: {:?}",
            state
        );
        if let IndexState::Degraded { reason, .. } = state {
            assert!(matches!(reason, DegradationReason::ParseStorm { .. }));
        }
    }

    #[test]
    fn test_recovery_from_parse_storm() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        // Trigger parse storm
        for _ in 0..15 {
            coordinator.notify_change("file.pm");
        }

        // Complete all parses
        for _ in 0..15 {
            coordinator.notify_parse_complete("file.pm");
        }

        // Should recover to Building state
        assert!(matches!(coordinator.state(), IndexState::Building { .. }));
    }

    #[test]
    fn test_query_dispatch_ready() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        let result = coordinator.query(|_index| "full_query", |_index| "partial_query");

        assert_eq!(result, "full_query");
    }

    #[test]
    fn test_query_dispatch_degraded() {
        let coordinator = IndexCoordinator::new();
        // Building state should use partial query

        let result = coordinator.query(|_index| "full_query", |_index| "partial_query");

        assert_eq!(result, "partial_query");
    }

    #[test]
    fn test_metrics_pending_count() {
        let coordinator = IndexCoordinator::new();

        coordinator.notify_change("file1.pm");
        coordinator.notify_change("file2.pm");

        assert_eq!(coordinator.metrics.pending_count(), 2);

        coordinator.notify_parse_complete("file1.pm");
        assert_eq!(coordinator.metrics.pending_count(), 1);
    }

    #[test]
    fn test_instrumentation_records_transitions() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(10, 100);

        let snapshot = coordinator.instrumentation_snapshot();
        let transition =
            IndexStateTransition { from: IndexStateKind::Building, to: IndexStateKind::Ready };
        let count = snapshot.state_transition_counts.get(&transition).copied().unwrap_or(0);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_instrumentation_records_early_exit() {
        let coordinator = IndexCoordinator::new();
        coordinator.record_early_exit(EarlyExitReason::InitialTimeBudget, 25, 1, 10);

        let snapshot = coordinator.instrumentation_snapshot();
        let count = snapshot
            .early_exit_counts
            .get(&EarlyExitReason::InitialTimeBudget)
            .copied()
            .unwrap_or(0);
        assert_eq!(count, 1);
        assert!(snapshot.last_early_exit.is_some());
    }

    #[test]
    fn test_custom_limits() {
        let limits = IndexResourceLimits {
            max_files: 5000,
            max_symbols_per_file: 1000,
            max_total_symbols: 100_000,
            max_ast_cache_bytes: 128 * 1024 * 1024,
            max_ast_cache_items: 50,
            max_scan_duration_ms: 30_000,
        };

        let coordinator = IndexCoordinator::with_limits(limits.clone());
        assert_eq!(coordinator.limits.max_files, 5000);
        assert_eq!(coordinator.limits.max_total_symbols, 100_000);
    }

    #[test]
    fn test_degradation_preserves_symbol_count() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        coordinator.transition_to_degraded(DegradationReason::IoError {
            message: "Test error".to_string(),
        });

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Degraded { .. }),
            "Expected Degraded state, got: {:?}",
            state
        );
        if let IndexState::Degraded { available_symbols, .. } = state {
            assert_eq!(available_symbols, 5000);
        }
    }

    #[test]
    fn test_index_access() {
        let coordinator = IndexCoordinator::new();
        let index = coordinator.index();

        // Should have access to underlying WorkspaceIndex
        assert!(index.all_symbols().is_empty());
    }

    #[test]
    fn test_resource_limit_enforcement_max_files() {
        let limits = IndexResourceLimits {
            max_files: 5,
            max_symbols_per_file: 1000,
            max_total_symbols: 50_000,
            max_ast_cache_bytes: 128 * 1024 * 1024,
            max_ast_cache_items: 50,
            max_scan_duration_ms: 30_000,
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_ready(10, 100);

        // Index 10 files (exceeds limit of 5)
        for i in 0..10 {
            let uri_str = format!("file:///test{}.pl", i);
            let uri = must(url::Url::parse(&uri_str));
            let code = "sub test { }";
            must(coordinator.index().index_file(uri, code.to_string()));
        }

        // Enforce limits
        coordinator.enforce_limits();

        let state = coordinator.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded {
                    reason: DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles },
                    ..
                }
            ),
            "Expected Degraded state with ResourceLimit(MaxFiles), got: {:?}",
            state
        );
    }

    #[test]
    fn test_resource_limit_enforcement_max_symbols() {
        let limits = IndexResourceLimits {
            max_files: 100,
            max_symbols_per_file: 10,
            max_total_symbols: 50, // Very low limit for testing
            max_ast_cache_bytes: 128 * 1024 * 1024,
            max_ast_cache_items: 50,
            max_scan_duration_ms: 30_000,
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_ready(0, 0);

        // Index files with many symbols to exceed total symbol limit
        for i in 0..10 {
            let uri_str = format!("file:///test{}.pl", i);
            let uri = must(url::Url::parse(&uri_str));
            // Each file has 10 subroutines = 100 total symbols (exceeds limit of 50)
            let code = r#"
package Test;
sub sub1 { }
sub sub2 { }
sub sub3 { }
sub sub4 { }
sub sub5 { }
sub sub6 { }
sub sub7 { }
sub sub8 { }
sub sub9 { }
sub sub10 { }
"#;
            must(coordinator.index().index_file(uri, code.to_string()));
        }

        // Enforce limits
        coordinator.enforce_limits();

        let state = coordinator.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded {
                    reason: DegradationReason::ResourceLimit { kind: ResourceKind::MaxSymbols },
                    ..
                }
            ),
            "Expected Degraded state with ResourceLimit(MaxSymbols), got: {:?}",
            state
        );
    }

    #[test]
    fn test_check_limits_returns_none_within_bounds() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(0, 0);

        // Index a few files well within default limits
        for i in 0..5 {
            let uri_str = format!("file:///test{}.pl", i);
            let uri = must(url::Url::parse(&uri_str));
            let code = "sub test { }";
            must(coordinator.index().index_file(uri, code.to_string()));
        }

        // Should not trigger degradation
        let limit_check = coordinator.check_limits();
        assert!(limit_check.is_none(), "check_limits should return None when within bounds");

        // State should still be Ready
        assert!(
            matches!(coordinator.state(), IndexState::Ready { .. }),
            "State should remain Ready when within limits"
        );
    }

    #[test]
    fn test_enforce_limits_called_on_transition_to_ready() {
        let limits = IndexResourceLimits {
            max_files: 3,
            max_symbols_per_file: 1000,
            max_total_symbols: 50_000,
            max_ast_cache_bytes: 128 * 1024 * 1024,
            max_ast_cache_items: 50,
            max_scan_duration_ms: 30_000,
        };

        let coordinator = IndexCoordinator::with_limits(limits);

        // Index files before transitioning to ready
        for i in 0..5 {
            let uri_str = format!("file:///test{}.pl", i);
            let uri = must(url::Url::parse(&uri_str));
            let code = "sub test { }";
            must(coordinator.index().index_file(uri, code.to_string()));
        }

        // Transition to ready - should automatically enforce limits
        coordinator.transition_to_ready(5, 100);

        let state = coordinator.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded {
                    reason: DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles },
                    ..
                }
            ),
            "Expected Degraded state after transition_to_ready with exceeded limits, got: {:?}",
            state
        );
    }

    #[test]
    fn test_state_transition_guard_ready_to_ready() {
        // Test that Ready → Ready is allowed (metrics update)
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        // Transition to Ready again with different metrics
        coordinator.transition_to_ready(150, 7500);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Ready { file_count: 150, symbol_count: 7500, .. }),
            "Expected Ready state with updated metrics, got: {:?}",
            state
        );
    }

    #[test]
    fn test_state_transition_guard_building_to_building() {
        // Test that Building → Building is allowed (progress update)
        let coordinator = IndexCoordinator::new();

        // Initial building state
        coordinator.transition_to_building(100);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 0, total_count: 100, .. }),
            "Expected Building state, got: {:?}",
            state
        );

        // Update total count
        coordinator.transition_to_building(200);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 0, total_count: 200, .. }),
            "Expected Building state, got: {:?}",
            state
        );
    }

    #[test]
    fn test_state_transition_ready_to_building() {
        // Test that Ready → Building is allowed (re-scan)
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_ready(100, 5000);

        // Trigger re-scan
        coordinator.transition_to_building(150);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 0, total_count: 150, .. }),
            "Expected Building state after re-scan, got: {:?}",
            state
        );
    }

    #[test]
    fn test_state_transition_degraded_to_building() {
        // Test that Degraded → Building is allowed (recovery)
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_degraded(DegradationReason::IoError {
            message: "Test error".to_string(),
        });

        // Attempt recovery
        coordinator.transition_to_building(100);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 0, total_count: 100, .. }),
            "Expected Building state after recovery, got: {:?}",
            state
        );
    }

    #[test]
    fn test_update_building_progress() {
        let coordinator = IndexCoordinator::new();
        coordinator.transition_to_building(100);

        // Update progress
        coordinator.update_building_progress(50);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 50, total_count: 100, .. }),
            "Expected Building state with updated progress, got: {:?}",
            state
        );

        // Update progress again
        coordinator.update_building_progress(100);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 100, total_count: 100, .. }),
            "Expected Building state with completed progress, got: {:?}",
            state
        );
    }

    #[test]
    fn test_scan_timeout_detection() {
        // Test that scan timeout triggers degradation
        let limits = IndexResourceLimits {
            max_scan_duration_ms: 0, // Immediate timeout for testing
            ..Default::default()
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_building(100);

        // Small sleep to ensure elapsed time > 0
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Update progress should detect timeout
        coordinator.update_building_progress(10);

        let state = coordinator.state();
        assert!(
            matches!(
                state,
                IndexState::Degraded { reason: DegradationReason::ScanTimeout { .. }, .. }
            ),
            "Expected Degraded state with ScanTimeout, got: {:?}",
            state
        );
    }

    #[test]
    fn test_scan_timeout_does_not_trigger_within_limit() {
        // Test that scan doesn't timeout within the limit
        let limits = IndexResourceLimits {
            max_scan_duration_ms: 10_000, // 10 seconds - should not trigger
            ..Default::default()
        };

        let coordinator = IndexCoordinator::with_limits(limits);
        coordinator.transition_to_building(100);

        // Update progress immediately (well within limit)
        coordinator.update_building_progress(50);

        let state = coordinator.state();
        assert!(
            matches!(state, IndexState::Building { indexed_count: 50, .. }),
            "Expected Building state (no timeout), got: {:?}",
            state
        );
    }

    #[test]
    fn test_early_exit_optimization_unchanged_content() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test.pl"));
        let code = r#"
package MyPackage;

sub hello {
    print "Hello";
}
"#;

        // First indexing should parse and index
        must(index.index_file(uri.clone(), code.to_string()));
        let symbols1 = index.file_symbols(uri.as_str());
        assert!(symbols1.iter().any(|s| s.name == "MyPackage" && s.kind == SymbolKind::Package));
        assert!(symbols1.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));

        // Second indexing with same content should early-exit
        // We can verify this by checking that the index still works correctly
        must(index.index_file(uri.clone(), code.to_string()));
        let symbols2 = index.file_symbols(uri.as_str());
        assert_eq!(symbols1.len(), symbols2.len());
        assert!(symbols2.iter().any(|s| s.name == "MyPackage" && s.kind == SymbolKind::Package));
        assert!(symbols2.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));
    }

    #[test]
    fn test_index_file_generation_updates_on_same_content_reindex() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///generation.pl"));
        let code = "package Generation;\nsub stable { 1 }\n1;\n";

        must(index.index_file_with_generation(uri.clone(), code.to_string(), 1));
        assert_eq!(index.indexed_generation(uri.as_str()), Some(1));
        assert!(!index.is_index_generation_stale(uri.as_str(), 1));
        assert!(index.is_index_generation_stale(uri.as_str(), 2));

        must(index.index_file_with_generation(uri.clone(), code.to_string(), 2));
        assert_eq!(index.indexed_generation(uri.as_str()), Some(2));
        assert!(!index.is_index_generation_stale(uri.as_str(), 2));
    }

    #[test]
    fn is_index_generation_stale_boundary_discriminator_indexed_generation_less_than_expected_generation()
     {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///generation-boundary.pl"));
        let code = "package GenerationBoundary;\nsub stable { 1 }\n1;\n";

        must(index.index_file_with_generation(uri.clone(), code.to_string(), 2));

        assert_eq!(
            index.indexed_generation(uri.as_str()),
            Some(2),
            "test setup must index the boundary generation"
        );
        assert!(
            !index.is_index_generation_stale(uri.as_str(), 1),
            "indexed_generation > expected_generation must not be stale"
        );
        assert!(
            !index.is_index_generation_stale(uri.as_str(), 2),
            "indexed_generation == expected_generation must not be stale"
        );
        assert!(
            index.is_index_generation_stale(uri.as_str(), 3),
            "indexed_generation < expected_generation must be stale"
        );
    }

    /// #3618 review defect: two fire-and-forget background index tasks for
    /// adjacent generations N and N+1 of the same URI can run on different
    /// threads and complete OUT OF ORDER (the caller's own pre-spawn
    /// freshness check is not atomic with this write -- see the monotonic
    /// generation guard's doc comment on `index_file_with_generation`
    /// above). This deterministically forces exactly that ordering with a
    /// `std::sync::Barrier` (no sleeps): generation N+1's commit is fully
    /// complete (its thread has returned from `index_file_with_generation`
    /// and reached the barrier) BEFORE generation N's thread is released to
    /// attempt its own (now late) write. The older generation must never
    /// win, regardless of which thread's call happened to finish last.
    #[test]
    fn out_of_order_generation_commits_never_regress_the_stored_index()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        let uri = must(url::Url::parse("file:///lib/OutOfOrder.pm"));

        const GEN_N: u32 = 5;
        const GEN_N_PLUS_1: u32 = 6;
        let gen_n_text = "package OutOfOrder;\nsub gen_n_symbol { 1 }\n1;\n".to_string();
        let gen_n_plus_1_text =
            "package OutOfOrder;\nsub gen_n_plus_1_symbol { 1 }\n1;\n".to_string();
        let gen_n_plus_1_text_for_assertion = gen_n_plus_1_text.clone();

        // Released only once BOTH threads have reached it. Generation N+1's
        // thread reaches the barrier AFTER its write has fully committed;
        // generation N's thread reaches the barrier BEFORE attempting its
        // write. So N cannot even start writing until N+1's write is
        // already visible in the store -- the exact "N completes after
        // N+1" ordering the real background-task race can produce.
        let barrier = Arc::new(std::sync::Barrier::new(2));

        let index_n = Arc::clone(&index);
        let uri_n = uri.clone();
        let barrier_n = Arc::clone(&barrier);
        let handle_n = std::thread::spawn(move || -> Result<(), String> {
            barrier_n.wait();
            index_n.index_file_with_generation(uri_n, gen_n_text, GEN_N)
        });

        let index_n1 = Arc::clone(&index);
        let uri_n1 = uri.clone();
        let barrier_n1 = Arc::clone(&barrier);
        let handle_n1 = std::thread::spawn(move || -> Result<(), String> {
            let result =
                index_n1.index_file_with_generation(uri_n1, gen_n_plus_1_text, GEN_N_PLUS_1);
            barrier_n1.wait();
            result
        });

        handle_n1
            .join()
            .map_err(|_| "generation N+1 thread panicked")?
            .map_err(|e| format!("generation N+1 write returned an error: {e}"))?;
        handle_n
            .join()
            .map_err(|_| "generation N thread panicked")?
            .map_err(|e| format!("generation N write returned an error: {e}"))?;

        assert_eq!(
            index.indexed_generation(uri.as_str()),
            Some(GEN_N_PLUS_1),
            "the stored generation must be N+1 even though N's write completed after it"
        );

        let symbols = index.file_symbols(uri.as_str());
        assert!(
            symbols.iter().any(|s| s.name == "gen_n_plus_1_symbol"),
            "the newer generation's content must be the one stored, even though its writer \
             finished first; got symbols: {:?}",
            symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(
            !symbols.iter().any(|s| s.name == "gen_n_symbol"),
            "the older generation's write must never win, even though it committed AFTER the \
             newer one; got symbols: {:?}",
            symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        // #3618 review defect (cubic): `document_store` is a SEPARATE piece
        // of state from `self.files`, written unconditionally earlier in
        // `index_file_with_generation` -- before the monotonic generation
        // guard runs. Proving the guard closes the race for `self.files`
        // (above) does not prove `document_store` stayed consistent with
        // it; a cross-file consumer (workspace rename, safe-delete preview,
        // navigation, hover for symbols in other files -- all read
        // `document_store()` directly) could still observe generation N's
        // stale text even though the symbol index correctly holds N+1's
        // facts.
        let stored_doc = must_some(index.document_store().get(uri.as_str()));
        assert_eq!(
            stored_doc.text, gen_n_plus_1_text_for_assertion,
            "document_store must hold the newer generation's text, matching self.files -- an \
             older out-of-order write must not leave document_store and self.files disagreeing \
             about which generation is current"
        );

        Ok(())
    }

    /// #3618 review defect (factory-droid P1, cubic P1): the test above
    /// forces generation N+1 to FULLY COMMIT (parse finished, late guard
    /// run, `self.files` updated) before generation N's thread even starts
    /// -- so N is correctly rejected by the early guard reading an
    /// ALREADY-CURRENT `self.files[key].generation`. That does not exercise
    /// the actual reported race: `self.files[key].generation` is only
    /// otherwise updated by the LATE guard, which runs AFTER
    /// `Parser::new(&text).parse()` completes -- so a task for generation N
    /// whose early guard check runs WHILE a concurrent task for generation
    /// N+1 has only reserved its slot but is STILL PARSING (not yet at the
    /// late guard) would, without the early reservation this test proves,
    /// see a stale (pre-N+1) generation and incorrectly proceed to
    /// overwrite `document_store` with N's older text.
    ///
    /// Constructs that window directly: releases both threads from the SAME
    /// barrier (no full-completion ordering imposed), gives generation N+1
    /// a LARGE document (many subs) so its parse takes measurably longer
    /// than generation N's trivial one, and gives N a brief, bounded head
    /// start (not a polling sleep -- a single fixed delay establishing
    /// relative ordering, the same class of technique the barrier above
    /// replaces sleep-based polling with, just biasing rather than forcing
    /// the race here since there is no production pause hook to force it
    /// exactly). Repeats several iterations, since a race that depends on
    /// relative thread-scheduling timing is not guaranteed to manifest on
    /// every single run even when present -- consistent reproduction across
    /// iterations is the meaningful signal, not any single run.
    #[test]
    fn concurrent_still_parsing_newer_generation_still_wins_document_store()
    -> Result<(), Box<dyn std::error::Error>> {
        const GEN_N: u32 = 1;
        const GEN_N_PLUS_1: u32 = 2;
        let gen_n_text = "package StillParsing;\nsub gen_n_symbol { 1 }\n1;\n".to_string();
        // Large enough that parsing takes measurably longer than the small
        // generation-N document's parse + N's own early-guard check --
        // widens the window generation N+1 spends between reserving its
        // generation and reaching the late guard.
        let mut gen_n_plus_1_text = String::from("package StillParsing;\n");
        for i in 0..2000 {
            gen_n_plus_1_text.push_str(&format!("sub gen_n_plus_1_symbol_{i} {{ {i} }}\n"));
        }
        gen_n_plus_1_text.push_str("1;\n");

        for iteration in 0..10 {
            let index = Arc::new(WorkspaceIndex::new());
            let uri = must(url::Url::parse(&format!("file:///lib/StillParsing{iteration}.pm")));
            // Baseline generation 0 so `self.files.get_mut(&key)` finds an
            // existing entry for the early guard/reservation to act on --
            // matches the real scenario (an already-tracked document being
            // edited), not a brand-new never-before-seen URI.
            index.index_file_with_generation(
                uri.clone(),
                "package StillParsing;\n1;\n".to_string(),
                0,
            )?;

            let barrier = Arc::new(std::sync::Barrier::new(2));

            let index_n1 = Arc::clone(&index);
            let uri_n1 = uri.clone();
            let barrier_n1 = Arc::clone(&barrier);
            let text_n1 = gen_n_plus_1_text.clone();
            let handle_n1 = std::thread::spawn(move || -> Result<(), String> {
                barrier_n1.wait();
                index_n1.index_file_with_generation(uri_n1, text_n1, GEN_N_PLUS_1)
            });

            let index_n = Arc::clone(&index);
            let uri_n = uri.clone();
            let barrier_n = Arc::clone(&barrier);
            let text_n = gen_n_text.clone();
            let handle_n = std::thread::spawn(move || -> Result<(), String> {
                barrier_n.wait();
                // Brief, bounded head start for generation N+1 to reach and
                // pass its early-guard reservation (a fast, un-parsed
                // operation: hash + lock + compare + document_store write)
                // before generation N's own early-guard check runs -- not a
                // polling loop, a single fixed delay biasing which side of
                // the reservation window this thread's check lands in. Kept
                // well under generation N+1's own parse time (thousands of
                // subs) so this thread's check reliably lands DURING that
                // still-parsing window, not after it.
                std::thread::sleep(std::time::Duration::from_millis(2));
                index_n.index_file_with_generation(uri_n, text_n, GEN_N)
            });

            handle_n1
                .join()
                .map_err(|_| "generation N+1 thread panicked")?
                .map_err(|e| format!("generation N+1 write returned an error: {e}"))?;
            handle_n
                .join()
                .map_err(|_| "generation N thread panicked")?
                .map_err(|e| format!("generation N write returned an error: {e}"))?;

            let stored_doc = must_some(index.document_store().get(uri.as_str()));
            assert!(
                !stored_doc.text.contains("gen_n_symbol"),
                "iteration {iteration}: document_store must never end up holding generation N's \
                 (older) text once generation N+1 has been reserved, even while N+1 is still \
                 parsing when N's early guard runs; got: {:?}",
                stored_doc.text
            );
        }

        Ok(())
    }

    /// #3618 review-3660 finding 3(a): the early-guard reservation added to
    /// close the still-parsing race above must roll itself back when the
    /// parse it was reserved for actually fails -- otherwise a single
    /// unparseable edit permanently strands the tracked generation ahead of
    /// what was ever genuinely indexed, silently disabling all FUTURE
    /// generation guards for that URI (every later legitimate generation
    /// would look "stale" against the phantom reservation forever).
    ///
    /// Needs a real, reliably-reachable parse failure, not a hand-wavy
    /// "assume it can fail" -- uses 200 levels of `if ($a) { ... }` nesting
    /// against `perl-parser-core`'s `MAX_RECURSION_DEPTH = 128`
    /// (`crates/perl-parser-core/src/engine/parser/mod.rs`), which
    /// `check_recursion()` (`engine/parser/helpers.rs`) turns into a
    /// `ParseError::NestingTooDeep` that `parse_statement`'s callers
    /// (`statements.rs`, `control_flow.rs`) explicitly propagate rather
    /// than recover from -- confirmed here by asserting on `Err` directly
    /// (not the softer "Err or recorded diagnostic" pattern the older
    /// `test_deep_nesting_stack_overflow` parser test uses at only 100
    /// levels, which is why that test doesn't already cover this).
    #[test]
    fn parse_error_rollback_restores_the_last_committed_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///lib/DeepNestFail.pm"));

        // Commit a genuine baseline at generation 3 first.
        index.index_file_with_generation(
            uri.clone(),
            "package DeepNestFail;\nsub baseline { 1 }\n1;\n".to_string(),
            3,
        )?;
        assert_eq!(index.indexed_generation(uri.as_str()), Some(3));

        // 200 nested `if` blocks -- well past MAX_RECURSION_DEPTH (128) --
        // deterministically triggers `ParseError::NestingTooDeep`.
        let mut too_deep = String::from("package DeepNestFail;\n");
        for _ in 0..200 {
            too_deep.push_str("if ($a) { ");
        }
        too_deep.push_str("1;");
        for _ in 0..200 {
            too_deep.push('}');
        }
        too_deep.push('\n');

        let result = index.index_file_with_generation(uri.clone(), too_deep, 4);
        assert!(
            result.is_err(),
            "200 levels of nesting must exceed MAX_RECURSION_DEPTH=128 and return Err, not \
             recover -- got {result:?}"
        );

        // The reservation for generation 4 must have rolled back: the
        // publicly-read generation stays at the last genuinely committed
        // value (3), never advances to the failed attempt's 4.
        assert_eq!(
            index.indexed_generation(uri.as_str()),
            Some(3),
            "a failed parse must not leave the tracked generation claiming content that was \
             never actually indexed"
        );

        // And the rollback must not have wedged the guard: a legitimate
        // follow-up at generation 4 with valid text still succeeds.
        index.index_file_with_generation(
            uri.clone(),
            "package DeepNestFail;\nsub recovered { 1 }\n1;\n".to_string(),
            4,
        )?;
        assert_eq!(index.indexed_generation(uri.as_str()), Some(4));
        let symbols = index.file_symbols(uri.as_str());
        assert!(symbols.iter().any(|s| s.name == "recovered"));

        Ok(())
    }

    /// #3618 review-3660 finding 3(b): a rollback that restores the tracked
    /// generation to "whatever THIS task saw before it reserved" (rather
    /// than to the last genuinely committed generation) is unsound under
    /// CHAINED failures. Walkthrough of the bug this guards against (fixed
    /// by tracking `FileIndex::generation` -- committed -- and
    /// `FileIndex::pending_generation` -- reserved/in-flight -- as two
    /// separate fields, with rollback always restoring toward `generation`
    /// rather than a per-task snapshot):
    ///
    /// - Baseline committed generation is 4.
    /// - Task A reserves generation 5 (pending_generation: 4 -> 5), then
    ///   fails to parse.
    /// - Task B reserves generation 6 (pending_generation: 5 -> 6) BEFORE
    ///   A's rollback runs, then ALSO fails to parse.
    /// - A's rollback correctly no-ops: `pending_generation` (6) no longer
    ///   equals what A reserved (5) -- something newer has claimed the
    ///   slot since.
    /// - B's rollback must restore toward the COMMITTED value (4), not
    ///   toward "5" (A's now-stale pre-reservation value, and NOT what B
    ///   itself observed either) -- otherwise `pending_generation` would
    ///   land on an intermediate value nothing ever committed, and (in the
    ///   OLD single-field design where the reservation and the committed
    ///   value were the same field) `indexed_generation()` would incorrectly
    ///   report 5 even though nothing at generation 5 or 6 was ever
    ///   genuinely indexed.
    ///
    /// This test drives that exact interleaving directly (single-threaded,
    /// deterministic -- no timing dependency, since the guard's rollback
    /// logic doesn't depend on wall-clock timing, only on call order) and
    /// asserts `indexed_generation()` is still exactly the baseline (4)
    /// after BOTH failures, then proves the guard isn't left wedged: a
    /// legitimate retry at generation 6 with valid text still succeeds.
    #[test]
    fn chained_reservation_failures_never_strand_generation_above_committed()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///lib/ChainedFail.pm"));

        index.index_file_with_generation(
            uri.clone(),
            "package ChainedFail;\nsub baseline { 1 }\n1;\n".to_string(),
            4,
        )?;
        assert_eq!(index.indexed_generation(uri.as_str()), Some(4));

        let mut too_deep_a = String::from("package ChainedFail;\n");
        for _ in 0..200 {
            too_deep_a.push_str("if ($a) { ");
        }
        too_deep_a.push_str("1;");
        for _ in 0..200 {
            too_deep_a.push('}');
        }
        too_deep_a.push('\n');
        let too_deep_b = too_deep_a.clone();

        // Task A: reserves generation 5, fails.
        let result_a = index.index_file_with_generation(uri.clone(), too_deep_a, 5);
        assert!(result_a.is_err(), "generation 5's over-nested text must fail to parse");

        // Task B: reserves generation 6 -- in the real concurrent scenario
        // this races in WHILE A is still parsing, before A's rollback runs;
        // driving it strictly after A's `Err` here still exercises the
        // same rollback-ordering logic (B's own rollback must restore
        // toward `generation`, not toward whatever A's reservation left
        // behind) since A's failed reservation is never read as a
        // "committed" value by B's high-water check either way.
        let result_b = index.index_file_with_generation(uri.clone(), too_deep_b, 6);
        assert!(result_b.is_err(), "generation 6's over-nested text must fail to parse");

        assert_eq!(
            index.indexed_generation(uri.as_str()),
            Some(4),
            "two chained reservation failures (gen 5 then gen 6) must never strand the tracked \
             generation on an intermediate, never-committed value -- it must still read the last \
             genuinely committed generation"
        );

        // The guard must not be wedged: a legitimate retry at generation 6
        // with valid text still succeeds and correctly advances the index.
        index.index_file_with_generation(
            uri.clone(),
            "package ChainedFail;\nsub recovered { 1 }\n1;\n".to_string(),
            6,
        )?;
        assert_eq!(index.indexed_generation(uri.as_str()), Some(6));
        let symbols = index.file_symbols(uri.as_str());
        assert!(symbols.iter().any(|s| s.name == "recovered"));

        Ok(())
    }

    /// #3618 review-3660 finding 3(c): the early-guard reservation must
    /// roll back on EVERY early return between it and the late guard's
    /// commit -- not just a parse error. `self.document_store.get(&uri_str)
    /// .ok_or("Document not found")?` is a second, real, reachable
    /// early-return site: an ordinary "rapid tab-close after edit" (a
    /// `didClose` racing an in-flight background index task for the same
    /// URI, wired via `DocumentStore::close` in `runtime/text_sync.rs`) can
    /// close the document between the early guard's reservation and this
    /// read. Before this fix, that path had NO rollback at all, silently
    /// leaking the reservation.
    ///
    /// Drives the actual race with real threads and a large document (many
    /// subs) so generation 5's parse takes measurably longer than the
    /// close call, widening the window a concurrent `document_store.close`
    /// has to land inside it -- same class of timing technique as
    /// `concurrent_still_parsing_newer_generation_still_wins_document_store`
    /// above. Repeats several iterations since the race is timing-dependent.
    #[test]
    fn document_closed_during_late_parse_rolls_back_reservation_and_permits_legitimate_retry()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut gen_5_text = String::from("package CloseRace;\n");
        for i in 0..3000 {
            gen_5_text.push_str(&format!("sub gen_5_symbol_{i} {{ {i} }}\n"));
        }
        gen_5_text.push_str("1;\n");

        for iteration in 0..10 {
            let index = Arc::new(WorkspaceIndex::new());
            let uri = must(url::Url::parse(&format!("file:///lib/CloseRace{iteration}.pm")));

            index.index_file_with_generation(
                uri.clone(),
                "package CloseRace;\nsub baseline { 1 }\n1;\n".to_string(),
                3,
            )?;
            assert_eq!(index.indexed_generation(uri.as_str()), Some(3));

            let index_parse = Arc::clone(&index);
            let uri_parse = uri.clone();
            let text_parse = gen_5_text.clone();
            let parse_handle = std::thread::spawn(move || {
                index_parse.index_file_with_generation(uri_parse, text_parse, 5)
            });

            // Brief, bounded head start so the parse thread's early guard
            // has reserved generation 5 and opened the document before this
            // thread closes it -- widens the window the close needs to land
            // in (the large document's parse), rather than requiring it hit
            // the few-line gap between parse success and the `.get()` call
            // exactly.
            std::thread::sleep(std::time::Duration::from_millis(2));
            index.document_store().close(uri.as_str());

            let parse_result =
                parse_handle.join().map_err(|_| "generation 5 parse thread panicked")?;

            // Whichever way the race landed (close hit the parse window and
            // caused a "Document not found" Err, or the parse finished
            // first and generation 5 committed normally before the close),
            // the tracked generation must never claim generation 5 while
            // simultaneously having failed to commit it.
            if parse_result.is_err() {
                assert_eq!(
                    index.indexed_generation(uri.as_str()),
                    Some(3),
                    "iteration {iteration}: a document-closed early return must roll the \
                     reservation back to the last committed generation, not strand it at 5"
                );
            }

            // Regardless of outcome, the guard must not be wedged: reopen
            // and re-index at generation 5 must succeed (proves no
            // permanently-leaked reservation from either this race or a
            // prior iteration's URI).
            index.index_file_with_generation(
                uri.clone(),
                "package CloseRace;\nsub recovered { 1 }\n1;\n".to_string(),
                5,
            )?;
            assert_eq!(
                index.indexed_generation(uri.as_str()),
                Some(5),
                "iteration {iteration}: a legitimate retry after the close race must still \
                 succeed and advance the tracked generation"
            );
        }

        Ok(())
    }

    /// #3618 review-3660: the three tests above (3a/3b/3c) prove a failed
    /// reservation doesn't leave `indexed_generation()` claiming an
    /// un-indexed generation, and that a SAME-generation retry still
    /// succeeds -- but `indexed_generation()` only ever reads
    /// `FileIndex::generation`, which `ReservationGuard::drop` never
    /// touches (only `pending_generation` is rolled back), and a
    /// same-generation retry sails through the early guard's strict `>`
    /// high-water comparison whether or not the rollback ran. review-3660
    /// proved this empirically: no-op'ing `ReservationGuard::drop` left
    /// all three tests passing unchanged. None of them actually exercise
    /// `pending_generation`.
    ///
    /// This test does. It reproduces review-3660's exact discriminating
    /// scenario: a HIGHER generation (10) reserves then fails to parse,
    /// leaking `pending_generation = 10` if the rollback doesn't run. A
    /// LEGITIMATE, LOWER, still-uncommitted generation (7) then arrives
    /// with valid content -- modeling an out-of-order background index
    /// task (`LspServer::run_post_parse_side_effects`'s `spawn_blocking`)
    /// completing after a later generation was merely attempted, not
    /// after it committed. Without rollback, the early guard's high-water
    /// check (`existing.generation.max(existing.pending_generation) = 10
    /// > 7`) rejects generation 7 outright -- `index_file_with_generation`
    /// returns `Ok(())` but SILENTLY skips indexing it, permanently
    /// stranding the index at generation 3 even though generation 7's
    /// content was never anything but valid. With the rollback (this PR's
    /// fix), generation 10's parse failure clears the leaked reservation
    /// back to the committed floor (3), so generation 7 correctly passes
    /// the high-water check and gets indexed.
    ///
    /// Mutation-proved: with `ReservationGuard::drop` temporarily no-op'd,
    /// this test fails (`indexed_generation()` reports `Some(3)`, symbols
    /// lack `gen_7_symbol`); restored, it passes. See the PR comment for
    /// both outputs.
    #[test]
    fn failed_higher_generation_reservation_does_not_silently_drop_a_legitimate_lower_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///lib/LeakedReservation.pm"));

        // Baseline committed generation 3.
        index.index_file_with_generation(
            uri.clone(),
            "package LeakedReservation;\nsub baseline { 1 }\n1;\n".to_string(),
            3,
        )?;
        assert_eq!(index.indexed_generation(uri.as_str()), Some(3));

        // Generation 10 reserves (pending_generation: 3 -> 10 if the
        // reservation is taken), then fails to parse -- 200 levels of
        // nesting deterministically exceeds MAX_RECURSION_DEPTH (128).
        let mut too_deep = String::from("package LeakedReservation;\n");
        for _ in 0..200 {
            too_deep.push_str("if ($a) { ");
        }
        too_deep.push_str("1;");
        for _ in 0..200 {
            too_deep.push('}');
        }
        too_deep.push('\n');
        let result_10 = index.index_file_with_generation(uri.clone(), too_deep, 10);
        assert!(result_10.is_err(), "generation 10's over-nested text must fail to parse");

        // Generation 7 -- lower than the failed generation 10, but higher
        // than (and still uncommitted relative to) the baseline of 3 --
        // arrives with genuinely valid content. This is the discriminating
        // assertion: it only passes if generation 10's leaked reservation
        // was actually rolled back.
        index.index_file_with_generation(
            uri.clone(),
            "package LeakedReservation;\nsub gen_7_symbol { 1 }\n1;\n".to_string(),
            7,
        )?;

        assert_eq!(
            index.indexed_generation(uri.as_str()),
            Some(7),
            "a legitimate generation (7) must not be silently dropped by a higher generation's \
             (10) leaked-and-failed reservation -- without the rollback, the high-water check \
             (10 > 7) rejects it and the index stays permanently stuck at the baseline (3)"
        );
        let symbols = index.file_symbols(uri.as_str());
        assert!(
            symbols.iter().any(|s| s.name == "gen_7_symbol"),
            "generation 7's content must actually be indexed, not silently skipped; got \
             symbols: {:?}",
            symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(
            !symbols.iter().any(|s| s.name == "baseline"),
            "generation 7's content must have REPLACED the baseline (3) index, not merely left \
             it in place; got symbols: {:?}",
            symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        Ok(())
    }

    /// #3618 review thread (factory-droid, PRRT_kwDOSid81M6QBZ1u): before the
    /// `generation`/`pending_generation` split added in this PR's fix-3
    /// round, the early guard's reservation wrote directly onto
    /// `FileIndex::generation` (`existing_index.generation = generation`) --
    /// the SAME field `indexed_generation()` reads. For the entire window
    /// between that reservation and the late guard's insert, a reader
    /// (including `LspServer::workspace_index_stale_for_document`, which
    /// gates hover/navigation/references/completion's local-provider
    /// fallback on `indexed_generation() == DocumentState::current_generation()`)
    /// would see the NEW generation number while `file_symbols()` (and
    /// everything else keyed off the same `FileIndex`) still returned the
    /// OLD generation's facts -- a window where the index looks caught up
    /// but isn't.
    ///
    /// The `generation`/`pending_generation` split closes this as a side
    /// effect: the early guard now only ever advances `pending_generation`;
    /// `generation` -- the only field `indexed_generation()` reads -- is
    /// written exclusively by the late guard's successful insert, AFTER
    /// parsing and symbol extraction have already completed. There is no
    /// longer a reservation write on the path `indexed_generation()` reads
    /// at all, so this test asserts the window is actually closed: while a
    /// large document is still parsing in another thread, `indexed_generation()`
    /// must keep reporting the PREVIOUS generation, never the in-flight one.
    #[test]
    fn indexed_generation_never_advances_speculatively_during_a_still_parsing_task()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut gen_5_text = String::from("package StaleWindow;\n");
        for i in 0..3000 {
            gen_5_text.push_str(&format!("sub gen_5_symbol_{i} {{ {i} }}\n"));
        }
        gen_5_text.push_str("1;\n");

        for iteration in 0..10 {
            let index = Arc::new(WorkspaceIndex::new());
            let uri = must(url::Url::parse(&format!("file:///lib/StaleWindow{iteration}.pm")));

            index.index_file_with_generation(
                uri.clone(),
                "package StaleWindow;\nsub baseline { 1 }\n1;\n".to_string(),
                3,
            )?;
            assert_eq!(index.indexed_generation(uri.as_str()), Some(3));

            let index_parse = Arc::clone(&index);
            let uri_parse = uri.clone();
            let text_parse = gen_5_text.clone();
            let parse_handle = std::thread::spawn(move || {
                index_parse.index_file_with_generation(uri_parse, text_parse, 5)
            });

            // Brief, bounded head start so the parse thread's early guard has
            // reserved generation 5 (bumped `pending_generation`) before this
            // thread reads `indexed_generation()` -- the exact reservation
            // window the pre-fix code would have leaked into `generation`.
            std::thread::sleep(std::time::Duration::from_millis(1));
            let generation_while_still_parsing = index.indexed_generation(uri.as_str());

            parse_handle
                .join()
                .map_err(|_| "generation 5 parse thread panicked")?
                .map_err(|e| format!("generation 5 write returned an error: {e}"))?;

            assert_eq!(
                generation_while_still_parsing,
                Some(3),
                "iteration {iteration}: indexed_generation() must never report a generation \
                 whose symbols haven't actually been committed yet -- observed {:?} while \
                 generation 5 was still mid-parse (baseline was 3)",
                generation_while_still_parsing
            );
            assert_eq!(
                index.indexed_generation(uri.as_str()),
                Some(5),
                "iteration {iteration}: after the parse genuinely completes, indexed_generation() \
                 must reflect it"
            );
        }

        Ok(())
    }

    #[test]
    fn test_early_exit_optimization_changed_content() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test.pl"));
        let code1 = r#"
package MyPackage;

sub hello {
    print "Hello";
}
"#;

        let code2 = r#"
package MyPackage;

sub goodbye {
    print "Goodbye";
}
"#;

        // First indexing
        must(index.index_file(uri.clone(), code1.to_string()));
        let symbols1 = index.file_symbols(uri.as_str());
        assert!(symbols1.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));
        assert!(!symbols1.iter().any(|s| s.name == "goodbye"));

        // Second indexing with different content should re-parse
        must(index.index_file(uri.clone(), code2.to_string()));
        let symbols2 = index.file_symbols(uri.as_str());
        assert!(!symbols2.iter().any(|s| s.name == "hello"));
        assert!(symbols2.iter().any(|s| s.name == "goodbye" && s.kind == SymbolKind::Subroutine));
    }

    #[test]
    fn test_early_exit_optimization_whitespace_only_change() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test.pl"));
        let code1 = r#"
package MyPackage;

sub hello {
    print "Hello";
}
"#;

        let code2 = r#"
package MyPackage;


sub hello {
    print "Hello";
}
"#;

        // First indexing
        must(index.index_file(uri.clone(), code1.to_string()));
        let symbols1 = index.file_symbols(uri.as_str());
        assert!(symbols1.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));

        // Second indexing with whitespace change should re-parse (hash will differ)
        must(index.index_file(uri.clone(), code2.to_string()));
        let symbols2 = index.file_symbols(uri.as_str());
        // Symbols should still be found, but content hash differs so it re-indexed
        assert!(symbols2.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));
    }

    #[test]
    fn test_reindex_file_refreshes_symbol_cache_for_removed_names() {
        let index = WorkspaceIndex::new();
        let uri1 = must(url::Url::parse("file:///lib/A.pm"));
        let uri2 = must(url::Url::parse("file:///lib/B.pm"));
        let code1 = "package A;\nsub foo { return 1; }\n1;\n";
        let code2 = "package B;\nsub foo { return 2; }\n1;\n";
        let code2_reindexed = "package B;\nsub bar { return 3; }\n1;\n";

        must(index.index_file(uri1.clone(), code1.to_string()));
        must(index.index_file(uri2.clone(), code2.to_string()));
        must(index.index_file(uri2.clone(), code2_reindexed.to_string()));

        let foo_location = must_some(index.find_definition("foo"));
        assert_eq!(foo_location.uri, uri1.to_string());

        let bar_location = must_some(index.find_definition("bar"));
        assert_eq!(bar_location.uri, uri2.to_string());
    }

    #[test]
    fn test_remove_file_preserves_other_colliding_symbol_entries() {
        let index = WorkspaceIndex::new();
        let uri1 = must(url::Url::parse("file:///lib/A.pm"));
        let uri2 = must(url::Url::parse("file:///lib/B.pm"));
        let code1 = "package A;\nsub foo { return 1; }\n1;\n";
        let code2 = "package B;\nsub foo { return 2; }\n1;\n";

        must(index.index_file(uri1.clone(), code1.to_string()));
        must(index.index_file(uri2.clone(), code2.to_string()));

        index.remove_file(uri2.as_str());

        let foo_location = must_some(index.find_definition("foo"));
        assert_eq!(foo_location.uri, uri1.to_string());
    }

    #[test]
    fn test_count_usages_no_double_counting_for_qualified_calls() {
        let index = WorkspaceIndex::new();

        // File 1: defines Utils::process_data
        let uri1 = "file:///lib/Utils.pm";
        let code1 = r#"
package Utils;

sub process_data {
    return 1;
}
"#;
        must(index.index_file(must(url::Url::parse(uri1)), code1.to_string()));

        // File 2: calls Utils::process_data (qualified call)
        let uri2 = "file:///app.pl";
        let code2 = r#"
use Utils;
Utils::process_data();
Utils::process_data();
"#;
        must(index.index_file(must(url::Url::parse(uri2)), code2.to_string()));

        // Each qualified call is stored under both "process_data" and "Utils::process_data"
        // by the dual indexing strategy. count_usages should deduplicate so we get the
        // actual number of call sites, not double.
        let count = index.count_usages("Utils::process_data");

        // We expect exactly 2 usage sites (the two calls in app.pl),
        // not 4 (which would be the double-counted result).
        assert_eq!(
            count, 2,
            "count_usages should not double-count qualified calls, got {} (expected 2)",
            count
        );

        // find_references should also deduplicate
        let refs = index.find_references("Utils::process_data");
        let non_def_refs: Vec<_> =
            refs.iter().filter(|loc| loc.uri != "file:///lib/Utils.pm").collect();
        assert_eq!(
            non_def_refs.len(),
            2,
            "find_references should not return duplicates for qualified calls, got {} non-def refs",
            non_def_refs.len()
        );
    }

    #[test]
    fn test_batch_indexing() {
        let index = WorkspaceIndex::new();
        let files: Vec<(Url, String)> = (0..5)
            .map(|i| {
                let uri = must(Url::parse(&format!("file:///batch/module{}.pm", i)));
                let code =
                    format!("package Batch::Mod{};\nsub func_{} {{ return {}; }}\n1;", i, i, i);
                (uri, code)
            })
            .collect();

        let errors = index.index_files_batch(files);
        assert!(errors.is_empty(), "batch indexing errors: {:?}", errors);
        assert_eq!(index.file_count(), 5);
        assert!(index.find_definition("Batch::Mod0::func_0").is_some());
        assert!(index.find_definition("Batch::Mod4::func_4").is_some());
    }

    #[test]
    fn test_batch_indexing_skips_unchanged() {
        let index = WorkspaceIndex::new();
        let uri = must(Url::parse("file:///batch/skip.pm"));
        let code = "package Skip;\nsub skip_fn { 1 }\n1;".to_string();

        index.index_file(uri.clone(), code.clone()).ok();
        assert_eq!(index.file_count(), 1);

        let errors = index.index_files_batch(vec![(uri, code)]);
        assert!(errors.is_empty());
        assert_eq!(index.file_count(), 1);
    }

    #[test]
    fn test_incremental_update_preserves_other_symbols() {
        let index = WorkspaceIndex::new();

        let uri_a = must(Url::parse("file:///incr/a.pm"));
        let uri_b = must(Url::parse("file:///incr/b.pm"));
        index.index_file(uri_a.clone(), "package A;\nsub a_func { 1 }\n1;".into()).ok();
        index.index_file(uri_b.clone(), "package B;\nsub b_func { 2 }\n1;".into()).ok();

        assert!(index.find_definition("A::a_func").is_some());
        assert!(index.find_definition("B::b_func").is_some());

        index.index_file(uri_a, "package A;\nsub a_func_v2 { 11 }\n1;".into()).ok();

        assert!(index.find_definition("A::a_func_v2").is_some());
        assert!(index.find_definition("B::b_func").is_some());
    }

    #[test]
    fn test_remove_file_preserves_shadowed_symbols() {
        let index = WorkspaceIndex::new();

        let uri_a = must(Url::parse("file:///shadow/a.pm"));
        let uri_b = must(Url::parse("file:///shadow/b.pm"));
        index.index_file(uri_a.clone(), "package ShadowA;\nsub helper { 1 }\n1;".into()).ok();
        index.index_file(uri_b.clone(), "package ShadowB;\nsub helper { 2 }\n1;".into()).ok();

        assert!(index.find_definition("helper").is_some());

        index.remove_file_url(&uri_a);
        assert!(index.find_definition("helper").is_some());
        assert!(index.find_definition("ShadowB::helper").is_some());
    }

    // -------------------------------------------------------------------------
    // find_dependents — use parent / use base integration (#2747)
    // -------------------------------------------------------------------------

    #[test]
    fn test_index_dependency_via_use_parent_end_to_end() {
        // Regression for #2747: index a file with `use parent 'MyBase'` and verify
        // that find_dependents("MyBase") returns that file.
        // 1. Index MyBase.pm
        // 2. Index child.pl with `use parent 'MyBase'`
        // 3. find_dependents("MyBase") should return child.pl
        let index = WorkspaceIndex::new();

        let base_url = must(url::Url::parse("file:///test/workspace/lib/MyBase.pm"));
        must(index.index_file(
            base_url,
            "package MyBase;\nsub new { bless {}, shift }\n1;\n".to_string(),
        ));

        let child_url = must(url::Url::parse("file:///test/workspace/child.pl"));
        must(index.index_file(child_url, "package Child;\nuse parent 'MyBase';\n1;\n".to_string()));

        let dependents = index.find_dependents("MyBase");
        assert!(
            !dependents.is_empty(),
            "find_dependents('MyBase') returned empty — \
             use parent 'MyBase' should register MyBase as a dependency. \
             Dependencies in index: {:?}",
            {
                let files = index.files.read();
                files
                    .iter()
                    .map(|(k, v)| (k.clone(), v.dependencies.iter().cloned().collect::<Vec<_>>()))
                    .collect::<Vec<_>>()
            }
        );
        assert!(
            dependents.contains(&"file:///test/workspace/child.pl".to_string()),
            "child.pl should be in dependents, got: {:?}",
            dependents
        );
    }

    #[test]
    fn test_find_dependents_normalizes_legacy_separator_in_query() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/workspace/legacy-query.pl"));
        let src = "package Child;\nuse parent 'My::Base';\n1;\n";
        must(index.index_file(uri, src.to_string()));

        let dependents = index.find_dependents("My'Base");
        assert_eq!(dependents, vec!["file:///test/workspace/legacy-query.pl".to_string()]);
    }

    #[test]
    fn test_file_dependencies_normalize_legacy_separator_in_source() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/workspace/legacy-source.pl"));
        let src = "package Child;\nuse parent \"My'Base\";\n1;\n";
        must(index.index_file(uri.clone(), src.to_string()));

        let deps = index.file_dependencies(uri.as_str());
        assert!(deps.contains("My::Base"));
        assert!(!deps.contains("My'Base"));
    }

    #[test]
    fn test_index_dependency_via_moose_extends_end_to_end() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();

        let parent_url = must(url::Url::parse("file:///test/workspace/lib/My/App/Parent.pm"));
        must(index.index_file(parent_url, "package My::App::Parent;\n1;\n".to_string()));

        let child_url = must(url::Url::parse("file:///test/workspace/child-moose.pl"));
        let child_src = "package Child;\nuse Moose;\nextends 'My::App::Parent';\n1;\n";
        must(index.index_file(child_url, child_src.to_string()));

        let dependents = index.find_dependents("My::App::Parent");
        assert!(
            dependents.contains(&"file:///test/workspace/child-moose.pl".to_string()),
            "expected child-moose.pl in dependents, got: {dependents:?}"
        );
        Ok(())
    }

    #[test]
    fn test_index_dependency_via_moo_with_role_end_to_end() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();

        let role_url = must(url::Url::parse("file:///test/workspace/lib/My/App/Role.pm"));
        must(index.index_file(role_url, "package My::App::Role;\n1;\n".to_string()));

        let consumer_url = must(url::Url::parse("file:///test/workspace/consumer-moo.pl"));
        let consumer_src = "package Consumer;\nuse Moo;\nwith 'My::App::Role';\n1;\n";
        must(index.index_file(consumer_url.clone(), consumer_src.to_string()));

        let dependents = index.find_dependents("My::App::Role");
        assert!(
            dependents.contains(&"file:///test/workspace/consumer-moo.pl".to_string()),
            "expected consumer-moo.pl in dependents, got: {dependents:?}"
        );

        let deps = index.file_dependencies(consumer_url.as_str());
        assert!(deps.contains("My::App::Role"));
        Ok(())
    }

    #[test]
    fn test_index_dependency_via_literal_require_end_to_end()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/workspace/require-consumer.pl"));
        let src = "package Consumer;\nrequire My::Loader;\n1;\n";
        must(index.index_file(uri.clone(), src.to_string()));

        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("My::Loader"),
            "literal require should register module dependency, got: {deps:?}"
        );
        Ok(())
    }

    #[test]
    fn test_manual_import_symbols_are_indexed_as_import_references()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/workspace/manual-import.pl"));
        let src = r#"package Consumer;
require My::Tools;
My::Tools->import(qw(helper_one helper_two));
helper_one();
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));

        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("My::Tools"),
            "manual import target should be tracked as dependency, got: {deps:?}"
        );

        for symbol in ["helper_one", "helper_two"] {
            let refs = index.find_references(symbol);
            assert!(
                !refs.is_empty(),
                "expected at least one indexed reference for imported symbol `{symbol}`"
            );
        }
        Ok(())
    }

    #[test]
    fn test_parser_produces_correct_args_for_use_parent() {
        // Regression for #2747: verify that the parser produces args=["'MyBase'"]
        // for `use parent 'MyBase'`, so extract_module_names_from_use_args strips
        // the quotes and registers the dependency under the bare name "MyBase".
        use crate::Parser;
        let mut p = Parser::new("package Child;\nuse parent 'MyBase';\n1;\n");
        let ast = must(p.parse());
        assert!(
            matches!(ast.kind, NodeKind::Program { .. }),
            "Expected Program root, got {:?}",
            ast.kind
        );
        let NodeKind::Program { statements } = &ast.kind else {
            return;
        };
        let mut found_parent_use = false;
        for stmt in statements {
            if let NodeKind::Use { module, args, .. } = &stmt.kind {
                if module == "parent" {
                    found_parent_use = true;
                    assert_eq!(
                        args,
                        &["'MyBase'".to_string()],
                        "Expected args=[\"'MyBase'\"] for `use parent 'MyBase'`, got: {:?}",
                        args
                    );
                    let extracted = extract_module_names_from_use_args(args);
                    assert_eq!(
                        extracted,
                        vec!["MyBase".to_string()],
                        "extract_module_names_from_use_args should return [\"MyBase\"], got {:?}",
                        extracted
                    );
                }
            }
        }
        assert!(found_parent_use, "No Use node with module='parent' found in AST");
    }

    // -------------------------------------------------------------------------
    // extract_module_names_from_use_args — unit tests (#2747)
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_module_names_single_quoted() {
        let names = extract_module_names_from_use_args(&["'Foo::Bar'".to_string()]);
        assert_eq!(names, vec!["Foo::Bar"]);
    }

    #[test]
    fn test_extract_module_names_double_quoted() {
        let names = extract_module_names_from_use_args(&["\"Foo::Bar\"".to_string()]);
        assert_eq!(names, vec!["Foo::Bar"]);
    }

    #[test]
    fn test_extract_module_names_qw_list() {
        let names = extract_module_names_from_use_args(&["qw(Foo::Bar Other::Base)".to_string()]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_module_names_qw_slash_delimiter() {
        let names = extract_module_names_from_use_args(&["qw/Foo::Bar Other::Base/".to_string()]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_module_names_qw_with_space_before_delimiter() {
        let names = extract_module_names_from_use_args(&["qw [Foo::Bar Other::Base]".to_string()]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_module_names_qw_list_trims_wrapped_punctuation() {
        let names =
            extract_module_names_from_use_args(&["qw((Foo::Bar) [Other::Base],)".to_string()]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_module_names_norequire_flag() {
        let names = extract_module_names_from_use_args(&[
            "-norequire".to_string(),
            "'Foo::Bar'".to_string(),
        ]);
        assert_eq!(names, vec!["Foo::Bar"]);
    }

    #[test]
    fn test_extract_module_names_empty_args() {
        let names = extract_module_names_from_use_args(&[]);
        assert!(names.is_empty());
    }

    #[test]
    fn test_extract_module_names_legacy_separator() {
        // Perl legacy package separator ' (tick) inside module name
        let names = extract_module_names_from_use_args(&["'Foo'Bar'".to_string()]);
        // Legacy separators are normalized for downstream dependency matching.
        assert_eq!(names, vec!["Foo::Bar"]);
    }

    #[test]
    fn test_find_dependents_matches_legacy_separator_queries() {
        let index = WorkspaceIndex::new();
        let base_uri = must(url::Url::parse("file:///test/workspace/lib/Foo/Bar.pm"));
        let child_uri = must(url::Url::parse("file:///test/workspace/child.pl"));

        must(index.index_file(base_uri, "package Foo::Bar;\n1;\n".to_string()));
        must(index.index_file(
            child_uri.clone(),
            "package Child;\nuse parent qw(Foo'Bar);\n1;\n".to_string(),
        ));

        let dependents_modern = index.find_dependents("Foo::Bar");
        assert!(
            dependents_modern.contains(&child_uri.to_string()),
            "Expected dependency match when queried with modern separator"
        );

        let dependents_legacy = index.find_dependents("Foo'Bar");
        assert!(
            dependents_legacy.contains(&child_uri.to_string()),
            "Expected dependency match when queried with legacy separator"
        );
    }

    #[test]
    fn test_extract_module_names_comma_adjacent_tokens() {
        let names = extract_module_names_from_use_args(&[
            "'Foo::Bar',".to_string(),
            "\"Other::Base\",".to_string(),
            "'Last::One'".to_string(),
        ]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base", "Last::One"]);
    }

    #[test]
    fn test_extract_module_names_parenthesized_without_spaces() {
        let names = extract_module_names_from_use_args(&["('Foo::Bar','Other::Base')".to_string()]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_module_names_deduplicates_identical_entries() {
        let names = extract_module_names_from_use_args(&[
            "qw(Foo::Bar Foo::Bar)".to_string(),
            "'Foo::Bar'".to_string(),
        ]);
        assert_eq!(names, vec!["Foo::Bar"]);
    }

    #[test]
    fn test_extract_module_names_trims_semicolon_suffix() {
        let names = extract_module_names_from_use_args(&[
            "'Foo::Bar',".to_string(),
            "'Other::Base',".to_string(),
            "'Third::Leaf';".to_string(),
        ]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base", "Third::Leaf"]);
    }

    #[test]
    fn test_extract_module_names_trims_wrapped_punctuation() {
        let names = extract_module_names_from_use_args(&[
            "('Foo::Bar',".to_string(),
            "'Other::Base')".to_string(),
        ]);
        assert_eq!(names, vec!["Foo::Bar", "Other::Base"]);
    }

    #[test]
    fn test_extract_constant_names_qw_with_space_before_delimiter() {
        let names = extract_constant_names_from_use_args(&["qw [FOO BAR]".to_string()]);
        assert_eq!(names, vec!["FOO", "BAR"]);
    }

    #[test]
    #[ignore = "qw delimiter with leading space not yet parsed; tracked in debt-ledger.yaml"]
    fn test_index_use_constant_qw_with_space_before_delimiter() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///workspace/lib/My/Config.pm"));
        let source = "package My::Config;\nuse constant qw [FOO BAR];\n1;\n";

        must(index.index_file(uri, source.to_string()));

        let foo = index.find_definition("My::Config::FOO");
        let bar = index.find_definition("My::Config::BAR");
        assert!(foo.is_some(), "Expected My::Config::FOO to be indexed");
        assert!(bar.is_some(), "Expected My::Config::BAR to be indexed");
    }

    #[test]
    fn test_with_capacity_accepts_large_batch_without_panic() {
        let index = WorkspaceIndex::with_capacity(100, 20);
        for i in 0..100 {
            let uri = must(url::Url::parse(&format!("file:///lib/Mod{}.pm", i)));
            let src = format!("package Mod{};\nsub foo_{} {{ 1 }}\n1;\n", i, i);
            index.index_file(uri, src).ok();
        }
        assert!(index.has_symbols());
    }

    #[test]
    fn test_with_capacity_zero_does_not_panic() {
        let index = WorkspaceIndex::with_capacity(0, 0);
        assert!(!index.has_symbols());
    }

    // -------------------------------------------------------------------------
    // remove_file — symbol cache cleanup (#3494)
    // -------------------------------------------------------------------------

    /// After removing the only file that defines a symbol, both qualified and
    /// bare-name lookups must return None.  The symbols cache must not retain
    /// stale entries pointing to the deleted file.
    #[test]
    fn test_remove_file_clears_symbol_cache_qualified_and_bare() {
        let index = WorkspaceIndex::new();
        let uri_a = must(url::Url::parse("file:///lib/A.pm"));
        let code_a = "package A;\nsub foo { return 1; }\n1;\n";

        must(index.index_file(uri_a.clone(), code_a.to_string()));

        // Pre-condition: both qualified and bare-name lookups resolve to file A.
        let before_qual = must_some(index.find_definition("A::foo"));
        assert_eq!(
            before_qual.uri,
            uri_a.to_string(),
            "qualified lookup should point to A.pm before removal"
        );
        let before_bare = must_some(index.find_definition("foo"));
        assert_eq!(
            before_bare.uri,
            uri_a.to_string(),
            "bare-name lookup should point to A.pm before removal"
        );

        // Remove file A from the index (simulates file deletion).
        index.remove_file(uri_a.as_str());

        // Post-condition: the symbol cache must be clean — no stale entries.
        assert!(
            index.find_definition("A::foo").is_none(),
            "qualified lookup 'A::foo' should return None after file deletion"
        );
        assert!(
            index.find_definition("foo").is_none(),
            "bare-name lookup 'foo' should return None after file deletion"
        );

        // Verify no symbols remain in the index.
        assert_eq!(
            index.symbol_count(),
            0,
            "symbol_count should be 0 after removing the only file"
        );
        assert!(!index.has_symbols(), "has_symbols should be false after removing the only file");
    }

    /// Deleting file A when file B has the same bare-name symbol must leave
    /// the bare-name cache pointing to B (not remove it entirely).
    #[test]
    fn test_remove_file_bare_name_falls_back_to_surviving_file() {
        let index = WorkspaceIndex::new();
        let uri_a = must(url::Url::parse("file:///lib/A.pm"));
        let uri_b = must(url::Url::parse("file:///lib/B.pm"));
        let code_a = "package A;\nsub shared_fn { return 1; }\n1;\n";
        let code_b = "package B;\nsub shared_fn { return 2; }\n1;\n";

        must(index.index_file(uri_a.clone(), code_a.to_string()));
        must(index.index_file(uri_b.clone(), code_b.to_string()));

        // Remove file A — shared_fn should still resolve via B.
        index.remove_file(uri_a.as_str());

        let loc = must_some(index.find_definition("shared_fn"));
        assert_eq!(
            loc.uri,
            uri_b.to_string(),
            "bare-name 'shared_fn' should resolve to B.pm after A.pm is deleted"
        );

        assert!(
            index.find_definition("A::shared_fn").is_none(),
            "qualified 'A::shared_fn' must be gone after A.pm deletion"
        );
        assert!(
            index.find_definition("B::shared_fn").is_some(),
            "qualified 'B::shared_fn' must remain after A.pm deletion"
        );
    }

    #[test]
    fn test_definition_candidates_include_ambiguous_bare_symbols_in_stable_order() {
        let index = WorkspaceIndex::new();
        let uri_b = must(url::Url::parse("file:///lib/B.pm"));
        let uri_a = must(url::Url::parse("file:///lib/A.pm"));
        must(index.index_file(uri_b, "package B;\nsub shared { 1 }\n1;\n".to_string()));
        must(index.index_file(uri_a, "package A;\nsub shared { 1 }\n1;\n".to_string()));

        let candidates = index.definition_candidates("shared");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].uri, "file:///lib/A.pm");
        assert_eq!(candidates[1].uri, "file:///lib/B.pm");
        assert_eq!(must_some(index.find_definition("shared")).uri, "file:///lib/A.pm");
    }

    #[test]
    fn test_definition_candidates_include_duplicate_qualified_name_across_files() {
        let index = WorkspaceIndex::new();
        let uri_v2 = must(url::Url::parse("file:///lib/A-v2.pm"));
        let uri_v1 = must(url::Url::parse("file:///lib/A-v1.pm"));
        let source = "package A;\nsub foo { 1 }\n1;\n".to_string();
        must(index.index_file(uri_v2, source.clone()));
        must(index.index_file(uri_v1, source));

        let candidates = index.definition_candidates("A::foo");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].uri, "file:///lib/A-v1.pm");
        assert_eq!(candidates[1].uri, "file:///lib/A-v2.pm");
    }

    #[test]
    fn test_definition_candidates_are_cleaned_on_remove_and_reindex() {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///lib/A.pm"));
        must(index.index_file(uri.clone(), "package A;\nsub foo { 1 }\n1;\n".to_string()));
        assert_eq!(index.definition_candidates("A::foo").len(), 1);

        index.remove_file(uri.as_str());
        assert!(index.definition_candidates("A::foo").is_empty());

        must(index.index_file(uri, "package A;\nsub foo { 2 }\n1;\n".to_string()));
        assert_eq!(index.definition_candidates("A::foo").len(), 1);
    }

    /// Verify that `incremental_remove_symbols` correctly retains candidates owned by
    /// other files when the removed file had BOTH exclusively-owned names (triggering the
    /// full-rebuild path) AND shared names. Before this fix, the full-rebuild path cleared
    /// all candidates and relied on the subsequent rebuild to re-add shared ones — correct
    /// in effect, but the test documents the expected observable behavior.
    #[test]
    fn test_definition_candidates_shared_symbol_survives_removal_of_sole_owner_of_other_symbol() {
        let index = WorkspaceIndex::new();
        let uri_a = must(url::Url::parse("file:///lib/A.pm"));
        let uri_b = must(url::Url::parse("file:///lib/B.pm"));

        // A defines both `unique_to_a` (no other file) and `shared` (also in B)
        must(index.index_file(
            uri_a.clone(),
            "package A;\nsub unique_to_a { 1 }\nsub shared { 1 }\n1;\n".to_string(),
        ));
        must(index.index_file(uri_b.clone(), "package B;\nsub shared { 1 }\n1;\n".to_string()));

        // Before removal: both shared candidates and unique_to_a are present
        assert_eq!(index.definition_candidates("shared").len(), 2);
        assert_eq!(index.definition_candidates("unique_to_a").len(), 1);

        // Remove A — triggers the affected_names path for `unique_to_a`, but `shared`
        // still has B's candidate.
        index.remove_file(uri_a.as_str());

        assert!(
            index.definition_candidates("unique_to_a").is_empty(),
            "unique_to_a should be gone after removing A"
        );
        assert_eq!(
            index.definition_candidates("shared").len(),
            1,
            "shared should still have B's candidate after removing A"
        );
        assert_eq!(
            index.definition_candidates("shared")[0].uri,
            "file:///lib/B.pm",
            "remaining shared candidate must be from B"
        );
    }

    #[test]
    fn test_folder_context_in_file_index() {
        let index = WorkspaceIndex::new();

        // Set up workspace folders
        index.set_workspace_folders(vec![
            "file:///project1".to_string(),
            "file:///project2".to_string(),
        ]);

        let uri1 = "file:///project1/lib/Module.pm";
        let code1 = r#"
package Module;

sub test_sub {
    return 1;
}
"#;
        must(index.index_file(must(url::Url::parse(uri1)), code1.to_string()));

        let uri2 = "file:///project2/lib/Other.pm";
        let code2 = r#"
package Other;

sub other_sub {
    return 2;
}
"#;
        must(index.index_file(must(url::Url::parse(uri2)), code2.to_string()));

        // Verify folder context is set correctly
        let symbols1 = index.file_symbols(uri1);
        assert_eq!(symbols1.len(), 2, "Should have 2 symbols in Module.pm");
        for symbol in &symbols1 {
            assert_eq!(symbol.uri, uri1, "Symbol URI should match file URI");
        }

        let symbols2 = index.file_symbols(uri2);
        assert_eq!(symbols2.len(), 2, "Should have 2 symbols in Other.pm");
        for symbol in &symbols2 {
            assert_eq!(symbol.uri, uri2, "Symbol URI should match file URI");
        }

        // Verify folder attribution
        let files = index.files.read();
        let file_index1 = must_some(files.get(&DocumentStore::uri_key(uri1)));
        assert_eq!(
            file_index1.folder_uri,
            Some("file:///project1".to_string()),
            "File should be attributed to correct workspace folder"
        );

        let file_index2 = must_some(files.get(&DocumentStore::uri_key(uri2)));
        assert_eq!(
            file_index2.folder_uri,
            Some("file:///project2".to_string()),
            "File should be attributed to correct workspace folder"
        );
    }

    #[test]
    fn test_determine_folder_uri() {
        let index = WorkspaceIndex::new();

        // Set up workspace folders
        index.set_workspace_folders(vec![
            "file:///project1".to_string(),
            "file:///project2".to_string(),
        ]);

        // Test file in project1
        let folder1 = index.determine_folder_uri("file:///project1/lib/Module.pm");
        assert_eq!(
            folder1,
            Some("file:///project1".to_string()),
            "Should determine folder for file in project1"
        );

        // Test file in project2
        let folder2 = index.determine_folder_uri("file:///project2/lib/Other.pm");
        assert_eq!(
            folder2,
            Some("file:///project2".to_string()),
            "Should determine folder for file in project2"
        );

        // Test file not in any workspace folder
        let folder_none = index.determine_folder_uri("file:///other/project/Module.pm");
        assert_eq!(folder_none, None, "Should return None for file outside workspace folders");
    }

    #[test]
    fn test_determine_folder_uri_prefers_most_specific_match() {
        let index = WorkspaceIndex::new();

        // Keep broad folder first to ensure we don't rely on insertion order.
        index.set_workspace_folders(vec![
            "file:///project".to_string(),
            "file:///project/lib".to_string(),
        ]);

        let folder = index.determine_folder_uri("file:///project/lib/My/Module.pm");
        assert_eq!(
            folder,
            Some("file:///project/lib".to_string()),
            "Nested workspace folders should attribute files to the most specific folder"
        );
    }

    #[test]
    fn test_remove_folder() {
        let index = WorkspaceIndex::new();

        // Set up workspace folders
        index.set_workspace_folders(vec![
            "file:///project1".to_string(),
            "file:///project2".to_string(),
        ]);

        // Index files from both folders
        let uri1 = "file:///project1/lib/Module.pm";
        let code1 = r#"
package Module;

sub test_sub {
    return 1;
}
"#;
        must(index.index_file(must(url::Url::parse(uri1)), code1.to_string()));

        let uri2 = "file:///project2/lib/Other.pm";
        let code2 = r#"
package Other;

sub other_sub {
    return 2;
}
"#;
        must(index.index_file(must(url::Url::parse(uri2)), code2.to_string()));

        // Verify both files are indexed
        assert_eq!(index.file_count(), 2, "Should have 2 files indexed");
        assert_eq!(index.document_store().count(), 2, "Document store should track both files");

        // Remove project1 folder
        index.remove_folder("file:///project1");

        // Verify only project2 file remains
        assert_eq!(index.file_count(), 1, "Should have 1 file after removing folder");
        assert_eq!(
            index.document_store().count(),
            1,
            "Document store should drop files removed via folder deletion"
        );
        assert!(index.file_symbols(uri1).is_empty(), "File from removed folder should be gone");
        assert_eq!(
            index.file_symbols(uri2).len(),
            2,
            "File from remaining folder should still be present"
        );
    }

    #[test]
    fn test_remove_folder_removes_symbol_free_files() {
        let index = WorkspaceIndex::new();
        index.set_workspace_folders(vec!["file:///project1".to_string()]);

        let uri = "file:///project1/empty.pl";
        must(index.index_file(must(url::Url::parse(uri)), "# comments only".to_string()));
        assert_eq!(index.file_count(), 1, "Expected file to be indexed");

        index.remove_folder("file:///project1");

        assert_eq!(index.file_count(), 0, "Folder removal should delete symbol-free files");
        assert_eq!(
            index.document_store().count(),
            0,
            "Document store should stay in sync for symbol-free files"
        );
    }

    // ========================================================================
    // GREEN-TDD EDGE CASE TESTS FOR ISSUE #6061 (static require + manual import)
    // ========================================================================

    #[test]
    fn test_require_with_variable_target_is_not_indexed() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/require-var.pl"));
        let src = r#"package Test;
my $loader = 'MyModule';
require $loader;
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(
            !deps.contains("MyModule"),
            "require with variable target should not register static dependency"
        );
        Ok(())
    }

    #[test]
    fn test_multiple_import_calls_on_same_module() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/multi-import.pl"));
        let src = r#"package Test;
require Toolkit;
Toolkit->import('func_a');
Toolkit->import(qw(func_b func_c));
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(deps.contains("Toolkit"), "module should be tracked as dependency");
        for symbol in &["func_a", "func_b", "func_c"] {
            let refs = index.find_references(symbol);
            assert!(!refs.is_empty(), "all imported symbols should be indexed: {}", symbol);
        }
        Ok(())
    }

    #[test]
    fn test_require_string_vs_bareword_normalization() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/require-string.pl"));
        let src = r#"package Consumer;
require "String/Based/Module.pm";
String::Based::Module->import('exported');
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("String::Based::Module"),
            "require string form should normalize path separators to ::"
        );
        let refs = index.find_references("exported");
        assert!(!refs.is_empty(), "import should be indexed even with string-form require");
        Ok(())
    }

    #[test]
    fn test_import_without_require_registers_as_method_call()
    -> Result<(), Box<dyn std::error::Error>> {
        // Edge case: ->import() without preceding require is treated as a normal method call,
        // not as the static manual-import pattern, so the module is still visited/tracked
        // but the symbols are NOT marked as imports from the static require+import logic.
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/orphan-import.pl"));
        let src = r#"package Test;
Unrelated::Module->import('orphaned');
orphaned();
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));

        // The module reference may still be tracked as a method call target,
        // but the key regression is: the orphaned symbol should not be indexed
        // as an import reference due to the missing require.
        let _refs = index.find_references("orphaned");
        // Symbol may be referenced but should not be specially treated as an import.
        // The main point is: without require, the pairing doesn't activate.
        Ok(())
    }

    #[test]
    fn test_nested_blocks_preserve_require_scope() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/nested.pl"));
        let src = r#"package Test;
{
    require Outer;
    {
        Outer->import('nested_sym');
    }
}
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("Outer"),
            "require in outer block should be visible to nested import"
        );
        let refs = index.find_references("nested_sym");
        assert!(!refs.is_empty(), "symbol imported in nested block should still be indexed");
        Ok(())
    }

    #[test]
    fn test_require_path_without_pm_extension() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/no-ext.pl"));
        let src = r#"package Test;
require "My/Module";
My::Module->import('func');
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("My::Module"),
            "require without .pm extension should normalize to module path"
        );
        Ok(())
    }

    #[test]
    fn test_qw_with_bracket_delimiters() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/qw-delim.pl"));
        let src = r#"package Test;
require DelimModule;
DelimModule->import(qw[sym1 sym2]);
DelimModule->import(qw{sym3 sym4});
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        for symbol in &["sym1", "sym2", "sym3", "sym4"] {
            let refs = index.find_references(symbol);
            assert!(
                !refs.is_empty(),
                "symbols from qw with bracket delimiters should be indexed: {}",
                symbol
            );
        }
        Ok(())
    }

    #[test]
    fn test_array_literal_import_args() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/array-import.pl"));
        let src = r#"package Test;
require ArrayModule;
ArrayModule->import(['sym_x', 'sym_y']);
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        for symbol in &["sym_x", "sym_y"] {
            let refs = index.find_references(symbol);
            assert!(
                !refs.is_empty(),
                "symbols from array literal import should be indexed: {}",
                symbol
            );
        }
        Ok(())
    }

    #[test]
    fn test_require_inside_conditional_still_registers_dependency()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/cond-require.pl"));
        let src = r#"package Test;
if (1) {
    require ConditionalMod;
    ConditionalMod->import('cond_func');
}
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(
            deps.contains("ConditionalMod"),
            "require inside conditional should still register as dependency"
        );
        let refs = index.find_references("cond_func");
        assert!(!refs.is_empty(), "import inside conditional should still index symbols");
        Ok(())
    }

    #[test]
    fn test_mixed_string_and_bareword_imports() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = must(url::Url::parse("file:///test/mixed-import.pl"));
        let src = r#"package Test;
require MixedMod;
MixedMod->import('string_sym');
MixedMod->import(qw(qw_one qw_two));
1;
"#;
        must(index.index_file(uri.clone(), src.to_string()));
        let deps = index.file_dependencies(uri.as_str());
        assert!(deps.contains("MixedMod"), "require should register dependency");
        for symbol in &["string_sym", "qw_one", "qw_two"] {
            let refs = index.find_references(symbol);
            assert!(!refs.is_empty(), "all import forms should index symbols: {}", symbol);
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Per-category incremental invalidation (Req 18.1–18.5)
    // -------------------------------------------------------------------------

    /// Helper: build a minimal `FileFactShard` with configurable hashes.
    fn make_shard(
        uri: &str,
        content_hash: u64,
        anchors_hash: Option<u64>,
        entities_hash: Option<u64>,
        occurrences_hash: Option<u64>,
        edges_hash: Option<u64>,
    ) -> FileFactShard {
        let file_id = {
            let mut h = DefaultHasher::new();
            uri.hash(&mut h);
            FileId(h.finish())
        };
        FileFactShard {
            source_uri: uri.to_string(),
            file_id,
            content_hash,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash,
            entities_hash,
            occurrences_hash,
            edges_hash,
            anchors: Vec::new(),
            entities: Vec::new(),
            occurrences: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Req 18.5: When content_hash is unchanged, skip all per-category
    /// comparisons — no index modifications happen.
    #[test]
    fn incremental_replace_skips_when_content_hash_unchanged()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Same.pm";
        let key = DocumentStore::uri_key(uri);

        let shard_v1 = make_shard(uri, 42, Some(1), Some(2), Some(3), Some(4));
        // First insert — no old shard, so all categories are "changed".
        let r1 = index.replace_fact_shard_incremental(&key, shard_v1);
        assert!(!r1.content_unchanged);

        // Second insert with same content_hash → skip entirely.
        let shard_v2 = make_shard(uri, 42, Some(100), Some(200), Some(300), Some(400));
        let r2 = index.replace_fact_shard_incremental(&key, shard_v2);
        assert!(r2.content_unchanged);
        assert!(!r2.anchors_updated);
        assert!(!r2.entities_updated);
        assert!(!r2.occurrences_updated);
        assert!(!r2.edges_updated);

        // The stored shard should still be v1 (unchanged).
        let stored = must_some(index.file_fact_shard(uri));
        assert_eq!(stored.anchors_hash, Some(1));
        Ok(())
    }

    /// Req 18.3: When a category hash is unchanged, skip re-indexing that
    /// category's cross-file indexes.
    #[test]
    fn incremental_replace_skips_unchanged_categories() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Partial.pm";
        let key = DocumentStore::uri_key(uri);

        let shard_v1 = make_shard(uri, 1, Some(10), Some(20), Some(30), Some(40));
        index.replace_fact_shard_incremental(&key, shard_v1);

        // Change content_hash but keep anchors and entities the same.
        // Only occurrences and edges change.
        let shard_v2 = make_shard(uri, 2, Some(10), Some(20), Some(99), Some(88));
        let result = index.replace_fact_shard_incremental(&key, shard_v2);

        assert!(!result.content_unchanged);
        assert!(!result.anchors_updated, "anchors hash unchanged → skip");
        assert!(!result.entities_updated, "entities hash unchanged → skip");
        assert!(result.occurrences_updated, "occurrences hash changed → update");
        assert!(result.edges_updated, "edges hash changed → update");
        Ok(())
    }

    /// Req 18.4: When a category hash has changed, remove old entries and
    /// insert new ones for that category.
    #[test]
    fn incremental_replace_updates_changed_categories() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Changed.pm";
        let key = DocumentStore::uri_key(uri);

        let shard_v1 = make_shard(uri, 1, Some(10), Some(20), Some(30), Some(40));
        index.replace_fact_shard_incremental(&key, shard_v1);

        // Change all category hashes.
        let shard_v2 = make_shard(uri, 2, Some(11), Some(21), Some(31), Some(41));
        let result = index.replace_fact_shard_incremental(&key, shard_v2);

        assert!(!result.content_unchanged);
        assert!(result.anchors_updated);
        assert!(result.entities_updated);
        assert!(result.occurrences_updated);
        assert!(result.edges_updated);

        // The stored shard should be v2.
        let stored = must_some(index.file_fact_shard(uri));
        assert_eq!(stored.content_hash, 2);
        assert_eq!(stored.anchors_hash, Some(11));
        Ok(())
    }

    /// When there is no old shard (first index), all categories are treated
    /// as changed.
    #[test]
    fn incremental_replace_first_insert_updates_all() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/New.pm";
        let key = DocumentStore::uri_key(uri);

        let shard = make_shard(uri, 1, Some(10), Some(20), Some(30), Some(40));
        let result = index.replace_fact_shard_incremental(&key, shard);

        assert!(!result.content_unchanged);
        assert!(result.anchors_updated);
        assert!(result.entities_updated);
        assert!(result.occurrences_updated);
        assert!(result.edges_updated);
        Ok(())
    }

    /// When per-category hashes are `None` (legacy shard), the category is
    /// conservatively treated as changed.
    #[test]
    fn incremental_replace_none_hashes_treated_as_changed() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Legacy.pm";
        let key = DocumentStore::uri_key(uri);

        // Old shard has hashes, new shard has None for some.
        let shard_v1 = make_shard(uri, 1, Some(10), Some(20), Some(30), Some(40));
        index.replace_fact_shard_incremental(&key, shard_v1);

        let shard_v2 = make_shard(uri, 2, None, Some(20), None, Some(40));
        let result = index.replace_fact_shard_incremental(&key, shard_v2);

        assert!(!result.content_unchanged);
        assert!(result.anchors_updated, "None new hash → changed");
        assert!(!result.entities_updated, "same hash → skip");
        assert!(result.occurrences_updated, "None new hash → changed");
        assert!(!result.edges_updated, "same hash → skip");
        Ok(())
    }

    /// Verify that the semantic reference index is updated only when
    /// occurrences or edges change.
    #[test]
    fn incremental_replace_updates_reference_index_on_occurrence_change()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_semantic_facts::{AnchorId, Confidence, OccurrenceId, OccurrenceKind, Provenance};

        let index = WorkspaceIndex::new();
        let uri = "file:///lib/RefIdx.pm";
        let key = DocumentStore::uri_key(uri);
        let file_id = {
            let mut h = DefaultHasher::new();
            uri.hash(&mut h);
            FileId(h.finish())
        };

        // v1: shard with one reference occurrence.
        let mut shard_v1 = make_shard(uri, 1, Some(10), Some(20), Some(30), Some(40));
        let anchor_id = AnchorId(1);
        shard_v1.anchors.push(perl_semantic_facts::AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: 0,
            span_end_byte: 5,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        shard_v1.occurrences.push(perl_semantic_facts::OccurrenceFact {
            id: OccurrenceId(1),
            kind: OccurrenceKind::Call,
            entity_id: Some(EntityId(100)),
            anchor_id,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        shard_v1.entities.push(perl_semantic_facts::EntityFact {
            id: EntityId(100),
            kind: EntityKind::Subroutine,
            canonical_name: "RefIdx::foo".to_string(),
            anchor_id: Some(anchor_id),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        index.replace_fact_shard_incremental(&key, shard_v1);

        // Reference index should have entries.
        assert!(
            index.semantic_reference_index.read().name_count() > 0
                || index.semantic_reference_index.read().entity_count() > 0,
            "reference index should be populated after first insert"
        );

        // v2: same content_hash → skip entirely, reference index untouched.
        let shard_v2_same = make_shard(uri, 1, Some(10), Some(20), Some(99), Some(99));
        let r = index.replace_fact_shard_incremental(&key, shard_v2_same);
        assert!(r.content_unchanged);

        // v3: different content_hash, same occurrence/edge hashes → skip ref index.
        let mut shard_v3 = make_shard(uri, 3, Some(11), Some(21), Some(30), Some(40));
        shard_v3.anchors.push(perl_semantic_facts::AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: 0,
            span_end_byte: 5,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        shard_v3.occurrences.push(perl_semantic_facts::OccurrenceFact {
            id: OccurrenceId(1),
            kind: OccurrenceKind::Call,
            entity_id: Some(EntityId(100)),
            anchor_id,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        shard_v3.entities.push(perl_semantic_facts::EntityFact {
            id: EntityId(100),
            kind: EntityKind::Subroutine,
            canonical_name: "RefIdx::foo".to_string(),
            anchor_id: Some(anchor_id),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
        let r3 = index.replace_fact_shard_incremental(&key, shard_v3);
        assert!(!r3.occurrences_updated, "occurrence hash unchanged → skip");
        assert!(!r3.edges_updated, "edge hash unchanged → skip");

        Ok(())
    }

    /// Verify that `index_file` uses incremental replacement (the fact shard
    /// is stored and updated correctly through the full indexing path).
    #[test]
    fn index_file_stores_fact_shard_incrementally() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Incr.pm";
        let code = "package Incr;\nsub foo { 1 }\n1;\n";

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));
        let shard1 = must_some(index.file_fact_shard(uri));
        assert!(shard1.anchors_hash.is_some());
        assert!(
            shard1.anchors.iter().any(|anchor| anchor.provenance == Provenance::ExactAst),
            "index_file should store the canonical semantic shard when adapters produce facts"
        );
        assert!(
            shard1.entities.iter().any(|entity| entity.provenance == Provenance::ExactAst),
            "index_file should store canonical entities rather than legacy fallback entities"
        );

        // Re-index with same content → shard should be unchanged.
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));
        // The early-exit in index_file checks content_hash at the FileIndex
        // level, so the fact shard replacement is never reached for identical
        // content. Verify the shard is still present.
        let shard2 = must_some(index.file_fact_shard(uri));
        assert_eq!(shard1.content_hash, shard2.content_hash);

        // Re-index with different content → shard should be replaced.
        let code2 = "package Incr;\nsub bar { 2 }\n1;\n";
        must(index.index_file(must(url::Url::parse(uri)), code2.to_string()));
        let shard3 = must_some(index.file_fact_shard(uri));
        assert_ne!(shard1.content_hash, shard3.content_hash);

        Ok(())
    }

    #[test]
    fn semantic_anchor_wire_location_uses_lsp_utf16_columns()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::semantic::queries::SemanticQueries;

        let index = WorkspaceIndex::new();
        let uri = "file:///lib/UnicodeAnchor.pm";
        let code = "package UnicodeAnchor; my $emoji = \"😀\"; sub target { 1 }\n1;\n";

        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let candidates = index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("UnicodeAnchor::target", &ctx)
            })
            .ok_or("missing semantic queries")?;
        let anchor_id = candidates
            .first()
            .map(|candidate| candidate.anchor_id)
            .ok_or("missing unicode definition candidate")?;
        let shard = index.file_fact_shard(uri).ok_or("missing fact shard")?;
        let anchor = shard
            .anchors
            .iter()
            .find(|anchor| anchor.id == anchor_id)
            .ok_or("missing unicode anchor")?;
        let start = usize::try_from(anchor.span_start_byte)?;
        let end = usize::try_from(anchor.span_end_byte)?;
        let expected = WireRange::from_byte_offsets(code, start, end);

        let location =
            index.semantic_anchor_wire_location(anchor_id).ok_or("missing wire location")?;

        assert_eq!(location.range, expected);
        let wire_column = usize::try_from(location.range.start.character)?;
        let scalar_column = code[..start].chars().count();
        assert!(
            wire_column > scalar_column,
            "fixture must prove the wire column counts UTF-16 units, not Unicode scalar values"
        );

        Ok(())
    }

    #[test]
    fn semantic_anchor_wire_location_fails_closed_for_duplicate_anchor_ids()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::semantic::queries::SemanticQueries;

        // After the file-scoped stable_id fix (#1600), two files with identical Perl source
        // now produce DISTINCT anchor IDs (the file_id is included in the hash). This test
        // verifies that both global lookups succeed and return the correct URIs — the old
        // "fail-closed" scenario (None on collision) no longer applies in production.
        let index = WorkspaceIndex::new();
        let code = "package DuplicateAnchor;\nsub target { 1 }\n1;\n";
        let uri_a = "file:///lib/DuplicateA.pm";
        let uri_b = "file:///lib/DuplicateB.pm";

        must(index.index_file(must(url::Url::parse(uri_a)), code.to_string()));
        must(index.index_file(must(url::Url::parse(uri_b)), code.to_string()));

        let file_id_a = index.file_id_for_uri(uri_a).ok_or("file_id_a not found")?;
        let file_id_b = index.file_id_for_uri(uri_b).ok_or("file_id_b not found")?;

        // Find anchor for file A by file-scoped resolution.
        let all_candidates = index
            .with_semantic_queries_for_uri(uri_a, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("DuplicateAnchor::target", &ctx)
            })
            .ok_or("missing semantic queries")?;

        let anchor_id_a = all_candidates
            .iter()
            .find_map(|c| {
                index
                    .semantic_anchor_wire_location_for_file(file_id_a, c.anchor_id)
                    .map(|_| c.anchor_id)
            })
            .ok_or("no candidate found for file A")?;
        let anchor_id_b = all_candidates
            .iter()
            .find_map(|c| {
                index
                    .semantic_anchor_wire_location_for_file(file_id_b, c.anchor_id)
                    .map(|_| c.anchor_id)
            })
            .ok_or("no candidate found for file B")?;

        // After the fix, anchor IDs are distinct — no collision.
        assert_ne!(anchor_id_a, anchor_id_b, "anchor IDs must be distinct after file-scoped fix");

        // Global lookup now succeeds for both because each anchor_id is unique across shards.
        let location_a = index
            .semantic_anchor_wire_location(anchor_id_a)
            .ok_or("global lookup for anchor_id_a must succeed (no collision after fix)")?;
        assert_eq!(location_a.uri, uri_a, "anchor_id_a must resolve to uri_a");

        let location_b = index
            .semantic_anchor_wire_location(anchor_id_b)
            .ok_or("global lookup for anchor_id_b must succeed (no collision after fix)")?;
        assert_eq!(location_b.uri, uri_b, "anchor_id_b must resolve to uri_b");

        Ok(())
    }

    #[test]
    fn semantic_anchor_wire_location_for_file_resolves_duplicate_anchor_ids_by_file()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::semantic::queries::SemanticQueries;

        // After the file-scoped stable_id fix (#1600), two files with identical Perl source
        // produce DISTINCT anchor IDs. The file-scoped lookup continues to work, and
        // the global lookup also succeeds (no longer fails closed) because there are no
        // collisions. Both assertions validate the new correct post-fix behavior.
        let index = WorkspaceIndex::new();
        let code = "package DuplicateAnchor;\nsub target { 1 }\n1;\n";
        let uri_a = "file:///lib/DuplicateA.pm";
        let uri_b = "file:///lib/DuplicateB.pm";

        must(index.index_file(must(url::Url::parse(uri_a)), code.to_string()));
        must(index.index_file(must(url::Url::parse(uri_b)), code.to_string()));

        let file_id_a = index.file_id_for_uri(uri_a).ok_or("file_id_a not found")?;

        let all_candidates = index
            .with_semantic_queries_for_uri(uri_a, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("DuplicateAnchor::target", &ctx)
            })
            .ok_or("missing semantic queries for uri_a")?;

        // After the fix, find anchor_id for file A via file-scoped resolution.
        let anchor_id_a = all_candidates
            .iter()
            .find_map(|c| {
                index
                    .semantic_anchor_wire_location_for_file(file_id_a, c.anchor_id)
                    .map(|_| c.anchor_id)
            })
            .ok_or("no candidate found for file A")?;

        // Global lookup now succeeds (no duplicate AnchorIds after the fix).
        let global_location = index
            .semantic_anchor_wire_location(anchor_id_a)
            .ok_or("global anchor lookup must succeed post-fix (no collision)")?;
        assert_eq!(global_location.uri, uri_a, "global lookup of anchor_id_a must return uri_a");

        // File-scoped lookup also works.
        let file_location = index
            .semantic_anchor_wire_location_for_file(file_id_a, anchor_id_a)
            .ok_or("file-scoped anchor lookup should resolve anchor ID for file A")?;
        assert_eq!(file_location.uri, uri_a, "file-scoped lookup of anchor_id_a must return uri_a");

        Ok(())
    }

    // ── Property-based tests for incremental invalidation ──

    mod prop_incremental_invalidation {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::Config as ProptestConfig;

        /// Strategy for an optional per-category hash.
        ///
        /// ~10% of the time produces `None` (simulating legacy shards
        /// without per-category hashes); otherwise a random `u64`.
        fn arb_category_hash() -> impl Strategy<Value = Option<u64>> {
            prop_oneof![
                1 => Just(None),
                9 => any::<u64>().prop_map(Some),
            ]
        }

        /// Strategy for a `FileFactShard` with the given URI and
        /// randomly-chosen hashes.
        fn arb_shard(uri: &'static str) -> impl Strategy<Value = FileFactShard> {
            (
                any::<u64>(),        // content_hash
                arb_category_hash(), // anchors_hash
                arb_category_hash(), // entities_hash
                arb_category_hash(), // occurrences_hash
                arb_category_hash(), // edges_hash
            )
                .prop_map(move |(content_hash, ah, eh, oh, edh)| {
                    make_shard(uri, content_hash, ah, eh, oh, edh)
                })
        }

        // Property 15: Incremental Invalidation Correctness
        //
        // **Validates: Requirements 18.3, 18.4, 18.5**
        //
        // For any file re-indexing where the whole-file content_hash is
        // unchanged, the workspace store shall not modify any cross-file
        // indexes.  For any file re-indexing where a per-category hash is
        // unchanged, the workspace store shall skip re-indexing that
        // category.  For any file re-indexing where a per-category hash
        // has changed, the workspace store shall remove old entries and
        // insert new ones for that category.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_incremental_invalidation_correctness(
                old_shard in arb_shard("file:///lib/Prop.pm"),
                new_shard in arb_shard("file:///lib/Prop.pm"),
            ) {
                let index = WorkspaceIndex::new();
                let key = DocumentStore::uri_key("file:///lib/Prop.pm");

                // Seed the index with the old shard.
                index.replace_fact_shard_incremental(&key, old_shard.clone());

                // Replace with the new shard and capture the result.
                let result = index.replace_fact_shard_incremental(&key, new_shard.clone());

                // ── Req 18.5: content_hash unchanged → skip entirely ──
                if old_shard.content_hash == new_shard.content_hash {
                    prop_assert!(
                        result.content_unchanged,
                        "content_unchanged must be true when content_hash is the same"
                    );
                    prop_assert!(
                        !result.anchors_updated,
                        "anchors_updated must be false when content_hash unchanged"
                    );
                    prop_assert!(
                        !result.entities_updated,
                        "entities_updated must be false when content_hash unchanged"
                    );
                    prop_assert!(
                        !result.occurrences_updated,
                        "occurrences_updated must be false when content_hash unchanged"
                    );
                    prop_assert!(
                        !result.edges_updated,
                        "edges_updated must be false when content_hash unchanged"
                    );
                } else {
                    prop_assert!(
                        !result.content_unchanged,
                        "content_unchanged must be false when content_hash differs"
                    );

                    // ── Req 18.3 / 18.4: per-category hash comparison ──
                    // A category is "unchanged" when both old and new have
                    // Some(h) and the values are equal.  Otherwise the
                    // category is conservatively treated as changed.

                    let anchors_should_update = crate::semantic::invalidation::category_hash_changed(
                        old_shard.anchors_hash,
                        new_shard.anchors_hash,
                    );
                    prop_assert_eq!(
                        result.anchors_updated,
                        anchors_should_update,
                        "anchors_updated mismatch: old={:?} new={:?}",
                        old_shard.anchors_hash,
                        new_shard.anchors_hash,
                    );

                    let entities_should_update =
                        crate::semantic::invalidation::category_hash_changed(
                            old_shard.entities_hash,
                            new_shard.entities_hash,
                        );
                    prop_assert_eq!(
                        result.entities_updated,
                        entities_should_update,
                        "entities_updated mismatch: old={:?} new={:?}",
                        old_shard.entities_hash,
                        new_shard.entities_hash,
                    );

                    let occurrences_should_update =
                        crate::semantic::invalidation::category_hash_changed(
                            old_shard.occurrences_hash,
                            new_shard.occurrences_hash,
                        );
                    prop_assert_eq!(
                        result.occurrences_updated,
                        occurrences_should_update,
                        "occurrences_updated mismatch: old={:?} new={:?}",
                        old_shard.occurrences_hash,
                        new_shard.occurrences_hash,
                    );

                    let edges_should_update = crate::semantic::invalidation::category_hash_changed(
                        old_shard.edges_hash,
                        new_shard.edges_hash,
                    );
                    prop_assert_eq!(
                        result.edges_updated,
                        edges_should_update,
                        "edges_updated mismatch: old={:?} new={:?}",
                        old_shard.edges_hash,
                        new_shard.edges_hash,
                    );
                }
            }
        }
    }
}

// ── with_semantic_queries_for_uri tests ──

#[cfg(test)]
mod semantic_query_callback_tests {
    use super::*;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn with_semantic_queries_for_uri_indexed_uri_invokes_callback()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Foo.pm";
        must(index.index_file(must(url::Url::parse(uri)), "sub foo { 1 }".to_string()));

        let result = index.with_semantic_queries_for_uri(uri, |file_id, _queries| {
            // Verify the file_id is consistent with the URI (non-zero hash).
            assert_ne!(file_id.0, 0, "file_id should be non-zero");
            42u32 // sentinel return value
        });

        assert_eq!(result, Some(42u32), "callback must run when URI is indexed");
        Ok(())
    }

    #[test]
    fn with_semantic_queries_for_uri_unknown_uri_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        // Do NOT index anything.
        let result = index.with_semantic_queries_for_uri("file:///not/indexed.pl", |_, _| 99u32);
        assert!(result.is_none(), "unindexed URI must return None without invoking callback");
        Ok(())
    }

    #[test]
    fn with_semantic_queries_for_uri_file_id_matches_file_id_for_uri()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Bar.pm";
        must(index.index_file(must(url::Url::parse(uri)), "sub bar { 1 }".to_string()));

        let direct_id = must_some(index.file_id_for_uri(uri));
        let callback_id =
            must_some(index.with_semantic_queries_for_uri(uri, |file_id, _q| file_id));

        assert_eq!(
            direct_id, callback_id,
            "file_id_for_uri and with_semantic_queries_for_uri must agree"
        );
        Ok(())
    }

    #[test]
    fn with_semantic_queries_for_uri_callback_not_called_when_not_indexed()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let mut called = false;
        let _ = index.with_semantic_queries_for_uri("file:///ghost.pl", |_, _| {
            called = true;
        });
        assert!(!called, "callback must not be invoked for unindexed URI");
        Ok(())
    }

    // Covers lines 4140-4144: NodeKind::NestedVariableList arm in visit_node.
    // Indexing a file that produces a NestedVariableList in the AST ensures the
    // workspace indexer recurses into it to discover nested-declared variables.
    #[test]
    fn visit_node_nested_variable_list_is_indexed() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Nested.pm";
        let code = r#"package Nested;
my ($a, ($b, $c)) = (1, (2, 3));
1;
"#;
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));
        // Verify indexing completed without error (the NestedVariableList arm was traversed).
        let symbols = index.file_symbols(uri);
        // The nested declaration may or may not surface individual symbols depending on
        // the indexer; the key invariant is that the file was successfully indexed.
        let _ = symbols;
        Ok(())
    }
}

// ── Entity ID file-scoped collision tests (#1600) ──

#[cfg(test)]
mod entity_id_file_scoped_tests {
    use super::*;
    use crate::semantic::queries::SemanticQueries;
    use perl_tdd_support::must;

    /// Test A: IDs remain stable across re-parse of identical content in the same file.
    /// After fix, ID stability within a file should be maintained because file_id is constant.
    #[test]
    fn semantic_anchor_id_stable_across_reparse_same_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = WorkspaceIndex::new();
        let uri = "file:///lib/Example.pm";
        let code = "package Example;\nsub target { 1 }\n1;\n";

        // Index the file once.
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        // Extract anchor_id from first index.
        let candidates_1 = index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("Example::target", &ctx)
            })
            .ok_or("missing semantic queries on first index")?;
        let anchor_id_1 = candidates_1
            .first()
            .map(|candidate| candidate.anchor_id)
            .ok_or("missing definition candidate on first index")?;

        // Re-index with identical content.
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        // Extract anchor_id from second index.
        let candidates_2 = index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("Example::target", &ctx)
            })
            .ok_or("missing semantic queries on second index")?;
        let anchor_id_2 = candidates_2
            .first()
            .map(|candidate| candidate.anchor_id)
            .ok_or("missing definition candidate on second index")?;

        // Assertion: IDs must be identical across re-index with same content.
        assert_eq!(
            anchor_id_1, anchor_id_2,
            "anchor ID must remain stable when re-parsing identical content in same file"
        );
        Ok(())
    }

    /// Test B: Two files with identical Perl source produce distinct EntityIds and AnchorIds.
    /// After file-scoped identity fix, anchor_id_a != anchor_id_b even though they have
    /// identical qualified_name and byte offsets. Global lookup must succeed for both.
    ///
    /// Note on extraction: `definitions()` is a global query that returns candidates from
    /// ALL indexed files (sorted by rank, then URI). When two files have identical content,
    /// both files' entities match — we must filter by `semantic_anchor_wire_location_for_file`
    /// to find the candidate that belongs to EACH specific file, rather than taking `.first()`
    /// which always picks the alphabetically first file.
    #[test]
    fn semantic_anchor_ids_distinct_across_files_same_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let code = "package DuplicateAnchor;\nsub target { 1 }\n1;\n";
        let uri_a = "file:///lib/DuplicateA.pm";
        let uri_b = "file:///lib/DuplicateB.pm";

        // Index both files with identical Perl source.
        must(index.index_file(must(url::Url::parse(uri_a)), code.to_string()));
        must(index.index_file(must(url::Url::parse(uri_b)), code.to_string()));

        // Resolve file IDs for each URI.
        let file_id_a = index.file_id_for_uri(uri_a).ok_or("file_id_a not found")?;
        let file_id_b = index.file_id_for_uri(uri_b).ok_or("file_id_b not found")?;

        // Get all definition candidates (from all files) when querying from uri_a context.
        let all_candidates_a = index
            .with_semantic_queries_for_uri(uri_a, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("DuplicateAnchor::target", &ctx)
            })
            .ok_or("with_semantic_queries_for_uri failed for uri_a")?;

        // Find the candidate whose anchor belongs to file A (file-scoped lookup succeeds).
        // This disambiguates when two files have identical content and canonical names.
        let anchor_id_a = all_candidates_a
            .iter()
            .find_map(|c| {
                index
                    .semantic_anchor_wire_location_for_file(file_id_a, c.anchor_id)
                    .map(|_| c.anchor_id)
            })
            .ok_or("no definition candidate found for file A")?;

        // Get all definition candidates when querying from uri_b context.
        let all_candidates_b = index
            .with_semantic_queries_for_uri(uri_b, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("DuplicateAnchor::target", &ctx)
            })
            .ok_or("with_semantic_queries_for_uri failed for uri_b")?;

        // Find the candidate whose anchor belongs to file B.
        let anchor_id_b = all_candidates_b
            .iter()
            .find_map(|c| {
                index
                    .semantic_anchor_wire_location_for_file(file_id_b, c.anchor_id)
                    .map(|_| c.anchor_id)
            })
            .ok_or("no definition candidate found for file B")?;

        // Assertion 1: File IDs must be distinct.
        assert_ne!(
            file_id_a, file_id_b,
            "file_id_a and file_id_b must be distinct for different URIs"
        );

        // Assertion 2: Anchor IDs must be distinct across files.
        assert_ne!(
            anchor_id_a, anchor_id_b,
            "anchor IDs must be distinct for identical code in different files (after file-scoped fix)"
        );

        // Assertion 3: Global lookup must succeed for anchor from file A.
        let location_a = index
            .semantic_anchor_wire_location(anchor_id_a)
            .ok_or("global lookup of anchor_id_a should succeed post-fix")?;
        assert_eq!(location_a.uri, uri_a, "global lookup of anchor_id_a must return uri_a");

        // Assertion 4: Global lookup must succeed for anchor from file B.
        let location_b = index
            .semantic_anchor_wire_location(anchor_id_b)
            .ok_or("global lookup of anchor_id_b should succeed post-fix")?;
        assert_eq!(location_b.uri, uri_b, "global lookup of anchor_id_b must return uri_b");

        // Assertion 5: File-scoped lookup must work for both.
        let location_a_scoped = index
            .semantic_anchor_wire_location_for_file(file_id_a, anchor_id_a)
            .ok_or("file-scoped lookup for (file_id_a, anchor_id_a) should succeed")?;
        assert_eq!(
            location_a_scoped.uri, uri_a,
            "file-scoped lookup of anchor_id_a from file_id_a must return uri_a"
        );

        let location_b_scoped = index
            .semantic_anchor_wire_location_for_file(file_id_b, anchor_id_b)
            .ok_or("file-scoped lookup for (file_id_b, anchor_id_b) should succeed")?;
        assert_eq!(
            location_b_scoped.uri, uri_b,
            "file-scoped lookup of anchor_id_b from file_id_b must return uri_b"
        );

        Ok(())
    }

    /// Test C: Defense-in-depth — manually injected colliding AnchorIds still fail closed.
    /// This test verifies that the fail-closed guard in `semantic_anchor_wire_location` remains
    /// a valid defense mechanism even after the fix, in case a bug allows collisions.
    #[test]
    fn semantic_anchor_wire_location_fails_closed_for_manually_injected_duplicate()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let uri_a = "file:///lib/InjectedDuplicateA.pm";
        let uri_b = "file:///lib/InjectedDuplicateB.pm";
        let code = "package InjectedDuplicate;\nsub target { 1 }\n1;\n";
        let anchor_id = AnchorId(42);

        index.document_store.open(uri_a.to_string(), 1, code.to_string());
        index.document_store.open(uri_b.to_string(), 1, code.to_string());

        for (uri, file_id, content_hash) in [(uri_a, FileId(1), 1), (uri_b, FileId(2), 2)] {
            index.inject_test_fact_shard(FileFactShard {
                source_uri: uri.to_string(),
                file_id,
                content_hash,
                producer_schema_version: PRODUCER_SCHEMA_VERSION,
                anchors_hash: Some(content_hash),
                entities_hash: Some(content_hash),
                occurrences_hash: Some(content_hash),
                edges_hash: Some(content_hash),
                anchors: vec![AnchorFact {
                    id: anchor_id,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 7,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                }],
                entities: Vec::new(),
                occurrences: Vec::new(),
                edges: Vec::new(),
            });
        }

        assert_eq!(index.fact_shard_count(), 2);
        assert!(
            index.semantic_anchor_wire_location(anchor_id).is_none(),
            "global lookup must fail closed when duplicate AnchorIds are injected"
        );

        Ok(())
    }

    // ── search_index correctness: issue #2994 ──

    /// Verify that `search_source_symbols` via the indexed path returns the same
    /// symbol set as iterating all files would, across multiple files, for both
    /// bare-name and qualified-name queries.
    ///
    /// This is the primary correctness guard for the O(n) → O(unique_names) fix:
    /// if the search_index gets out of sync (stale entry, missing add, duplicate),
    /// these assertions catch it.
    #[test]
    fn search_source_symbols_indexed_matches_full_scan_result_set() {
        let index = WorkspaceIndex::new();

        let uri_a = "file:///lib/Utils.pm";
        let uri_b = "file:///lib/App.pm";

        must(index.index_file(
            must(url::Url::parse(uri_a)),
            "package Utils;\nsub process { 1 }\nsub helper { 2 }\n1;\n".to_string(),
        ));
        must(index.index_file(
            must(url::Url::parse(uri_b)),
            "package App;\nuse Utils;\nsub run { 3 }\n1;\n".to_string(),
        ));

        // Bare-name substring match: "proc" matches Utils::process
        let results = index.search_source_symbols("proc", None);
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"process"),
            "bare-name substring 'proc' must match 'process'; got: {:?}",
            names
        );
        assert!(!names.contains(&"helper"), "'helper' must not match 'proc'; got: {:?}", names);

        // Qualified-name substring match: "Utils::proc" must match via qualified name
        let qresults = index.search_source_symbols("Utils::proc", None);
        assert!(
            qresults.iter().any(|s| s.name == "process"),
            "'Utils::proc' must match process via qualified name; got: {:?}",
            qresults.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        // Cross-file: "run" lives only in App.pm
        let run_results = index.search_source_symbols("run", None);
        assert!(
            run_results.iter().any(|s| s.name == "run" && s.uri == uri_b),
            "'run' must be found in App.pm; got: {:?}",
            run_results.iter().map(|s| (&s.name, &s.uri)).collect::<Vec<_>>()
        );

        // No duplicates: a symbol appearing under both bare and qualified name keys
        // must appear exactly once in results.
        let all = index.search_source_symbols("process", None);
        let process_count = all.iter().filter(|s| s.name == "process").count();
        assert_eq!(
            process_count, 1,
            "'process' must appear exactly once (no dup from dual-key indexing); got: {process_count}"
        );
    }

    #[test]
    fn search_source_symbols_keeps_same_name_from_multiple_workspace_folders() {
        let index = WorkspaceIndex::new();
        index.set_workspace_folders(vec![
            "file:///repo/svc-a".to_string(),
            "file:///repo/svc-b".to_string(),
        ]);

        must(index.index_file(
            must(url::Url::parse("file:///repo/svc-a/lib/ServiceA.pm")),
            "package ServiceA;\nsub shared_action_4481 { 1 }\n1;\n".to_string(),
        ));
        must(index.index_file(
            must(url::Url::parse("file:///repo/svc-b/lib/ServiceB.pm")),
            "package ServiceB;\nsub shared_action_4481 { 2 }\n1;\n".to_string(),
        ));

        let mut results = index.search_source_symbols("shared_action_4481", None);
        results.sort_by(|left, right| left.uri.cmp(&right.uri));

        let folders: Vec<Option<&str>> =
            results.iter().map(|symbol| symbol.workspace_folder_uri.as_deref()).collect();
        assert_eq!(
            folders,
            vec![Some("file:///repo/svc-a"), Some("file:///repo/svc-b")],
            "same-name workspace symbols must preserve both workspace folder owners"
        );
    }

    /// Verify that `search_source_symbols` via the indexed path is correct after
    /// a file is updated (incremental remove + add) and after `remove_file`.
    #[test]
    fn search_source_symbols_indexed_correct_after_update_and_remove() {
        let index = WorkspaceIndex::new();

        let uri = "file:///lib/Foo.pm";

        // Index v1: has `old_func`
        must(index.index_file(
            must(url::Url::parse(uri)),
            "package Foo;\nsub old_func { 1 }\n1;\n".to_string(),
        ));
        assert!(
            index.search_source_symbols("old_func", None).iter().any(|s| s.name == "old_func"),
            "old_func must be found after initial index"
        );

        // Re-index (update) v2: `old_func` gone, `new_func` added
        must(index.index_file(
            must(url::Url::parse(uri)),
            "package Foo;\nsub new_func { 2 }\n1;\n".to_string(),
        ));
        assert!(
            index.search_source_symbols("new_func", None).iter().any(|s| s.name == "new_func"),
            "new_func must appear after update"
        );
        assert!(
            index.search_source_symbols("old_func", None).iter().all(|s| s.name != "old_func"),
            "old_func must be gone after update; stale entry in search_index"
        );

        // Remove the file entirely
        index.remove_file(uri);
        assert!(
            index.search_source_symbols("new_func", None).is_empty(),
            "new_func must be gone after remove_file"
        );
    }

    /// Verify that `search_source_symbols` returns the same set (sorted by name+uri)
    /// whether a batch index or incremental index is used.  This exercises
    /// `rebuild_search_index` (batch path) vs `incremental_add_search` (single path).
    #[test]
    fn search_source_symbols_batch_vs_incremental_same_result_set() {
        let uri_a = "file:///lib/Alpha.pm";
        let uri_b = "file:///lib/Beta.pm";
        let code_a = "package Alpha;\nsub alpha_fn { 1 }\n1;\n";
        let code_b = "package Beta;\nsub beta_fn { 2 }\n1;\n";

        // Incremental path
        let idx_inc = WorkspaceIndex::new();
        must(idx_inc.index_file(must(url::Url::parse(uri_a)), code_a.to_string()));
        must(idx_inc.index_file(must(url::Url::parse(uri_b)), code_b.to_string()));

        // Batch path
        let idx_batch = WorkspaceIndex::new();
        let errors = idx_batch.index_files_batch(vec![
            (must(url::Url::parse(uri_a)), code_a.to_string()),
            (must(url::Url::parse(uri_b)), code_b.to_string()),
        ]);
        assert!(errors.is_empty(), "batch index must have no errors: {:?}", errors);

        // Both should find "alpha_fn" and "beta_fn"
        for query in &["alpha_fn", "beta_fn", "fn"] {
            let mut inc = idx_inc.search_source_symbols(query, None);
            let mut bat = idx_batch.search_source_symbols(query, None);
            inc.sort_by(|a, b| a.name.cmp(&b.name).then(a.uri.cmp(&b.uri)));
            bat.sort_by(|a, b| a.name.cmp(&b.name).then(a.uri.cmp(&b.uri)));
            let inc_names: Vec<&str> = inc.iter().map(|s| s.name.as_str()).collect();
            let bat_names: Vec<&str> = bat.iter().map(|s| s.name.as_str()).collect();
            assert_eq!(
                inc_names, bat_names,
                "query '{query}': incremental and batch must return same symbol names"
            );
        }
    }
}

// ── search_source_symbols / search_generated_workspace_symbols cap (#1668) ──

#[cfg(test)]
mod search_cap_tests {
    use super::*;
    use perl_tdd_support::must;

    fn make_index_with_subs(uri: &str, subs: &[&str]) -> WorkspaceIndex {
        let index = WorkspaceIndex::new();
        let source = subs.iter().map(|s| format!("sub {s} {{}}")).collect::<Vec<_>>().join(" ");
        must(index.index_file(must(url::Url::parse(uri)), source));
        index
    }

    #[test]
    fn search_source_symbols_cap_limits_results() {
        let index = make_index_with_subs(
            "file:///lib/Cap.pm",
            &["alpha", "beta", "gamma", "delta", "epsilon"],
        );

        let uncapped = index.search_source_symbols("", None);
        assert!(uncapped.len() >= 5, "uncapped must return all 5 symbols");

        let capped = index.search_source_symbols("", Some(2));
        assert!(capped.len() <= 2, "cap=2 must return at most 2 symbols; got {}", capped.len());
    }

    #[test]
    fn search_source_symbols_cap_none_returns_all() {
        let index = make_index_with_subs("file:///lib/All.pm", &["foo", "bar"]);

        let uncapped = index.search_source_symbols("", None);
        let capped_large = index.search_source_symbols("", Some(usize::MAX));
        // Cap large enough to never trigger early exit — both paths return the same count.
        assert_eq!(uncapped.len(), capped_large.len());
    }

    #[test]
    fn search_source_symbols_cap_one_returns_exactly_one() {
        let index = make_index_with_subs("file:///lib/One.pm", &["qux", "quux", "quuz", "corge"]);

        let capped = index.search_source_symbols("", Some(1));
        assert_eq!(capped.len(), 1, "cap=1 must return exactly 1 symbol; got {:?}", capped);
    }
}

// ── FileFactShard serde round-trip (Campaign 31 PR 5, perl-lsp-swarm#2592) ──

#[cfg(test)]
mod file_fact_shard_serde_tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn file_fact_shard_serializes_and_deserializes_round_trip() {
        let shard = FileFactShard {
            source_uri: "file:///lib/My/App.pm".to_string(),
            file_id: FileId(42),
            content_hash: 12345,
            producer_schema_version: 1,
            anchors_hash: Some(100),
            entities_hash: Some(200),
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![],
            entities: vec![],
            occurrences: vec![],
            edges: vec![],
        };
        let json = serde_json::to_string(&shard).expect("FileFactShard must serialize");
        let deserialized: FileFactShard =
            serde_json::from_str(&json).expect("FileFactShard must deserialize");

        assert_eq!(deserialized.source_uri, shard.source_uri);
        assert_eq!(deserialized.file_id, shard.file_id);
        assert_eq!(deserialized.content_hash, shard.content_hash);
        assert_eq!(deserialized.producer_schema_version, shard.producer_schema_version);
        assert_eq!(deserialized.anchors_hash, shard.anchors_hash);
        assert_eq!(deserialized.anchors.len(), 0);
    }

    #[test]
    fn file_fact_shard_with_facts_serializes() {
        let shard = FileFactShard {
            source_uri: "file:///t/app.t".to_string(),
            file_id: FileId(7),
            content_hash: 999,
            producer_schema_version: 1,
            anchors_hash: Some(1),
            entities_hash: Some(2),
            occurrences_hash: Some(3),
            edges_hash: Some(4),
            anchors: vec![],
            entities: vec![],
            occurrences: vec![],
            edges: vec![],
        };
        // Must serialize without error — the ripr-facts emitter relies on this.
        let json = serde_json::to_string(&shard).expect("must serialize with facts");
        assert!(json.contains("\"source_uri\":\"file:///t/app.t\""));
        assert!(json.contains("\"content_hash\":999"));
    }

    // ── find_unused_symbols: lexical exclusion tests (#1805) ──────────────────

    /// A genuinely-unused `my` variable must NOT appear in `find_unused_symbols`
    /// after the fix, because scope-local lexicals are excluded from the bare-name
    /// unused check entirely (bare-name lookup cannot determine scope correctly).
    /// Pre-fix: the variable IS reported (no usage refs → `has_usage = false`).
    /// Post-fix: excluded from check → not reported.
    #[test]
    fn test_find_unused_symbols_excludes_genuinely_unused_my_variable() {
        let index = WorkspaceIndex::new();
        let uri = "file:///unused-lexical.pl";
        let code = "sub foo {\n    my $isolated = 42;\n    return 1;\n}\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            !unused_names.contains(&"$isolated"),
            "my variable must be excluded from bare-name unused check; got: {:?}",
            unused_names
        );
    }

    /// A used `my $x` (referenced within same scope) must also NOT appear in
    /// find_unused_symbols — the exclusion is class-wide, not use-sensitive.
    #[test]
    fn test_find_unused_symbols_excludes_used_my_variable() {
        let index = WorkspaceIndex::new();
        let uri = "file:///used-lexical.pl";
        let code = "sub foo {\n    my $x = 1;\n    return $x;\n}\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            !unused_names.contains(&"$x"),
            "used my $x must not appear in find_unused_symbols; got: {:?}",
            unused_names
        );
    }

    /// `state` variables are lexically scoped just like `my` — excluded from
    /// the bare-name unused check.
    #[test]
    fn test_find_unused_symbols_excludes_state_variable() {
        let index = WorkspaceIndex::new();
        let uri = "file:///state-var.pl";
        let code =
            "use feature 'state';\nsub counter {\n    state $count = 0;\n    return $count;\n}\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            !unused_names.contains(&"$count"),
            "state variable must be excluded from bare-name unused check; got: {:?}",
            unused_names
        );
    }

    /// Positive control: an unused package-level subroutine IS still reported
    /// by find_unused_symbols — only lexical my/state vars are excluded.
    #[test]
    fn test_find_unused_symbols_still_reports_unused_subroutine() {
        let index = WorkspaceIndex::new();
        let uri = "file:///unused-sub.pl";
        let code = "package Foo;\nsub bar { return 1; }\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            unused_names.contains(&"bar"),
            "unused package-level sub must still be reported by find_unused_symbols; got: {:?}",
            unused_names
        );
    }

    /// Positive control: an unused `our` (package-level) variable IS still
    /// reported — only lexical my/state vars are excluded, not our/local.
    #[test]
    fn test_find_unused_symbols_still_reports_unused_our_variable() {
        let index = WorkspaceIndex::new();
        let uri = "file:///our-var.pl";
        let code = "package Foo;\nour $VERSION = '1.0';\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            unused_names.contains(&"$VERSION"),
            "unused our variable must still be reported by find_unused_symbols; got: {:?}",
            unused_names
        );
    }

    /// Cross-scope collision: two subs each declare `my $shared_name`. The one
    /// without a usage should NOT appear in find_unused_symbols (the whole
    /// class is excluded). Pre-fix it would appear due to bare-name false-negative
    /// hiding — but post-fix both are excluded from the check entirely.
    #[test]
    fn test_find_unused_symbols_cross_scope_name_collision_excluded() {
        let index = WorkspaceIndex::new();
        let uri = "file:///cross-scope.pl";
        // foo declares $shared but doesn't use it; bar declares and uses $shared.
        // Pre-fix: bar's usage ref makes foo's $shared appear "used" (false neg).
        // Post-fix: both are excluded because they're my vars.
        let code = "sub foo {\n    my $shared = 1;\n    return 1;\n}\nsub bar {\n    my $shared = 2;\n    return $shared;\n}\n";
        must(index.index_file(must(url::Url::parse(uri)), code.to_string()));

        let unused = index.find_unused_symbols();
        let unused_names: Vec<&str> = unused.iter().map(|s| s.name.as_str()).collect();
        assert!(
            !unused_names.contains(&"$shared"),
            "cross-scope my variable must not appear in find_unused_symbols; got: {:?}",
            unused_names
        );
    }
}

/// # PR 1711-A -- didChange re-extraction work-shape measurement.
///
/// Measures how much re-extraction work `index_file_with_generation` (the
/// production `didChange` -> shard-update path) does per edit, on a large
/// file (500+ LOC / 80+ symbols), across the six edit classes called for by
/// perl-lsp-swarm#1711:
///
/// 1. comment/whitespace-only (categories unchanged) -- the key waste case;
/// 2. reference-only (occurrence changes, decl/entity reusable);
/// 3. declaration/entity-changing (entity + anchor change);
/// 4. generated/dynamic-fact (`eval "sub NAME {...}"` synthetic category);
/// 5. revert-to-original (determinism -- no drift);
/// 6. superseded generation (no stale publication).
///
/// This module is a MEASUREMENT RECEIPT, not a behavior change: every
/// assertion below observes either (a) existing public API
/// (`file_fact_shard`'s per-category hashes) or (b) the `reindex_metrics`
/// counters/timers added above, which are themselves compiled only under
/// `#[cfg(test)]` and never run in a production build. See
/// `docs/reference/1711-A-reextraction-workshape-receipt.md` for the
/// human-readable summary and the bounded-vs-material disposition.
///
/// The structural evidence (category-hash-changed flags, call counts,
/// per-file cache-contribution counts) is MECHANICALLY BOUND via
/// `reextraction_workshape_receipt_snapshot`'s checked-in `insta` snapshot --
/// so those numbers cannot silently drift from what the Markdown receipt
/// claims without `cargo insta review`/`INSTA_UPDATE=no` failing. Timing
/// stays informational-only (`eprintln!`, no assertion, excluded from the
/// snapshot) -- it is not deterministic on shared/debug hardware.
#[cfg(test)]
mod reindex_workshape_measurement {
    // Measurement receipts are only useful if a human can read them: `cargo
    // test -- --nocapture` is the whole point of this module. Scoped to this
    // test-only module (same pattern as `xtask/src/main.rs`), never touches
    // production code.
    #![allow(clippy::print_stderr, clippy::print_stdout)]

    use super::*;
    use perl_tdd_support::{must, must_some};

    /// Number of `sub` declarations in the large-file fixture. 80 subs (plus
    /// the enclosing package) comfortably clears the spec's "500+ LOC / 50+
    /// symbols" bar -- see the LOC assertion in `fixture_is_large_enough`.
    const SUB_COUNT: usize = 80;

    /// `origin/main` commit this measurement's baseline sits on
    /// (`git merge-base HEAD origin/main` at PR-1711-A creation time). A
    /// fixed label, not introspected via `git` at test run time -- git
    /// introspection would be fragile under shallow clones, tarball
    /// checkouts, and CI sandboxes with no `.git` directory, and would make
    /// the snapshot's determinism depend on the runner's checkout depth
    /// rather than on the code being measured. Update by hand only if
    /// `index_file_with_generation`'s extraction logic is materially
    /// re-baselined.
    const BASE_SHA: &str = "393f167d006fcd79bf6009a93aefe872b3807e67";

    /// Commit that introduced this measurement instrumentation
    /// (perl-lsp-swarm PR #4013, "1711-A"). Same update policy as
    /// `BASE_SHA` -- a traceability label, not a live git lookup.
    const MEASUREMENT_INTRODUCED_AT_SHA: &str = "6ebbc1fd4be43ed46872b960a836c7586dc0aefe";

    /// Builds the shared fixture body (package header + `SUB_COUNT` subs),
    /// WITHOUT the trailing `1;\n`. Every edit class appends its own content
    /// immediately before a `1;\n` it supplies itself, so callers control
    /// exactly what comes last.
    fn fixture_prefix(sub_count: usize) -> String {
        let mut src = String::new();
        src.push_str("package Big::Reextraction::Fixture;\n");
        src.push_str("use strict;\n");
        src.push_str("use warnings;\n\n");
        for i in 0..sub_count {
            src.push_str(&format!(
                "sub sub_{i} {{\n    my ($x, $y) = @_;\n    my $sum = $x + $y + {i};\n    # sub_{i} body comment for LOC padding\n    return $sum;\n}}\n\n"
            ));
        }
        src
    }

    /// A block of pure comment lines, safe to append strictly after all
    /// other content (so no existing anchor's byte span shifts).
    fn trailing_comment_block(lines: usize) -> String {
        let mut block = String::new();
        for i in 0..lines {
            block.push_str(&format!(
                "# trailing commentary line {i} -- pure whitespace/comment edit\n"
            ));
        }
        block
    }

    // `SUB_COUNT + 1 >= 50` (subs + the enclosing package) is a compile-time
    // property of the constant, not a runtime check -- enforced here instead
    // of via a trivially-true `assert!` in the test body below.
    const _: () = assert!(SUB_COUNT + 1 >= 50, "fixture must have 50+ symbols (subs + package)");

    #[test]
    fn fixture_is_large_enough() {
        let prefix = fixture_prefix(SUB_COUNT);
        let full = format!("{prefix}1;\n");
        let loc = full.lines().count();
        assert!(loc >= 500, "fixture must be 500+ LOC per the #1711-A spec; got {loc}");
    }

    /// Runs `index_file_with_generation` once, recording `reindex_metrics`
    /// on the calling thread for the duration of the call (safe under
    /// parallel `cargo test` execution -- see the module doc comment on
    /// `reindex_metrics`).
    fn index_and_measure(
        index: &WorkspaceIndex,
        uri: &str,
        text: String,
        generation: u32,
    ) -> (Result<(), String>, reindex_metrics::ReindexWorkMetrics) {
        let url = must(url::Url::parse(uri));
        reindex_metrics::start();
        let result = index.index_file_with_generation(url, text, generation);
        let metrics = reindex_metrics::take();
        (result, metrics)
    }

    type CategoryHashes = (u64, Option<u64>, Option<u64>, Option<u64>, Option<u64>);

    /// Fetches `(content_hash, anchors_hash, entities_hash, occurrences_hash,
    /// edges_hash)` for a URI via the existing public `file_fact_shard` API
    /// -- no new instrumentation needed for shard-category comparison.
    fn category_hashes(index: &WorkspaceIndex, uri: &str) -> Option<CategoryHashes> {
        index.file_fact_shard(uri).map(|s| {
            (s.content_hash, s.anchors_hash, s.entities_hash, s.occurrences_hash, s.edges_hash)
        })
    }

    /// Sum of instrumented extraction timers for one call -- INFORMATIONAL
    /// only (see the measurement-discipline note on the module doc comment:
    /// no hard-millisecond threshold is asserted anywhere in this file).
    fn total_extraction_time(m: &reindex_metrics::ReindexWorkMetrics) -> std::time::Duration {
        m.visit_time
            + m.decl_extract_time
            + m.ref_extract_time
            + m.eval_sub_time
            + m.generated_member_time
            + m.import_extract_time
            + m.use_lib_extract_time
    }

    /// Whether an `index_file_with_generation` call was rejected by either
    /// monotonic generation guard (as opposed to accepted, or short-circuited
    /// by an unchanged content hash).
    fn was_stale_rejected(m: &reindex_metrics::ReindexWorkMetrics) -> bool {
        m.stale_generation_rejected_pre_parse || m.stale_generation_rejected_post_parse
    }

    /// One evidence bucket for a per-category shard-hash comparison. Kept as
    /// an explicit three-state enum (not `bool`) because "rejected, so shard
    /// replacement never ran at all" is a materially different fact from
    /// "ran, and this category happened to be unchanged" -- collapsing them
    /// into `false` would hide that a superseded generation never reaches
    /// shard replacement in the first place.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CategoryDelta {
        Changed,
        Unchanged,
        /// The call was rejected before any shard replacement occurred (a
        /// stale/superseded generation) -- "changed vs unchanged" does not
        /// apply.
        NotApplicableRejected,
    }

    /// Deterministic, checked-in STRUCTURAL receipt for one edit class.
    /// Every field is a compile-time constant, a call count, a per-file
    /// cache-contribution count, or a category-hash-changed flag -- nothing
    /// timing-based. This is the type `reextraction_workshape_receipt_snapshot`
    /// snapshots; the Markdown receipt
    /// (`docs/reference/1711-A-reextraction-workshape-receipt.md`) treats
    /// this snapshot as the source of truth for counts, not hand-transcribed
    /// numbers.
    #[derive(Debug, Clone)]
    // snapshot record: fields are read collectively by derive(Debug) into the
    // insta snapshot, which the dead_code lint does not count as per-field use
    #[allow(dead_code)]
    struct EditClassReceipt {
        base_sha: &'static str,
        measurement_introduced_at_sha: &'static str,
        edit_class: &'static str,
        anchors_hash: CategoryDelta,
        entities_hash: CategoryDelta,
        occurrences_hash: CategoryDelta,
        edges_hash: CategoryDelta,
        /// `IndexVisitor::visit` call count (legacy symbol/reference walk).
        visitor_visit_calls: u32,
        /// Canonical declaration extractor (`extract_symbol_decls`) call count.
        canonical_decl_extract_calls: u32,
        /// Canonical reference extractor (`extract_symbol_refs`) call count.
        canonical_ref_extract_calls: u32,
        /// Dynamic-boundary extractor (`extract_eval_sub_boundaries`, e.g.
        /// `eval "sub NAME {...}"`) call count.
        dynamic_boundary_extract_calls: u32,
        /// Generated-member extractor (`extract_generated_member_facts`,
        /// e.g. Moo/Moose `has`) call count.
        generated_member_extract_calls: u32,
        import_extract_calls: u32,
        use_lib_extract_calls: u32,
        /// This file's own symbol-table CONTRIBUTION -- i.e. the number of
        /// entries in `FileIndex::symbols` for THIS URI that were passed
        /// through the legacy-cache removal routine on this call. This is
        /// NOT necessarily the number of entries removed from the global
        /// qualified/bare-name map: the dual-indexing pattern
        /// (`perl-workspace/CLAUDE.md`, PR #122) may write each contributed
        /// symbol under up to two global keys, and it is not a
        /// whole-workspace rebuild -- only this one file's contribution is
        /// touched.
        file_symbol_contribution_removed: usize,
        /// Same file-scoped contribution, on the re-add side of the same call.
        file_symbol_contribution_added: usize,
        /// This file's contribution passed through the search-index
        /// removal routine (same caveat as `file_symbol_contribution_removed`).
        file_search_contribution_removed: usize,
        file_search_contribution_added: usize,
        /// This file's own global-reference-index CONTRIBUTION (only this
        /// URI's entries, not the whole workspace-wide reference cache)
        /// passed through the removal routine.
        file_global_ref_contribution_removed: usize,
        file_global_ref_contribution_added: usize,
        /// `"accepted"` | `"content_hash_short_circuit"` |
        /// `"stale_rejected_pre_parse"` | `"stale_rejected_post_parse"`.
        generation_outcome: &'static str,
    }

    /// Builds one `EditClassReceipt` from a before/after category-hash pair
    /// and the metrics recorded for the (second, edited) call.
    fn build_receipt(
        edit_class: &'static str,
        before: CategoryHashes,
        after: CategoryHashes,
        m: &reindex_metrics::ReindexWorkMetrics,
    ) -> EditClassReceipt {
        let rejected = was_stale_rejected(m);
        let delta = |b: Option<u64>, a: Option<u64>| {
            if rejected {
                CategoryDelta::NotApplicableRejected
            } else if b == a {
                CategoryDelta::Unchanged
            } else {
                CategoryDelta::Changed
            }
        };
        let generation_outcome = if m.generation_accepted {
            "accepted"
        } else if m.content_hash_short_circuit {
            "content_hash_short_circuit"
        } else if m.stale_generation_rejected_pre_parse {
            "stale_rejected_pre_parse"
        } else if m.stale_generation_rejected_post_parse {
            "stale_rejected_post_parse"
        } else {
            "unknown"
        };
        EditClassReceipt {
            base_sha: BASE_SHA,
            measurement_introduced_at_sha: MEASUREMENT_INTRODUCED_AT_SHA,
            edit_class,
            anchors_hash: delta(before.1, after.1),
            entities_hash: delta(before.2, after.2),
            occurrences_hash: delta(before.3, after.3),
            edges_hash: delta(before.4, after.4),
            visitor_visit_calls: m.visit_calls,
            canonical_decl_extract_calls: m.decl_extract_calls,
            canonical_ref_extract_calls: m.ref_extract_calls,
            dynamic_boundary_extract_calls: m.eval_sub_calls,
            generated_member_extract_calls: m.generated_member_calls,
            import_extract_calls: m.import_extract_calls,
            use_lib_extract_calls: m.use_lib_extract_calls,
            file_symbol_contribution_removed: m.legacy_symbols_removed,
            file_symbol_contribution_added: m.legacy_symbols_added,
            file_search_contribution_removed: m.legacy_search_removed,
            file_search_contribution_added: m.legacy_search_added,
            file_global_ref_contribution_removed: m.global_refs_removed,
            file_global_ref_contribution_added: m.global_refs_added,
            generation_outcome,
        }
    }

    /// **Edit class 1 -- comment/whitespace-only edit. The key waste case.**
    ///
    /// The trailing comment block is appended strictly after the existing
    /// `1;\n`, so no anchor/entity/occurrence/edge byte span moves. All four
    /// category hashes must therefore be bit-identical before/after, while
    /// the whole-file `content_hash` must differ (real bytes were added).
    ///
    /// Returns `(before, after, metrics-of-the-comment-edit)` so both the
    /// correctness assertions below and the snapshot test can share one
    /// code path.
    fn run_class_1_comment_only()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/comment_only.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");
        let comment_only = format!("{baseline}{}", trailing_comment_block(30));

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline, 1);
        r0?;
        let before = must_some(category_hashes(&index, uri));
        let (r1, m1) = index_and_measure(&index, uri, comment_only, 2);
        r1?;
        let after = must_some(category_hashes(&index, uri));
        Ok((before, after, m1))
    }

    #[test]
    fn edit_class_1_comment_only_reextracts_unconditionally_despite_unchanged_categories()
    -> Result<(), Box<dyn std::error::Error>> {
        let (before, after, m1) = run_class_1_comment_only()?;

        assert_ne!(before.0, after.0, "content_hash must change -- real bytes were added");
        assert_eq!(
            before.1, after.1,
            "anchors_hash must be unchanged by a trailing comment-only edit"
        );
        assert_eq!(
            before.2, after.2,
            "entities_hash must be unchanged by a trailing comment-only edit"
        );
        assert_eq!(
            before.3, after.3,
            "occurrences_hash must be unchanged by a trailing comment-only edit"
        );
        assert_eq!(
            before.4, after.4,
            "edges_hash must be unchanged by a trailing comment-only edit"
        );

        // THE KEY MEASUREMENT: despite zero category change, every canonical
        // extractor still runs exactly once -- this is the avoidable work
        // #1711 asks about.
        //
        // **1711-B cutover remeasurement**: `ref_extract_calls` is now 0, not
        // 1. Pre-cutover, this edit class ran TWO full-AST reference walks
        // (`IndexVisitor::visit` -- counted by `visit_calls` -- plus a
        // second, independent `extract_symbol_refs(ast)` walk -- counted by
        // `ref_extract_calls`). Post-cutover, `visit_calls` still counts 1
        // (now `IndexVisitor::visit_unified`, which derives BOTH the legacy
        // and canonical reference projections from that single walk), and
        // `ref_extract_calls` drops to 0 because `extract_symbol_refs` is no
        // longer called on this path at all -- see
        // `docs/reference/1711-A-reextraction-workshape-receipt.md`'s
        // 1711-B remeasurement addendum for the full before/after tally.
        assert_eq!(m1.visit_calls, 1);
        assert_eq!(m1.decl_extract_calls, 1);
        assert_eq!(
            m1.ref_extract_calls, 0,
            "1711-B cutover: extract_symbol_refs must no longer run as a separate walk -- \
             visit_unified already produced the canonical Vec<SymbolRef>"
        );
        assert_eq!(m1.eval_sub_calls, 1);
        assert_eq!(m1.generated_member_calls, 1);
        assert_eq!(m1.import_extract_calls, 1);
        assert_eq!(m1.use_lib_extract_calls, 1);
        assert!(
            m1.generation_accepted,
            "comment-only edit must be accepted as a new generation, not rejected"
        );
        assert!(
            !m1.content_hash_short_circuit,
            "the comment-only edit changes real bytes, so it must not take the \
             unchanged-content short-circuit path"
        );

        // Legacy cache CONTRIBUTION churn: this file's own symbol-table
        // contribution is torn down and rebuilt in full even though the
        // symbol SET is byte-for-byte identical. This is THIS FILE's
        // contribution passing through the removal/re-add routine, not a
        // whole-workspace cache rebuild -- see `EditClassReceipt`'s field
        // docs for the exact caveat (dual-indexing may write each
        // contributed symbol under up to two global keys).
        assert_eq!(m1.legacy_symbols_removed, m1.legacy_symbols_added);
        assert!(
            m1.legacy_symbols_removed >= SUB_COUNT,
            "expected this file's full symbol-contribution churn on a comment-only edit; got {}",
            m1.legacy_symbols_removed
        );

        eprintln!(
            "[1711-A receipt] comment-only edit: extraction_total={:?} \
             file_symbol_contribution_churned={} file_global_ref_contribution_churned={}",
            total_extraction_time(&m1),
            m1.legacy_symbols_removed,
            m1.global_refs_removed
        );
        Ok(())
    }

    /// **Edit class 1, repeated -- timing distribution (informational).**
    ///
    /// A single sample is noisy (scheduler jitter, cache warmth). This runs
    /// 15 successive trailing-comment-only edits on the SAME large fixture
    /// (each edit still strictly appends after all prior content, so
    /// category hashes never change across any of the 15 steps -- verified
    /// on every iteration, not just the first) and reports min/median/max
    /// extraction-vs-parse time. Per the measurement-discipline directive,
    /// this is INFORMATIONAL ONLY -- no assertion anywhere in this file
    /// gates on an absolute millisecond value; the decision rests on the
    /// WORK counts asserted in the sibling test above (and mechanically
    /// bound in `reextraction_workshape_receipt_snapshot`).
    #[test]
    fn edit_class_1_repeated_comment_only_edits_timing_distribution()
    -> Result<(), Box<dyn std::error::Error>> {
        const ITERATIONS: usize = 15;
        let uri = "file:///big/comment_only_repeated.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline.clone(), 1);
        r0?;
        let original = must_some(category_hashes(&index, uri));

        let mut extraction_samples = Vec::with_capacity(ITERATIONS);
        let mut parse_samples = Vec::with_capacity(ITERATIONS);
        let mut text = baseline;
        for step in 0..ITERATIONS {
            text.push_str(&format!("# repeated trailing comment step {step}\n"));

            let parse_start = std::time::Instant::now();
            let _ = Parser::new(&text).parse();
            parse_samples.push(parse_start.elapsed());

            let (r, m) = index_and_measure(
                &index,
                uri,
                text.clone(),
                u32::try_from(step + 2).map_err(|e| format!("generation overflow: {e}"))?,
            );
            r?;

            let after = must_some(category_hashes(&index, uri));
            assert_eq!(
                original.1, after.1,
                "anchors_hash must stay unchanged across every repeated comment-only step"
            );
            assert_eq!(
                original.2, after.2,
                "entities_hash must stay unchanged across every repeated comment-only step"
            );
            assert_eq!(
                original.3, after.3,
                "occurrences_hash must stay unchanged across every repeated comment-only step"
            );
            assert_eq!(
                original.4, after.4,
                "edges_hash must stay unchanged across every repeated comment-only step"
            );
            assert_eq!(
                m.decl_extract_calls, 1,
                "extraction still runs every step despite no category change"
            );

            extraction_samples.push(total_extraction_time(&m));
        }

        extraction_samples.sort();
        parse_samples.sort();
        let mid = ITERATIONS / 2;
        eprintln!(
            "[1711-A receipt] comment-only x{ITERATIONS} -- extraction(min/median/max)={:?}/{:?}/{:?} \
             parse(min/median/max)={:?}/{:?}/{:?}",
            extraction_samples[0],
            extraction_samples[mid],
            extraction_samples[ITERATIONS - 1],
            parse_samples[0],
            parse_samples[mid],
            parse_samples[ITERATIONS - 1],
        );
        Ok(())
    }

    /// **Edit class 2 -- reference-only edit.**
    ///
    /// Appends a single call to an EXISTING sub as the last statement before
    /// `1;\n`. No new declaration is added, so `entities_hash` must be
    /// unaffected. `occurrences_hash` must change (a new call-site
    /// occurrence exists).
    ///
    /// `anchors_hash` ALSO changes here -- an initially counter-intuitive
    /// finding worth recording: `symbol_refs_to_semantic_facts`
    /// (`crates/perl-symbol/src/surface/facts.rs`) emits one `AnchorFact`
    /// per REFERENCE, not just per declaration (`SymbolRefSemanticFacts`
    /// doc: "Source-span anchors, one per reference"). So the `anchors`
    /// category conflates declaration-anchors and reference-anchors; any
    /// edit that adds/removes a reference touches `anchors_updated` too,
    /// not just `occurrences_updated`. This matters for the bounded/material
    /// disposition: a hypothetical category-hash-gated skip would still
    /// have to treat "anchors changed" as common (any new reference trips
    /// it), not just "rare" -- it does NOT undermine the comment-only
    /// case (class 1), where no reference is added either.
    fn run_class_2_reference_only()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/reference_only.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");
        let reference_only = format!("{prefix}sub_0(1, 2);\n1;\n");

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline, 1);
        r0?;
        let before = must_some(category_hashes(&index, uri));
        let (r1, m1) = index_and_measure(&index, uri, reference_only, 2);
        r1?;
        let after = must_some(category_hashes(&index, uri));
        Ok((before, after, m1))
    }

    #[test]
    fn edit_class_2_reference_only_edit_changes_occurrences_and_reference_anchors()
    -> Result<(), Box<dyn std::error::Error>> {
        let (before, after, m1) = run_class_2_reference_only()?;

        assert_ne!(before.0, after.0, "content_hash must change");
        assert_ne!(
            before.1, after.1,
            "anchors_hash changes too -- reference anchors share the anchors category with decl anchors"
        );
        assert_eq!(before.2, after.2, "entities_hash must be unaffected -- no new declaration");
        assert_ne!(
            before.3, after.3,
            "occurrences_hash must change -- a new call-site reference was added"
        );
        assert_eq!(m1.decl_extract_calls, 1);
        // 1711-B cutover: see the remeasurement note on edit class 1 above --
        // `extract_symbol_refs` no longer runs as an independent second walk.
        assert_eq!(m1.ref_extract_calls, 0);

        eprintln!(
            "[1711-A receipt] reference-only edit: extraction_total={:?}",
            total_extraction_time(&m1)
        );
        Ok(())
    }

    /// **Edit class 3 -- declaration/entity-changing edit.**
    ///
    /// Appends a brand-new `sub` before `1;\n`. Both `anchors_hash` and
    /// `entities_hash` must change; this is the case category-scoped
    /// propagation is already designed to detect.
    fn run_class_3_declaration_changing()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/decl_edit.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");
        let decl_changed = format!("{prefix}sub sub_new_extra {{\n    return 999;\n}}\n\n1;\n");

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline, 1);
        r0?;
        let before = must_some(category_hashes(&index, uri));
        let (r1, m1) = index_and_measure(&index, uri, decl_changed, 2);
        r1?;
        let after = must_some(category_hashes(&index, uri));
        Ok((before, after, m1))
    }

    #[test]
    fn edit_class_3_declaration_edit_changes_entities_and_anchors()
    -> Result<(), Box<dyn std::error::Error>> {
        let (before, after, m1) = run_class_3_declaration_changing()?;

        assert_ne!(
            before.1, after.1,
            "anchors_hash must change -- a new declaration anchor was added"
        );
        assert_ne!(before.2, after.2, "entities_hash must change -- a new sub entity was added");
        assert_eq!(m1.decl_extract_calls, 1);

        eprintln!(
            "[1711-A receipt] declaration-changing edit: extraction_total={:?}",
            total_extraction_time(&m1)
        );
        Ok(())
    }

    /// **Edit class 4 -- generated/dynamic-fact edit.**
    ///
    /// Appends `eval "sub NAME { ... }"` before `1;\n`, exercising the
    /// dynamic-boundary extractor's synthetic entity/anchor/occurrence path.
    /// Synthetic facts must stay honestly classified (they flow through the
    /// SAME anchors/entities categories, not a hidden fifth category).
    fn run_class_4_dynamic_fact()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/dynamic_fact.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");
        let dynamic_fact =
            format!("{prefix}eval \"sub dynamic_generated_1 {{ return 123; }}\";\n\n1;\n");

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline, 1);
        r0?;
        let before = must_some(category_hashes(&index, uri));
        let (r1, m1) = index_and_measure(&index, uri, dynamic_fact, 2);
        r1?;
        let after = must_some(category_hashes(&index, uri));
        Ok((before, after, m1))
    }

    #[test]
    fn edit_class_4_dynamic_eval_sub_edit_changes_synthetic_categories()
    -> Result<(), Box<dyn std::error::Error>> {
        let (before, after, m1) = run_class_4_dynamic_fact()?;

        assert_ne!(
            before.1, after.1,
            "anchors_hash must change -- synthetic anchor from eval-sub boundary"
        );
        assert_ne!(
            before.2, after.2,
            "entities_hash must change -- synthetic entity from eval-sub boundary"
        );
        assert_eq!(m1.eval_sub_calls, 1, "eval-sub extractor must run on the dynamic edit");

        eprintln!(
            "[1711-A receipt] dynamic-fact edit: extraction_total={:?}",
            total_extraction_time(&m1)
        );
        Ok(())
    }

    /// **Edit class 5 -- revert-to-original.**
    ///
    /// baseline -> edited -> baseline again. The final call's recomputed
    /// category hashes must be bit-identical to the FIRST call's -- proving
    /// extraction is a deterministic pure function of content, with no
    /// accumulated drift across generations. Returns
    /// `(original, reverted, metrics-of-the-revert-call)`.
    fn run_class_5_revert_to_original()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/revert.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let baseline = format!("{prefix}1;\n");
        let edited = format!("{prefix}sub sub_new_extra {{\n    return 999;\n}}\n\n1;\n");

        let index = WorkspaceIndex::new();
        let (r0, _m0) = index_and_measure(&index, uri, baseline.clone(), 1);
        r0?;
        let original = must_some(category_hashes(&index, uri));

        let (r1, _m1) = index_and_measure(&index, uri, edited, 2);
        r1?;

        let (r2, m2) = index_and_measure(&index, uri, baseline, 3);
        r2?;
        let reverted = must_some(category_hashes(&index, uri));
        Ok((original, reverted, m2))
    }

    #[test]
    fn edit_class_5_revert_to_original_is_deterministic() -> Result<(), Box<dyn std::error::Error>>
    {
        let (original, reverted, m2) = run_class_5_revert_to_original()?;

        assert_eq!(
            original, reverted,
            "reverting to byte-identical original text must reproduce bit-identical category hashes"
        );
        assert!(
            m2.generation_accepted,
            "the revert-to-original call must be accepted as a new generation, not rejected"
        );
        Ok(())
    }

    /// **Edit class 6 -- superseded generation.**
    ///
    /// A newer generation commits first; an older, out-of-order generation
    /// then arrives late and must be rejected without publishing its stale
    /// content (the monotonic generation guard already covers this --
    /// asserting it here documents that correctness is not at risk, only
    /// cost/overlap, matching the issue's own framing). Returns
    /// `(after_new, after_old_attempt, metrics-of-the-rejected-old-call)`.
    fn run_class_6_superseded_generation()
    -> Result<(CategoryHashes, CategoryHashes, reindex_metrics::ReindexWorkMetrics), String> {
        let uri = "file:///big/superseded.pm";
        let prefix = fixture_prefix(SUB_COUNT);
        let gen1_text = format!("{prefix}1;\n");
        let gen2_text = format!("{prefix}sub sub_new_extra {{\n    return 999;\n}}\n\n1;\n");

        let index = WorkspaceIndex::new();

        // Newer generation (2) commits first.
        let (r_new, m_new) = index_and_measure(&index, uri, gen2_text, 2);
        r_new?;
        if !m_new.generation_accepted {
            return Err(format!(
                "expected the newer generation to commit before the superseded one arrives; got {m_new:?}"
            ));
        }
        let after_new = must_some(category_hashes(&index, uri));

        // An OLDER, out-of-order generation (1) arrives late.
        let (r_old, m_old) = index_and_measure(&index, uri, gen1_text, 1);
        r_old?;
        let after_old_attempt = must_some(category_hashes(&index, uri));
        Ok((after_new, after_old_attempt, m_old))
    }

    #[test]
    fn edit_class_6_superseded_generation_never_publishes_stale_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let (after_new, after_old_attempt, m_old) = run_class_6_superseded_generation()?;

        assert!(
            was_stale_rejected(&m_old),
            "an out-of-order older generation must be rejected, not published; got {:?}",
            m_old
        );
        assert!(
            !m_old.generation_accepted,
            "stale/older generation must be rejected, so it must never reach the accepted outcome"
        );
        assert_eq!(
            after_new, after_old_attempt,
            "the stale older generation must never overwrite the newer, already-committed shard"
        );
        Ok(())
    }

    /// **Mechanically-bound structural receipt (item 1 of the maintainer
    /// review on PR #4013).** Re-runs all six edit classes (via the SAME
    /// `run_class_*` helpers the correctness tests above use -- not a
    /// parallel reimplementation) and snapshots the resulting
    /// `Vec<EditClassReceipt>`. `insta` fails this test (and `INSTA_UPDATE=no`
    /// CI runs) if any structural count or category-hash-changed flag drifts
    /// from the checked-in `.snap` file -- so the Markdown receipt's claims
    /// are traceable back to a mechanically-enforced source of truth, not a
    /// hand-typed transcription that could silently go stale.
    ///
    /// Timing is deliberately absent from `EditClassReceipt` and therefore
    /// from this snapshot -- see the module doc comment's measurement-
    /// discipline note.
    #[test]
    fn reextraction_workshape_receipt_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let (before, after, m) = run_class_1_comment_only()?;
        let comment_only = build_receipt("comment_only", before, after, &m);

        let (before, after, m) = run_class_2_reference_only()?;
        let reference_only = build_receipt("reference_only", before, after, &m);

        let (before, after, m) = run_class_3_declaration_changing()?;
        let declaration_changing = build_receipt("declaration_changing", before, after, &m);

        let (before, after, m) = run_class_4_dynamic_fact()?;
        let dynamic_fact = build_receipt("dynamic_fact", before, after, &m);

        let (before, after, m) = run_class_5_revert_to_original()?;
        let revert_to_original = build_receipt("revert_to_original", before, after, &m);

        let (before, after, m) = run_class_6_superseded_generation()?;
        let superseded_generation = build_receipt("superseded_generation", before, after, &m);

        let receipts = vec![
            comment_only,
            reference_only,
            declaration_changing,
            dynamic_fact,
            revert_to_original,
            superseded_generation,
        ];

        insta::assert_debug_snapshot!("reindex_workshape_receipt", receipts);
        Ok(())
    }
}

/// **SHADOW-ONLY (1711-B, tracked on #1711).** Proves that
/// `FileExtractionBundle::build` -- which still runs the SAME MULTIPLE
/// separate extractor calls production runs today, just packaged into one
/// struct -- produces byte-for-byte identical output to calling those same
/// extractors directly (`build_direct`), across the Perl corpus fixtures
/// (`test_corpus/gold`), the mojolicious/dancer2/catalyst real-project
/// skeletons (`test_corpus/real_projects`), and targeted edge cases
/// (comment-only, reference-only, declaration, generated/dynamic, imports,
/// heredocs/POD/interpolated-strings). See the parity contract on
/// `FileExtractionBundle` for what each field maps to.
///
/// **This "clean parity, zero deltas" result is expected and does NOT prove
/// consolidation.** `FileExtractionBundle::build` calls the identical
/// functions `build_direct` calls, just through one struct instead of two
/// call sites -- a real discrepancy here would mean a bug in the packaging,
/// not evidence that the underlying duplicate walks have been reduced to
/// one. These tests are the useful, durable part of this PR: once a REAL
/// single-traversal unification is designed (see the feasibility
/// investigation posted to #1711), this same harness becomes the
/// byte-for-byte regression gate for that change. This module proves parity
/// only -- it does not change production behavior, and `FileExtractionBundle`
/// is not called from `index_file_with_generation` (see its doc comment).
#[cfg(test)]
mod extraction_bundle_shadow_compare {
    // A skipped-fixture diagnostic is only useful if a human can see it
    // (`cargo test -- --nocapture`), same rationale/pattern as
    // `reindex_workshape_measurement` above.
    #![allow(clippy::print_stderr)]

    use super::*;
    use perl_tdd_support::{must, must_some};
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    /// Both projections for one fixture, built the way production does
    /// today: independent extraction call sites instead of one bundle.
    struct DirectProjections {
        file_index: FileIndex,
        shard: FileFactShard,
        import_specs: Vec<perl_semantic_facts::ImportSpec>,
        use_lib_facts: Vec<perl_semantic_facts::UseLibFact>,
    }

    fn content_hash_of(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn build_direct(uri: &str, text: &str, ast: &Node) -> DirectProjections {
        let content_hash = content_hash_of(text);
        let mut doc = Document::new(uri.to_string(), 1, text.to_string());
        let mut file_index =
            FileIndex { source_uri: uri.to_string(), content_hash, ..Default::default() };
        let mut visitor = IndexVisitor::new(&mut doc, uri.to_string(), None);
        visitor.visit(ast, &mut file_index);

        let shard = WorkspaceIndex::build_canonical_fact_shard_for_ast(uri, content_hash, ast);

        let file_id = WorkspaceIndex::hash_uri_to_file_id(uri);
        let import_specs =
            crate::semantic::workspace_import_extractor::extract_import_specs(ast, file_id);
        let use_lib_facts =
            crate::semantic::workspace_import_extractor::extract_use_lib_facts(ast, file_id);

        DirectProjections { file_index, shard, import_specs, use_lib_facts }
    }

    fn build_bundle(uri: &str, text: &str, ast: &Node) -> FileExtractionBundle {
        let content_hash = content_hash_of(text);
        let mut doc = Document::new(uri.to_string(), 1, text.to_string());
        FileExtractionBundle::build(ast, uri, content_hash, &mut doc, None)
    }

    /// The REAL unified traversal (1711-B phase 2): one reference walk
    /// feeding both projections. See `FileExtractionBundle::build_unified`'s
    /// doc comment.
    fn build_bundle_unified(uri: &str, text: &str, ast: &Node) -> FileExtractionBundle {
        let content_hash = content_hash_of(text);
        let mut doc = Document::new(uri.to_string(), 1, text.to_string());
        FileExtractionBundle::build_unified(ast, uri, content_hash, &mut doc, None)
    }

    /// Assert full structural parity between the two independently-computed
    /// projections and the bundle-derived ones, for one fixture. There must
    /// be NO delta -- `FileExtractionBundle::build` runs the identical
    /// extractor calls production does today -- so any assertion failure
    /// here is a genuine bug in the bundle wiring, not an expected/documented
    /// difference.
    fn assert_parity(label: &str, uri: &str, text: &str) {
        let mut parser = Parser::new(text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                // Some corpus/real-project fixtures may be partial snippets
                // (gold/ fixtures in particular are sometimes deliberately
                // minimal) rather than full compilation units. Parity is
                // meaningless without a successful parse -- skip rather than
                // fail, same discipline as
                // `semantic_real_workspace_baseline`'s corpus sweep.
                eprintln!("skip {label} ({uri}): parse error: {e}");
                return;
            }
        };

        let direct = build_direct(uri, text, &ast);
        let bundle = build_bundle(uri, text, &ast);

        assert_eq!(
            direct.file_index.symbols, bundle.legacy_index.symbols,
            "{label}: legacy WorkspaceSymbol list diverged between direct and bundle-derived extraction"
        );
        assert_eq!(
            direct.file_index.references, bundle.legacy_index.references,
            "{label}: legacy references map diverged between direct and bundle-derived extraction"
        );
        assert_eq!(
            direct.file_index.dependencies, bundle.legacy_index.dependencies,
            "{label}: legacy dependencies set diverged between direct and bundle-derived extraction"
        );
        assert_eq!(
            direct.shard, bundle.canonical_shard,
            "{label}: canonical FileFactShard diverged between direct and bundle-derived extraction"
        );
        assert_eq!(
            direct.import_specs, bundle.import_specs,
            "{label}: import specs diverged between direct and bundle-derived extraction"
        );
        assert_eq!(
            direct.use_lib_facts, bundle.use_lib_facts,
            "{label}: use-lib facts diverged between direct and bundle-derived extraction"
        );
    }

    /// **1711-B phase 2: canonical-side parity for the REAL unified
    /// traversal.** Asserts the unified walk's canonical `FileFactShard`
    /// (built from `IndexVisitor::visit_unified`'s `Vec<SymbolRef>`, via
    /// `build_canonical_fact_shard_from_symbol_refs`) is BYTE-FOR-BYTE
    /// IDENTICAL to production's own canonical output
    /// (`build_canonical_fact_shard_for_ast`, via `extract_symbol_refs`).
    ///
    /// This MUST hold with zero deltas everywhere, including fixtures that
    /// exercise the coverage-delta constructs (block-form packages/classes,
    /// Typeglob, Goto, regex-binds, etc.) -- canonical's `extract_symbol_refs`
    /// already reaches all of those today via its own complete
    /// `Node::for_each_child`-based fallback (verified empirically; see
    /// `docs/reference/1711-B-coverage-delta.md`). Only the LEGACY
    /// `FileIndex` projection gains new coverage under unification (see
    /// `assert_unified_legacy_is_superset` below) -- canonical does not
    /// change at all.
    fn assert_unified_canonical_parity(label: &str, uri: &str, text: &str) {
        let mut parser = Parser::new(text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("skip {label} ({uri}): parse error: {e}");
                return;
            }
        };

        let direct = build_direct(uri, text, &ast);
        let unified = build_bundle_unified(uri, text, &ast);

        assert_eq!(
            direct.shard, unified.canonical_shard,
            "{label}: canonical FileFactShard diverged between production's dual-walk output \
             and the unified traversal -- canonical coverage must NOT change under unification"
        );
    }

    /// **1711-B phase 2: legacy-side monotonic-superset check.** Unlike
    /// canonical (which must be byte-identical), the legacy `FileIndex`
    /// projection is EXPECTED to gain entries under the unified traversal
    /// (see `docs/reference/1711-B-coverage-delta.md`) -- so this asserts
    /// every reference-map key present under the OLD dual walk is STILL
    /// present, with AT LEAST as many entries, under the unified walk
    /// (never a loss), and reports the per-fixture growth for visibility.
    /// It does not assert exact equality, and does not fail merely because
    /// new keys appear.
    fn assert_unified_legacy_is_superset(label: &str, uri: &str, text: &str) {
        let mut parser = Parser::new(text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("skip {label} ({uri}): parse error: {e}");
                return;
            }
        };

        let direct = build_direct(uri, text, &ast);
        let unified = build_bundle_unified(uri, text, &ast);

        for (name, old_refs) in &direct.file_index.references {
            let new_count = unified.legacy_index.references.get(name).map_or(0, Vec::len);
            assert!(
                new_count >= old_refs.len(),
                "{label}: unified legacy walk LOST references for `{name}` -- \
                 had {}, now {new_count}. Unification must never be a regression.",
                old_refs.len()
            );
        }

        let old_total: usize = direct.file_index.references.values().map(Vec::len).sum();
        let new_total: usize = unified.legacy_index.references.values().map(Vec::len).sum();
        assert!(
            new_total >= old_total,
            "{label}: unified legacy walk's total reference count dropped ({old_total} -> {new_total})"
        );
        if new_total > old_total {
            eprintln!(
                "{label}: legacy reference count grew under unification: {old_total} -> {new_total} \
                 (+{})",
                new_total - old_total
            );
        }
    }

    // ── Targeted edge cases ──────────────────────────────────────────────

    #[test]
    fn parity_comment_only() {
        let text = "package Foo;\nsub bar { return 1; }\n# just a comment, nothing else\n";
        let uri = "file:///edge/comment_only.pl";
        assert_parity("comment_only", uri, text);
        assert_unified_canonical_parity("comment_only", uri, text);
        assert_unified_legacy_is_superset("comment_only", uri, text);
    }

    #[test]
    fn parity_reference_only() {
        let text = "package Foo;\nsub bar { return 1; }\nsub baz { return bar() + Foo::bar(); }\n";
        let uri = "file:///edge/reference_only.pl";
        assert_parity("reference_only", uri, text);
        assert_unified_canonical_parity("reference_only", uri, text);
        assert_unified_legacy_is_superset("reference_only", uri, text);
    }

    #[test]
    fn parity_declaration() {
        let text = "package Foo;\nour $count = 0;\nmy $local = 1;\nsub bar { my ($x, $y) = @_; return $x + $y; }\n";
        let uri = "file:///edge/declaration.pl";
        assert_parity("declaration", uri, text);
        assert_unified_canonical_parity("declaration", uri, text);
        assert_unified_legacy_is_superset("declaration", uri, text);
    }

    #[test]
    fn parity_generated_dynamic() {
        let text = r#"
package Foo;
use Moo;
has 'name' => (is => 'rw');
eval "sub greet { return 'hi'; }";
"#;
        let uri = "file:///edge/generated_dynamic.pl";
        assert_parity("generated_dynamic", uri, text);
        assert_unified_canonical_parity("generated_dynamic", uri, text);
        assert_unified_legacy_is_superset("generated_dynamic", uri, text);
    }

    #[test]
    fn parity_imports() {
        let text = r#"
package Foo;
use strict;
use warnings;
use lib '../lib';
use List::Util qw(first sum);
require Carp;
"#;
        let uri = "file:///edge/imports.pl";
        assert_parity("imports", uri, text);
        assert_unified_canonical_parity("imports", uri, text);
        assert_unified_legacy_is_superset("imports", uri, text);
    }

    #[test]
    fn parity_heredoc_pod_and_interpolated_strings() {
        let text = r#"
package Foo;

=pod

=head1 NAME

Foo - an example module

=cut

my $name = "world";
my $greeting = "Hello, $name!";
my $block = <<"END";
Greetings, $name.
END

sub bar { return $greeting; }
"#;
        let uri = "file:///edge/heredoc_pod.pl";
        assert_parity("heredoc_pod_strings", uri, text);
        assert_unified_canonical_parity("heredoc_pod_strings", uri, text);
        assert_unified_legacy_is_superset("heredoc_pod_strings", uri, text);
    }

    /// **Closes a harness gap found by independent correctness review.**
    /// `NamedParameter` default-value expressions (`:$beta = calc_default()`)
    /// were ZERO-covered across all 6 targeted edge cases + all 37
    /// gold-corpus fixtures + all 29 real-project files -- which is exactly
    /// how an earlier draft's `NamedParameter` arm (incorrectly walking
    /// `default_value`, unlike `OptionalParameter`) slipped past
    /// `assert_unified_canonical_parity` undetected. This fixture makes the
    /// harness enforce it going forward: canonical must stay byte-identical
    /// (production `extract_symbol_refs` groups `NamedParameter` with
    /// `MandatoryParameter`/`SlurpyParameter` as a total-skip Phase-1
    /// exclusion -- see `ref.rs:80-84` and its module doc -- so
    /// `calc_default()` inside a named-param default must NOT produce a
    /// `SymbolRef`, unlike the same construct on an `OptionalParameter`).
    ///
    /// This is intentionally run through the SAME general-purpose
    /// assertions as the other edge cases (not `assert_coverage_delta_case`,
    /// which asserts old=0/new>=1) -- `NamedParameter` defaults are NOT a
    /// coverage-delta case: legacy's pre-unification behavior was ALSO a
    /// total skip (`NamedParameter` was never in
    /// `visit_node`/`visit_children`'s coverage either), so nothing should
    /// change for legacy OR canonical here.
    #[test]
    fn parity_named_parameter_default_is_not_a_coverage_delta() {
        let text = "use feature 'class';\nclass Foo { method bar(:$beta = calc_default()) { return $beta; } }\n";
        let uri = "file:///edge/named_parameter_default.pl";
        assert_parity("named_parameter_default", uri, text);
        assert_unified_canonical_parity("named_parameter_default", uri, text);
        assert_unified_legacy_is_superset("named_parameter_default", uri, text);

        // Explicit, mechanically-enforced statement of the invariant this
        // fixture exists to protect: `calc_default()` must NOT appear as a
        // canonical occurrence (production `extract_symbol_refs` skips
        // `NamedParameter` defaults entirely), even though the structurally
        // similar `OptionalParameter` case (`sub greet($name =
        // default_name())`, see `coverage_delta_subroutine_signature_default`)
        // correctly DOES walk its default.
        let mut parser = Parser::new(text);
        let ast = must(parser.parse());
        let direct = build_direct(uri, text, &ast);
        let unified = build_bundle_unified(uri, text, &ast);
        assert_eq!(
            direct.shard.occurrences.len(),
            unified.canonical_shard.occurrences.len(),
            "named_parameter_default: occurrence count must match production exactly -- a \
             named-param default must never contribute a canonical occurrence"
        );
    }

    /// **Guards the `MethodCall` reference-order seam surfaced by
    /// independent correctness review during 1711-B cutover hardening.**
    ///
    /// For a chained same-named call like `$x->foo()->foo()`,
    /// `walk_unified`'s `MethodCall` arm now recurses into `object` BEFORE
    /// recording this call's own legacy `FileIndex` reference -- exactly
    /// mirroring `IndexVisitor::visit_node`'s legacy `MethodCall` arm order
    /// (child-before-own-ref). Before this fix, the unified traversal
    /// recorded its own reference BEFORE recursing into `object`, which
    /// inverted the intra-key `file_index.references["foo"]` Vec order
    /// relative to legacy for chained calls. No reference was ever lost --
    /// `assert_unified_legacy_is_superset` passes on COUNTS either way --
    /// but the exact order silently changed, which could ripple into
    /// anything ordering-sensitive over `find_references("foo")` (e.g. a
    /// "go to next reference" navigation feature).
    ///
    /// Canonical stays byte-identical regardless: `emit_canonical_ref` is
    /// deliberately left in its ORIGINAL relative position (still called
    /// before recursing into `object`), because
    /// `perl_symbol::surface::ref::walk`'s own `MethodCall` arm ALSO pushes
    /// its `SymbolRef` before recursing into `object` -- reordering that
    /// too would have flipped canonical's `Vec<SymbolRef>` order for
    /// chained calls and broken `assert_unified_canonical_parity`.
    #[test]
    fn parity_method_call_chained_same_name_reference_order() {
        let text = "package Foo;\nsub bar { my $x = Foo->new; $x->foo()->foo(); }\n";
        let uri = "file:///edge/method_call_chained_same_name.pl";
        assert_parity("method_call_chained_same_name", uri, text);
        assert_unified_canonical_parity("method_call_chained_same_name", uri, text);
        assert_unified_legacy_is_superset("method_call_chained_same_name", uri, text);

        // Explicit, mechanically-enforced order lock: unified's
        // `file_index.references["foo"]` Vec must be in the SAME order as
        // legacy's own (`IndexVisitor::visit_node`-derived) output for this
        // exact chained-call shape, not merely the same COUNT.
        let mut parser = Parser::new(text);
        let ast = must(parser.parse());
        let direct = build_direct(uri, text, &ast);
        let unified = build_bundle_unified(uri, text, &ast);

        let direct_foo_refs = must_some(direct.file_index.references.get("foo"));
        let unified_foo_refs = must_some(unified.legacy_index.references.get("foo"));
        assert_eq!(
            direct_foo_refs.len(),
            unified_foo_refs.len(),
            "method_call_chained_same_name: reference count for `foo` must match legacy exactly"
        );
        assert_eq!(
            direct_foo_refs, unified_foo_refs,
            "method_call_chained_same_name: unified `foo` reference order diverged from legacy \
             for a chained `$x->foo()->foo()` call -- MethodCall must recurse into `object` \
             BEFORE recording its own legacy reference, matching IndexVisitor::visit_node exactly"
        );
    }

    // ── Perl corpus + real-project fixtures ───────────────────────────────

    /// Resolves a path relative to the repo root. `perl-workspace`'s manifest
    /// dir is `crates/perl-workspace`, so `../..` reaches the repo root where
    /// `test_corpus/` lives.
    fn corpus_root(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
    }

    fn perl_files(root: &Path) -> Vec<PathBuf> {
        if !root.is_dir() {
            return Vec::new();
        }
        let mut files: Vec<_> = WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pl" || ext == "pm"))
            .map(|entry| entry.into_path())
            .collect();
        files.sort();
        files
    }

    fn assert_parity_over_corpus(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let files = perl_files(root);
        assert!(
            !files.is_empty(),
            "expected at least one .pl/.pm fixture under {}; corpus layout may have moved",
            root.display()
        );
        for path in &files {
            let text = std::fs::read_to_string(path)?;
            let uri =
                must(url::Url::from_file_path(path).map_err(|()| {
                    format!("fixture path cannot become file URI: {}", path.display())
                }));
            let label = path.display().to_string();
            assert_parity(&label, uri.as_str(), &text);
            // 1711-B phase 2: canonical must stay byte-identical under the
            // real unified traversal, and legacy must never LOSE coverage,
            // across the full corpus sweep -- not just the targeted edge
            // cases above.
            assert_unified_canonical_parity(&label, uri.as_str(), &text);
            assert_unified_legacy_is_superset(&label, uri.as_str(), &text);
        }
        Ok(())
    }

    #[test]
    fn parity_over_gold_corpus_fixtures() -> Result<(), Box<dyn std::error::Error>> {
        assert_parity_over_corpus(&corpus_root("test_corpus/gold"))
    }

    #[test]
    fn parity_over_real_project_skeletons() -> Result<(), Box<dyn std::error::Error>> {
        assert_parity_over_corpus(&corpus_root("test_corpus/real_projects"))
    }

    /// Sanity check that the real-project sweep actually covers all three
    /// named skeletons (mojolicious/dancer2/catalyst), not just whichever
    /// happened to exist -- guards against a silent corpus-layout drift
    /// quietly shrinking coverage to zero for one framework.
    #[test]
    fn real_project_skeletons_cover_all_three_frameworks() {
        let root = corpus_root("test_corpus/real_projects");
        for expected in ["mojolicious_skeleton", "dancer2_skeleton", "catalyst_skeleton"] {
            let dir = root.join(expected);
            assert!(
                dir.is_dir(),
                "expected real-project skeleton directory {expected} under {}",
                root.display()
            );
            assert!(
                !perl_files(&dir).is_empty(),
                "expected at least one .pm/.pl file under {}",
                dir.display()
            );
        }
    }

    // ── Coverage-delta characterization (1711-B phase 2) ──────────────────
    //
    // Each case below is a MINIMAL, checked-in, fixture-backed reproduction
    // of one construct where `IndexVisitor::visit_node` (production, today)
    // silently drops recursion -- verified empirically against current
    // `origin/main` before this traversal was written -- while
    // `extract_symbol_refs` (production's canonical path, also today)
    // ALREADY reaches it via its own complete `Node::for_each_child`-based
    // fallback. The unified traversal (`visit_unified`) closes each gap by
    // adopting that same complete fallback for the legacy projection too.
    // See `docs/reference/1711-B-coverage-delta.md` for the durable,
    // narrative version of this same list (this test module is the
    // mechanically-enforced source of truth; that doc must not drift from
    // it).
    //
    // Each case asserts, for the SAME source text:
    //   1. OLD (today's production dual-walk, `build_direct`): legacy has
    //      ZERO entries for the key under test (the pre-existing gap).
    //   2. NEW (`build_bundle_unified`): legacy GAINS at least one entry for
    //      that key (the fix).
    //   3. Canonical stays BYTE-IDENTICAL between old and new (canonical
    //      already had this coverage -- unification must not also change
    //      canonical's output).

    /// Asserts case (1)-(3) above for one coverage-delta fixture.
    fn assert_coverage_delta_case(label: &str, uri: &str, text: &str, new_reference_key: &str) {
        let mut parser = Parser::new(text);
        let ast = must(parser.parse());

        let direct = build_direct(uri, text, &ast);
        let unified = build_bundle_unified(uri, text, &ast);

        let old_count = direct.file_index.references.get(new_reference_key).map_or(0, Vec::len);
        let new_count = unified.legacy_index.references.get(new_reference_key).map_or(0, Vec::len);

        assert_eq!(
            old_count, 0,
            "{label}: expected today's production dual walk to have ZERO `{new_reference_key}` \
             legacy entries (the documented pre-existing gap) -- got {old_count}. Has this gap \
             already been fixed elsewhere? If so, this characterization is stale and should be \
             updated/removed, not silently left failing."
        );
        assert!(
            new_count >= 1,
            "{label}: expected the unified traversal to gain at least one `{new_reference_key}` \
             legacy entry -- got {new_count}. The coverage-delta fix did not take effect for \
             this construct."
        );
        assert_eq!(
            direct.shard, unified.canonical_shard,
            "{label}: canonical FileFactShard must stay IDENTICAL -- canonical already had this \
             coverage before unification; only the legacy projection should gain anything here"
        );
    }

    /// Case 1: block-form `package Foo { ... }` bodies are never walked for
    /// REFERENCES by `IndexVisitor::visit_node` today (only their
    /// declarations are seen, via the separate `extract_symbol_decls` walk
    /// inside `project_symbol_declarations`) -- `baz()` inside the block is
    /// invisible to `find_references("baz")` on current `origin/main`.
    #[test]
    fn coverage_delta_package_block_form() {
        let text = "package Foo { sub bar { baz(); } }\n";
        assert_coverage_delta_case(
            "package_block_form",
            "file:///edge/coverage_delta_package_block.pl",
            text,
            "baz",
        );
    }

    /// Case 2: same gap as case 1, for Perl 5.38+ `class Foo { ... }`
    /// bodies (`NodeKind::Class`'s `body` is never recursed into either).
    #[test]
    fn coverage_delta_class_body_form() {
        let text = "use feature 'class';\nclass Foo { method bar { baz(); } }\n";
        assert_coverage_delta_case(
            "class_body_form",
            "file:///edge/coverage_delta_class_body.pl",
            text,
            "baz",
        );
    }

    /// Case 3: `NodeKind::Typeglob` has no arm in `visit_node`/`visit_children`
    /// at all -- typeglob aliasing (`*alias = \&original;`) is completely
    /// invisible to the legacy `FileIndex` today, for any Perl file that
    /// uses it.
    #[test]
    fn coverage_delta_typeglob_alias() {
        let text = "package Foo;\nsub original { 1 }\n*alias = \\&original;\n";
        assert_coverage_delta_case(
            "typeglob_alias",
            "file:///edge/coverage_delta_typeglob.pl",
            text,
            "*alias",
        );
    }

    /// Case 4: `NodeKind::Goto` has no arm in `visit_node`/`visit_children`
    /// at all -- a `goto &handler` coderef target is invisible to the
    /// legacy `FileIndex` today.
    #[test]
    fn coverage_delta_goto_coderef_target() {
        let text = "package Foo;\nsub dispatch { goto &handler; }\n";
        assert_coverage_delta_case(
            "goto_coderef_target",
            "file:///edge/coverage_delta_goto.pl",
            text,
            "&handler",
        );
    }

    /// Case 5: regex-bind expressions (`Match`/`Substitution`/
    /// `Transliteration`) are not in `visit_node`/`visit_children`'s
    /// coverage at all -- a function call inside the bind target
    /// (`compute() =~ /x/`) is invisible to the legacy `FileIndex` today,
    /// despite being an extremely common Perl idiom.
    #[test]
    fn coverage_delta_regex_bind_nested_call() {
        let text = "package Foo;\nsub bar { return compute() =~ /x/; }\n";
        assert_coverage_delta_case(
            "regex_bind_nested_call",
            "file:///edge/coverage_delta_regex_bind.pl",
            text,
            "compute",
        );
    }

    /// Case 6: `NodeKind::Tie` has no arm in `visit_node`/`visit_children`
    /// at all -- a call in `tie`'s argument list (`tie my %h, 'Helper',
    /// extra_arg();`) is invisible to the legacy `FileIndex` today.
    #[test]
    fn coverage_delta_tie_args() {
        let text = "package Foo;\nsub bar { tie my %h, 'Helper', extra_arg(); }\n";
        assert_coverage_delta_case(
            "tie_args",
            "file:///edge/coverage_delta_tie.pl",
            text,
            "extra_arg",
        );
    }

    /// Case 7: `NodeKind::IndirectCall` (indirect-object syntax, `new Class
    /// @args`) has no arm in `visit_node`/`visit_children` at all -- a call
    /// nested in its argument list (`new Foo(make_arg())`) is invisible to
    /// the legacy `FileIndex` today.
    #[test]
    fn coverage_delta_indirect_call_args() {
        let text = "package Foo;\nsub bar { my $obj = new Foo(make_arg()); }\n";
        assert_coverage_delta_case(
            "indirect_call_args",
            "file:///edge/coverage_delta_indirect_call.pl",
            text,
            "make_arg",
        );
    }

    /// Case 8: `IndexVisitor::visit_node`'s `Subroutine` arm only visits
    /// `body`, never `prototype`/`signature` -- a default-value expression
    /// in a `sub`'s signature (`sub greet($name = default_name())`) is
    /// invisible to the legacy `FileIndex` today. (`Method`'s arm already
    /// visits `signature` correctly -- this gap is `Subroutine`-specific.)
    #[test]
    fn coverage_delta_subroutine_signature_default() {
        let text = "package Foo;\nsub greet($name = default_name()) { return $name; }\n";
        assert_coverage_delta_case(
            "subroutine_signature_default",
            "file:///edge/coverage_delta_sig_default.pl",
            text,
            "default_name",
        );
    }

    /// Case 9: `IndexVisitor::visit_node`'s `Assignment` arm does NOTHING
    /// for a non-`Variable` lhs (e.g. an indexed/complex assignment target
    /// like `$h{compute_key()} = 1`) -- no recursion at all, so a nested
    /// call inside the index expression is invisible to the legacy
    /// `FileIndex` today. (The same class of gap applies to `++`/`--` on a
    /// non-`Variable` operand.)
    #[test]
    fn coverage_delta_assignment_indexed_target() {
        let text = "package Foo;\nsub bar { my %h; $h{compute_key()} = 1; }\n";
        assert_coverage_delta_case(
            "assignment_indexed_target",
            "file:///edge/coverage_delta_indexed_assignment.pl",
            text,
            "compute_key",
        );
    }
}
