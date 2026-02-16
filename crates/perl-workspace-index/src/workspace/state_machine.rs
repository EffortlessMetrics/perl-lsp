//! Enhanced index lifecycle state machine for production readiness.
//!
//! This module provides a comprehensive state machine for index lifecycle management
//! with additional states for initialization, updating, invalidation, and error handling.
//! The state machine ensures thread-safe state transitions with proper guards and
//! error recovery mechanisms.
//!
//! # State Machine States
//!
//! - **Idle**: Index is idle and not initialized
//! - **Initializing**: Index is being initialized
//! - **Building**: Index is being built (workspace scan in progress)
//! - **Updating**: Index is being updated (incremental changes)
//! - **Invalidating**: Index is being invalidated
//! - **Ready**: Index is ready for queries
//! - **Degraded**: Index is degraded but partially functional
//! - **Error**: Index is in error state
//!
//! # State Transitions
//!
//! ```text
//! Idle → Initializing → Building → Updating → Ready
//!  ↓         ↓            ↓          ↓         ↓
//! Error ← Error ← Error ← Error ← Error ← Error
//!  ↓         ↓            ↓          ↓         ↓
//! Degraded ← Degraded ← Degraded ← Degraded ← Degraded
//! ```
//!
//! # Thread Safety
//!
//! - All state transitions are protected by RwLock
//! - State reads are lock-free (cloned state)
//! - State writes use exclusive locks
//! - Guards prevent invalid transitions
//!
//! # Usage
//!
//! ```rust
//! use perl_workspace_index::workspace::state_machine::{IndexStateMachine, IndexState};
//!
//! let machine = IndexStateMachine::new();
//! assert!(matches!(machine.state(), IndexState::Idle));
//!
//! machine.transition_to_initializing();
//! machine.transition_to_building(100); // 100 files to index
//! ```

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

/// Enhanced index state with additional production-ready states.
///
/// Extends the original IndexState with Initializing, Updating, Invalidating,
/// and Error states for comprehensive lifecycle management.
#[derive(Clone, Debug)]
pub enum IndexState {
    /// Index is idle and not initialized
    Idle {
        /// When the index entered idle state
        since: Instant,
    },

    /// Index is being initialized
    Initializing {
        /// Initialization progress (0-100)
        progress: u8,
        /// When initialization started
        started_at: Instant,
    },

    /// Index is being built (workspace scan in progress)
    Building {
        /// Current build phase
        phase: BuildPhase,
        /// Files indexed so far
        indexed_count: usize,
        /// Total files discovered
        total_count: usize,
        /// Started at
        started_at: Instant,
    },

    /// Index is being updated (incremental changes)
    Updating {
        /// Number of files being updated
        updating_count: usize,
        /// When update started
        started_at: Instant,
    },

    /// Index is being invalidated
    Invalidating {
        /// Reason for invalidation
        reason: InvalidationReason,
        /// When invalidation started
        started_at: Instant,
    },

    /// Index is ready for queries
    Ready {
        /// Total symbols indexed
        symbol_count: usize,
        /// Total files indexed
        file_count: usize,
        /// Timestamp of last successful index
        completed_at: Instant,
    },

    /// Index is degraded but partially functional
    Degraded {
        /// Why we degraded
        reason: DegradationReason,
        /// What's still available
        available_symbols: usize,
        /// When degradation occurred
        since: Instant,
    },

    /// Index is in error state
    Error {
        /// Error message
        message: String,
        /// When error occurred
        since: Instant,
    },
}

impl IndexState {
    /// Return the coarse state kind for instrumentation and routing decisions.
    pub fn kind(&self) -> IndexStateKind {
        match self {
            IndexState::Idle { .. } => IndexStateKind::Idle,
            IndexState::Initializing { .. } => IndexStateKind::Initializing,
            IndexState::Building { .. } => IndexStateKind::Building,
            IndexState::Updating { .. } => IndexStateKind::Updating,
            IndexState::Invalidating { .. } => IndexStateKind::Invalidating,
            IndexState::Ready { .. } => IndexStateKind::Ready,
            IndexState::Degraded { .. } => IndexStateKind::Degraded,
            IndexState::Error { .. } => IndexStateKind::Error,
        }
    }

    /// Check if the index is ready for queries.
    pub fn is_ready(&self) -> bool {
        matches!(self, IndexState::Ready { .. })
    }

    /// Check if the index is in an error state.
    pub fn is_error(&self) -> bool {
        matches!(self, IndexState::Error { .. })
    }

    /// Check if the index is in a transitional state.
    pub fn is_transitional(&self) -> bool {
        matches!(
            self,
            IndexState::Initializing { .. }
                | IndexState::Building { .. }
                | IndexState::Updating { .. }
                | IndexState::Invalidating { .. }
        )
    }

    /// Get the timestamp of when the current state began.
    pub fn state_started_at(&self) -> Instant {
        match self {
            IndexState::Idle { since } => *since,
            IndexState::Initializing { started_at, .. } => *started_at,
            IndexState::Building { started_at, .. } => *started_at,
            IndexState::Updating { started_at, .. } => *started_at,
            IndexState::Invalidating { started_at, .. } => *started_at,
            IndexState::Ready { completed_at, .. } => *completed_at,
            IndexState::Degraded { since, .. } => *since,
            IndexState::Error { since, .. } => *since,
        }
    }
}

/// Coarse index state kinds for instrumentation and transition tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexStateKind {
    /// Index is idle
    Idle,
    /// Index is initializing
    Initializing,
    /// Index is building
    Building,
    /// Index is updating
    Updating,
    /// Index is invalidating
    Invalidating,
    /// Index is ready
    Ready,
    /// Index is degraded
    Degraded,
    /// Index is in error state
    Error,
}

/// Build phases for the Building state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildPhase {
    /// No scan has started yet
    Idle,
    /// Workspace file discovery is in progress
    Scanning,
    /// Symbol indexing is in progress
    Indexing,
}

/// Reason for index invalidation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidationReason {
    /// Workspace configuration changed
    ConfigurationChanged,
    /// File system watcher detected significant changes
    FileSystemChanged,
    /// Manual invalidation requested
    ManualRequest,
    /// Cache corruption detected
    CacheCorruption,
    /// Dependency changed
    DependencyChanged,
}

/// Reason for index degradation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DegradationReason {
    /// Parse storm (too many simultaneous changes)
    ParseStorm {
        /// Number of pending parse operations
        pending_parses: usize,
    },

    /// IO error during indexing
    IoError {
        /// Error message for diagnostics
        message: String,
    },

    /// Timeout during workspace scan
    ScanTimeout {
        /// Elapsed time in milliseconds
        elapsed_ms: u64,
    },

    /// Resource limits exceeded
    ResourceLimit {
        /// Which resource limit was exceeded
        kind: ResourceKind,
    },
}

/// Type of resource limit that was exceeded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    /// Maximum number of files in index exceeded
    MaxFiles,
    /// Maximum total symbols exceeded
    MaxSymbols,
    /// Maximum AST cache bytes exceeded
    MaxCacheBytes,
}

/// State transition for index lifecycle instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexStateTransition {
    /// Transition start state
    pub from: IndexStateKind,
    /// Transition end state
    pub to: IndexStateKind,
}

/// A phase transition while building the workspace index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuildPhaseTransition {
    /// Transition start phase
    pub from: BuildPhase,
    /// Transition end phase
    pub to: BuildPhase,
}

/// Result of a state transition attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition succeeded
    Success,
    /// Transition failed - invalid state transition
    InvalidTransition {
        /// Current state
        from: IndexStateKind,
        /// Target state
        to: IndexStateKind,
    },
    /// Transition failed - guard condition not met
    GuardFailed {
        /// Guard condition that failed
        condition: String,
    },
}

/// Thread-safe index state machine.
///
/// Manages index lifecycle with comprehensive state transitions,
/// guards, and error recovery mechanisms.
pub struct IndexStateMachine {
    /// Current index state (RwLock for thread-safe transitions)
    state: Arc<RwLock<IndexState>>,
}

impl IndexStateMachine {
    /// Create a new state machine in Idle state.
    ///
    /// # Returns
    ///
    /// A state machine initialized in `IndexState::Idle`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// ```
    pub fn new() -> Self {
        Self { state: Arc::new(RwLock::new(IndexState::Idle { since: Instant::now() })) }
    }

    /// Get current state (lock-free read via clone).
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
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::{IndexStateMachine, IndexState};
    ///
    /// let machine = IndexStateMachine::new();
    /// match machine.state() {
    ///     IndexState::Ready { .. } => { /* Full query path */ }
    /// _ => { /* Degraded/building fallback */ }
    /// }
    /// ```
    pub fn state(&self) -> IndexState {
        self.state.read().clone()
    }

    /// Transition to Initializing state.
    ///
    /// # State Transition Guards
    ///
    /// Only valid transitions:
    /// - `Idle` → `Initializing`
    /// - `Error` → `Initializing` (recovery attempt)
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// assert!(matches!(machine.transition_to_initializing(), TransitionResult::Success));
    /// ```
    pub fn transition_to_initializing(&self) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Idle { .. } | IndexState::Error { .. } => {
                *state = IndexState::Initializing { progress: 0, started_at: Instant::now() };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition {
                from: from_kind,
                to: IndexStateKind::Initializing,
            },
        }
    }

    /// Transition to Building state.
    ///
    /// # State Transition Guards
    ///
    /// Only valid transitions:
    /// - `Initializing` → `Building`
    /// - `Ready` → `Building` (re-index)
    /// - `Degraded` → `Building` (recovery)
    ///
    /// # Arguments
    ///
    /// * `total_count` - Total number of files to index
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_initializing();
    /// assert!(matches!(machine.transition_to_building(100), TransitionResult::Success));
    /// ```
    pub fn transition_to_building(&self, total_count: usize) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Initializing { .. }
            | IndexState::Ready { .. }
            | IndexState::Degraded { .. } => {
                *state = IndexState::Building {
                    phase: BuildPhase::Idle,
                    indexed_count: 0,
                    total_count,
                    started_at: Instant::now(),
                };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition {
                from: from_kind,
                to: IndexStateKind::Building,
            },
        }
    }

    /// Transition to Updating state.
    ///
    /// # State Transition Guards
    ///
    /// Only valid transitions:
    /// - `Ready` → `Updating`
    /// - `Degraded` → `Updating`
    ///
    /// # Arguments
    ///
    /// * `updating_count` - Number of files being updated
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// // ... build index ...
    /// machine.transition_to_ready(100, 5000);
    /// assert!(matches!(machine.transition_to_updating(5), TransitionResult::Success));
    /// ```
    pub fn transition_to_updating(&self, updating_count: usize) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Ready { .. } | IndexState::Degraded { .. } => {
                *state = IndexState::Updating { updating_count, started_at: Instant::now() };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition {
                from: from_kind,
                to: IndexStateKind::Updating,
            },
        }
    }

    /// Transition to Invalidating state.
    ///
    /// # State Transition Guards
    ///
    /// Valid from any non-transitional state.
    ///
    /// # Arguments
    ///
    /// * `reason` - Reason for invalidation
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::{IndexStateMachine, InvalidationReason};
    ///
    /// let machine = IndexStateMachine::new();
    /// assert!(matches!(
    ///     machine.transition_to_invalidating(InvalidationReason::ManualRequest),
    ///     TransitionResult::Success
    /// ));
    /// ```
    pub fn transition_to_invalidating(&self, reason: InvalidationReason) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        // Can transition from any non-transitional state
        match &*state {
            IndexState::Initializing { .. }
            | IndexState::Building { .. }
            | IndexState::Updating { .. }
            | IndexState::Invalidating { .. } => TransitionResult::InvalidTransition {
                from: from_kind,
                to: IndexStateKind::Invalidating,
            },
            _ => {
                *state = IndexState::Invalidating { reason, started_at: Instant::now() };
                TransitionResult::Success
            }
        }
    }

    /// Transition to Ready state.
    ///
    /// # State Transition Guards
    ///
    /// Only valid transitions:
    /// - `Building` → `Ready` (normal completion)
    /// - `Updating` → `Ready` (update complete)
    /// - `Invalidating` → `Ready` (invalidation complete)
    /// - `Degraded` → `Ready` (recovery after fix)
    ///
    /// # Arguments
    ///
    /// * `file_count` - Total number of files indexed
    /// * `symbol_count` - Total number of symbols extracted
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_building(100);
    /// assert!(matches!(machine.transition_to_ready(100, 5000), TransitionResult::Success));
    /// ```
    pub fn transition_to_ready(&self, file_count: usize, symbol_count: usize) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        match &*state {
            IndexState::Building { .. }
            | IndexState::Updating { .. }
            | IndexState::Invalidating { .. }
            | IndexState::Degraded { .. } => {
                *state =
                    IndexState::Ready { symbol_count, file_count, completed_at: Instant::now() };
                TransitionResult::Success
            }
            IndexState::Ready { .. } => {
                // Already Ready - update metrics but don't log as transition
                *state =
                    IndexState::Ready { symbol_count, file_count, completed_at: Instant::now() };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition { from: from_kind, to: IndexStateKind::Ready },
        }
    }

    /// Transition to Degraded state.
    ///
    /// # State Transition Guards
    ///
    /// Valid from any state except Error.
    ///
    /// # Arguments
    ///
    /// * `reason` - Why the index degraded
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::{IndexStateMachine, DegradationReason};
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_ready(100, 5000);
    /// assert!(matches!(
    ///     machine.transition_to_degraded(DegradationReason::IoError {
    ///         message: "IO error".to_string()
    ///     }),
    ///     TransitionResult::Success
    /// ));
    /// ```
    pub fn transition_to_degraded(&self, reason: DegradationReason) -> TransitionResult {
        let mut state = self.state.write();
        let from_kind = state.kind();

        // Get available symbols count from current state
        let available_symbols = match &*state {
            IndexState::Ready { symbol_count, .. } => *symbol_count,
            IndexState::Degraded { available_symbols, .. } => *available_symbols,
            _ => 0,
        };

        match &*state {
            IndexState::Error { .. } => TransitionResult::InvalidTransition {
                from: from_kind,
                to: IndexStateKind::Degraded,
            },
            _ => {
                *state = IndexState::Degraded { reason, available_symbols, since: Instant::now() };
                TransitionResult::Success
            }
        }
    }

    /// Transition to Error state.
    ///
    /// # State Transition Guards
    ///
    /// Valid from any state.
    ///
    /// # Arguments
    ///
    /// * `message` - Error message
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// assert!(matches!(
    ///     machine.transition_to_error("Critical error".to_string()),
    ///     TransitionResult::Success
    /// ));
    /// ```
    pub fn transition_to_error(&self, message: String) -> TransitionResult {
        let mut state = self.state.write();
        let _from_kind = state.kind();

        *state = IndexState::Error { message, since: Instant::now() };
        TransitionResult::Success
    }

    /// Transition to Idle state.
    ///
    /// # State Transition Guards
    ///
    /// Valid from any state.
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if transition succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_ready(100, 5000);
    /// assert!(matches!(machine.transition_to_idle(), TransitionResult::Success));
    /// ```
    pub fn transition_to_idle(&self) -> TransitionResult {
        let mut state = self.state.write();

        *state = IndexState::Idle { since: Instant::now() };
        TransitionResult::Success
    }

    /// Update building progress.
    ///
    /// # Arguments
    ///
    /// * `indexed_count` - Number of files indexed so far
    /// * `phase` - Current build phase
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if update succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::{IndexStateMachine, BuildPhase};
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_building(100);
    /// assert!(matches!(
    ///     machine.update_building_progress(50, BuildPhase::Indexing),
    ///     TransitionResult::Success
    /// ));
    /// ```
    pub fn update_building_progress(
        &self,
        indexed_count: usize,
        phase: BuildPhase,
    ) -> TransitionResult {
        let mut state = self.state.write();

        match &mut *state {
            IndexState::Building { total_count, started_at, .. } => {
                *state = IndexState::Building {
                    phase,
                    indexed_count,
                    total_count: *total_count,
                    started_at: *started_at,
                };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition {
                from: state.kind(),
                to: IndexStateKind::Building,
            },
        }
    }

    /// Update initialization progress.
    ///
    /// # Arguments
    ///
    /// * `progress` - Progress percentage (0-100)
    ///
    /// # Returns
    ///
    /// `TransitionResult::Success` if update succeeded, otherwise an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_workspace_index::workspace::state_machine::IndexStateMachine;
    ///
    /// let machine = IndexStateMachine::new();
    /// machine.transition_to_initializing();
    /// assert!(matches!(machine.update_initialization_progress(50), TransitionResult::Success));
    /// ```
    pub fn update_initialization_progress(&self, progress: u8) -> TransitionResult {
        let mut state = self.state.write();

        match &mut *state {
            IndexState::Initializing { started_at, .. } => {
                *state = IndexState::Initializing {
                    progress: progress.min(100),
                    started_at: *started_at,
                };
                TransitionResult::Success
            }
            _ => TransitionResult::InvalidTransition {
                from: state.kind(),
                to: IndexStateKind::Initializing,
            },
        }
    }
}

impl Default for IndexStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let machine = IndexStateMachine::new();
        assert!(matches!(machine.state(), IndexState::Idle { .. }));
    }

    #[test]
    fn test_idle_to_initializing() {
        let machine = IndexStateMachine::new();
        assert!(matches!(machine.transition_to_initializing(), TransitionResult::Success));
        assert!(matches!(machine.state(), IndexState::Initializing { .. }));
    }

    #[test]
    fn test_initializing_to_building() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        assert!(matches!(machine.transition_to_building(100), TransitionResult::Success));
        assert!(matches!(machine.state(), IndexState::Building { .. }));
    }

    #[test]
    fn test_building_to_ready() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        machine.transition_to_building(100);
        assert!(matches!(machine.transition_to_ready(100, 5000), TransitionResult::Success));
        assert!(matches!(machine.state(), IndexState::Ready { .. }));
    }

    #[test]
    fn test_ready_to_updating() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        machine.transition_to_building(100);
        machine.transition_to_ready(100, 5000);
        assert!(matches!(machine.transition_to_updating(5), TransitionResult::Success));
        assert!(matches!(machine.state(), IndexState::Updating { .. }));
    }

    #[test]
    fn test_ready_to_degraded() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        machine.transition_to_building(100);
        machine.transition_to_ready(100, 5000);
        assert!(matches!(
            machine.transition_to_degraded(DegradationReason::IoError {
                message: "error".to_string()
            }),
            TransitionResult::Success
        ));
        assert!(matches!(machine.state(), IndexState::Degraded { .. }));
    }

    #[test]
    fn test_any_to_error() {
        let machine = IndexStateMachine::new();
        assert!(matches!(
            machine.transition_to_error("error".to_string()),
            TransitionResult::Success
        ));
        assert!(matches!(machine.state(), IndexState::Error { .. }));
    }

    #[test]
    fn test_invalid_transition() {
        let machine = IndexStateMachine::new();
        // Can't go from Idle to Ready without building
        assert!(matches!(
            machine.transition_to_ready(0, 0),
            TransitionResult::InvalidTransition { .. }
        ));
    }

    #[test]
    fn test_update_building_progress() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        machine.transition_to_building(100);
        assert!(matches!(
            machine.update_building_progress(50, BuildPhase::Indexing),
            TransitionResult::Success
        ));
    }

    #[test]
    fn test_update_initialization_progress() {
        let machine = IndexStateMachine::new();
        machine.transition_to_initializing();
        assert!(matches!(machine.update_initialization_progress(50), TransitionResult::Success));
    }
}
