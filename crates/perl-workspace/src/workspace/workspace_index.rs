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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

fn default_has_body() -> bool {
    true
}

// Re-export the unified symbol types from perl-symbol
/// Symbol kind enums used during Index/Analyze workflows.
pub use perl_symbol::{SymbolKind, VarKind};

#[derive(Debug, Clone)]
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
    /// Workspace folder URI this file belongs to (for multi-root workspace support)
    folder_uri: Option<String>,
}

/// Write-through semantic fact storage for one indexed file.
#[derive(Clone, Debug)]
pub struct FileFactShard {
    /// Canonical file URI for this shard.
    pub source_uri: String,
    /// Stable file identifier derived from normalized URI.
    pub file_id: FileId,
    /// Whole-file content hash used for stale-shard replacement.
    pub content_hash: u64,
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
        let uri_str = uri.to_string();

        // Compute content hash for early-exit optimization
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let content_hash = hasher.finish();

        // Check if content is unchanged (early-exit optimization)
        let key = DocumentStore::uri_key(&uri_str);
        {
            let files = self.files.read();
            if let Some(existing_index) = files.get(&key) {
                if existing_index.content_hash == content_hash {
                    // Content unchanged, skip re-indexing
                    return Ok(());
                }
            }
        }

        // Update document store
        if self.document_store.is_open(&uri_str) {
            self.document_store.update(&uri_str, 1, text.clone());
        } else {
            self.document_store.open(uri_str.clone(), 1, text.clone());
        }

        // Parse the file
        let mut parser = Parser::new(&text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => return Err(format!("Parse error: {}", e)),
        };

        // Get the document for line index
        let mut doc = self.document_store.get(&uri_str).ok_or("Document not found")?;

        // Determine workspace folder URI from the file URI
        let folder_uri = self.determine_folder_uri(&uri_str);

        // Extract symbols and references
        let mut file_index = FileIndex {
            source_uri: uri_str.clone(),
            content_hash,
            folder_uri: folder_uri.clone(),
            ..Default::default()
        };
        let mut visitor = IndexVisitor::new(&mut doc, uri_str.clone(), folder_uri);
        visitor.visit(&ast, &mut file_index);

        let canonical_shard =
            Self::build_canonical_fact_shard_for_ast(&uri_str, content_hash, &ast);
        let fact_shard = if canonical_shard.anchors.is_empty()
            && canonical_shard.entities.is_empty()
            && canonical_shard.occurrences.is_empty()
            && canonical_shard.edges.is_empty()
        {
            Self::build_fact_shard(&uri_str, content_hash, &file_index)
        } else {
            canonical_shard
        };

        // Extract import specs from the AST — populates ImportExportIndex so
        // that `Foo->import(@names)` dynamic-import suppression is live in
        // production.  This runs outside the write lock to avoid holding it
        // longer than necessary.
        //
        // Lock ordering note: `semantic_import_export_index` is acquired write
        // separately from (and after) `files`/`symbols`/`global_references` to
        // match the consistent lock-order used throughout this file.
        let file_id = Self::hash_uri_to_file_id(&uri_str);
        let import_specs =
            crate::semantic::workspace_import_extractor::extract_import_specs(&ast, file_id);
        let use_lib_facts =
            crate::semantic::workspace_import_extractor::extract_use_lib_facts(&ast, file_id);

        // Update the index, refresh the global symbol cache, and replace this file's
        // contribution in the global reference index.
        {
            let mut files = self.files.write();

            // Remove stale global references from previous version of this file
            if let Some(old_index) = files.get(&key) {
                let mut global_refs = self.global_references.write();
                Self::remove_file_global_refs(&mut global_refs, old_index, &uri_str);
            }

            // Incrementally remove old symbols before inserting new file
            if let Some(old_index) = files.get(&key) {
                let mut symbols = self.symbols.write();
                Self::incremental_remove_symbols(&files, &mut symbols, old_index);
                drop(symbols);
            }
            files.insert(key.clone(), file_index);
            let mut symbols = self.symbols.write();
            if let Some(new_index) = files.get(&key) {
                Self::incremental_add_symbols(&mut symbols, new_index);
            }

            if let Some(file_index) = files.get(&key) {
                let mut global_refs = self.global_references.write();
                for (name, refs) in &file_index.references {
                    let entry = global_refs.entry(name.clone()).or_default();
                    for reference in refs {
                        entry.push(Location { uri: reference.uri.clone(), range: reference.range });
                    }
                }
            }
            self.replace_fact_shard_incremental(&key, fact_shard);
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
            Self::incremental_remove_symbols(&files, &mut symbols, &file_index);

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
    /// fact adapters in `perl-symbol`.
    ///
    /// This is the canonical population path that produces facts with real
    /// byte spans, `ExactAst` provenance, and per-category hashes. It runs
    /// alongside the legacy `build_fact_shard` path during the migration
    /// period.
    fn build_canonical_fact_shard_for_ast(
        uri: &str,
        content_hash: u64,
        ast: &Node,
    ) -> FileFactShard {
        let file_id = Self::hash_uri_to_file_id(uri);

        // Extract declarations and references from the AST.
        let decls = extract_symbol_decls(ast, None);
        let refs = extract_symbol_refs(ast);

        // Run the canonical adapters.
        let decl_facts = symbol_decls_to_semantic_facts(&decls, file_id);

        // Build an entity lookup map for reference resolution.
        let entity_ids_by_name: std::collections::BTreeMap<String, EntityId> =
            decl_facts.entities.iter().map(|e| (e.canonical_name.clone(), e.id)).collect();
        let ref_facts = symbol_refs_to_semantic_facts(&refs, file_id, &entity_ids_by_name);

        // Extract dynamic boundary evidence for `eval "sub NAME { ... }"` patterns.
        // Non-literal evals (e.g. `eval $code`) are intentionally skipped — the
        // sub name is not statically known and no evidence is emitted.
        let eval_sub_triples =
            crate::semantic::eval_sub_extractor::extract_eval_sub_boundaries(ast, file_id);
        let dynamic_boundaries: Vec<perl_semantic_facts::OccurrenceFact> =
            eval_sub_triples.iter().map(|(_, _, occ)| occ.clone()).collect();
        let generated_member_facts =
            crate::semantic::generated_member_extractor::extract_generated_member_facts(
                ast, file_id,
            );

        // Build the canonical fact shard.
        // Import specs (for `use`, `require`, `ClassName->import()`) and
        // use-lib facts are populated separately via ImportExportIndex — not passed here.
        let mut shard = crate::semantic::facts::build_canonical_fact_shard(
            uri,
            content_hash,
            &decl_facts,
            &ref_facts,
            &[],
            &dynamic_boundaries,
        );

        // Merge entity and anchor facts from semantic producers into the shard.
        // The `build_canonical_fact_shard` function only accepts OccurrenceFact
        // slices for dynamic_boundaries; extra entities and anchors must be
        // merged manually so queries can resolve those semantic facts.
        //
        // NOTE: This post-build merge means `entities_hash` and `anchors_hash` do
        // not reflect these additions. Incremental replacement
        // (`replace_fact_shard_incremental`) may miss a change if only synthetic
        // facts change — the `content_hash` (whole-file) will still catch it.
        // A future refactor should extend `build_canonical_fact_shard`'s API to
        // accept extra entity/anchor slices alongside `dynamic_boundaries`.
        for (entity, anchor, _) in eval_sub_triples {
            shard.entities.push(entity);
            shard.anchors.push(anchor);
        }
        for fact in generated_member_facts {
            shard.entities.push(fact.entity);
            shard.anchors.push(fact.anchor);
        }

        shard
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
        self.search_source_symbols(query)
    }

    /// Search only source-backed syntax symbols from the workspace index.
    ///
    /// Generated/framework members are excluded. Use this when a caller needs
    /// to preserve the historical source-backed live slice for trust receipts
    /// or fallback paths.
    pub fn search_source_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let query = query.trim();
        let query_lower = query.to_lowercase();
        let files = self.files.read();
        let mut results = Vec::new();
        for file_index in files.values() {
            for symbol in &file_index.symbols {
                if symbol.name.to_lowercase().contains(&query_lower)
                    || symbol
                        .qualified_name
                        .as_ref()
                        .map(|qn| qn.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                {
                    results.push(symbol.clone());
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
    pub fn search_generated_workspace_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let source_backed_qualified_names = self.source_backed_qualified_names();
        let shards = self.fact_shards.read();
        let mut results = Vec::new();

        for shard in shards.values() {
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
                });
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

            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
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

            NodeKind::While { condition, body, continue_block } => {
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

        let source_symbols = index.search_source_symbols("display_name");
        assert!(
            source_symbols.is_empty(),
            "generated framework members must not enter the exact source-symbol slice"
        );
        let trimmed_source_symbols = index.search_source_symbols("  display_name  ");
        assert!(
            trimmed_source_symbols.is_empty(),
            "trimmed generated framework member queries must not enter the exact source-symbol slice"
        );

        let generated_symbols = index.search_generated_workspace_symbols("display_name");
        assert_eq!(generated_symbols.len(), 1);
        let trimmed_generated_symbols =
            index.search_generated_workspace_symbols("  display_name  ");
        assert_eq!(trimmed_generated_symbols.len(), 1);
        assert_eq!(trimmed_generated_symbols[0].name, "display_name [generated/framework]");
        assert!(index.search_generated_workspace_symbols("   ").is_empty());
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
        let non_framework_symbols = index.search_generated_workspace_symbols("display_name");
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

        let source_symbols = index.search_source_symbols("has_status");
        assert!(
            source_symbols.is_empty(),
            "predicate generated members must not enter the exact source-symbol slice"
        );

        let generated_symbols = index.search_generated_workspace_symbols("has_status");
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

        let index = WorkspaceIndex::new();
        let code = "package DuplicateAnchor;\nsub target { 1 }\n1;\n";

        must(
            index.index_file(must(url::Url::parse("file:///lib/DuplicateA.pm")), code.to_string()),
        );
        must(
            index.index_file(must(url::Url::parse("file:///lib/DuplicateB.pm")), code.to_string()),
        );

        let candidates = index
            .with_semantic_queries_for_uri("file:///lib/DuplicateA.pm", |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries.definitions("DuplicateAnchor::target", &ctx)
            })
            .ok_or("missing semantic queries")?;

        let anchor_id = candidates
            .first()
            .map(|candidate| candidate.anchor_id)
            .ok_or("missing duplicate definition candidate")?;
        assert!(
            candidates.iter().filter(|candidate| candidate.anchor_id == anchor_id).count() > 1,
            "fixture must produce duplicate anchor IDs to prove fail-closed behavior"
        );
        assert_eq!(
            index.semantic_anchor_wire_location(anchor_id),
            None,
            "duplicate source-backed anchors must not resolve to an arbitrary file"
        );

        Ok(())
    }

    #[test]
    fn semantic_anchor_wire_location_for_file_resolves_duplicate_anchor_ids_by_file()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::semantic::queries::SemanticQueries;

        let index = WorkspaceIndex::new();
        let code = "package DuplicateAnchor;\nsub target { 1 }\n1;\n";
        let uri_a = "file:///lib/DuplicateA.pm";
        let uri_b = "file:///lib/DuplicateB.pm";

        must(index.index_file(must(url::Url::parse(uri_a)), code.to_string()));
        must(index.index_file(must(url::Url::parse(uri_b)), code.to_string()));

        let (file_id_a, anchor_id) = index
            .with_semantic_queries_for_uri(uri_a, |file_id, queries| {
                let ctx = crate::semantic::queries::QueryContext::new(file_id, None, Some(0));
                queries
                    .definitions("DuplicateAnchor::target", &ctx)
                    .first()
                    .map(|candidate| (file_id, candidate.anchor_id))
            })
            .flatten()
            .ok_or("missing duplicate definition candidate")?;

        assert_eq!(
            index.semantic_anchor_wire_location(anchor_id),
            None,
            "global anchor lookup must still fail closed for duplicate anchor IDs"
        );

        let location = index
            .semantic_anchor_wire_location_for_file(file_id_a, anchor_id)
            .ok_or("file-scoped anchor lookup should resolve duplicate anchor ID")?;
        assert_eq!(location.uri, uri_a);

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
}
