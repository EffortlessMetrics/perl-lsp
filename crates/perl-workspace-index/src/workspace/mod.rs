//! Workspace-level orchestration primitives for indexing and refactoring.
//!
//! This module groups the concrete building blocks used by the
//! [`WorkspaceIndex`](crate::workspace::WorkspaceIndex):
//!
//! - cache configuration and bounded cache implementations
//! - in-memory document storage for open editor buffers
//! - production coordination helpers and readiness orchestration
//! - state-machine and SLO/runtime monitoring integration
//! - workspace-wide symbol lookup and rename execution
//!
//! Use this module when wiring higher-level services that need to coordinate
//! indexing lifecycle, telemetry, and cross-file edits.

/// Cache policies and bounded cache implementations used by workspace indexing.
pub mod cache;
/// Open-document storage used to overlay in-editor content over on-disk files.
pub mod document_store;
#[cfg(feature = "memory-profiling")]
/// Optional memory-profiling utilities for workspace index internals.
pub mod memory;
/// Index lifecycle instrumentation and production readiness monitoring helpers.
pub mod monitoring;
/// High-level coordinator that integrates state transitions, SLOs, and limits.
pub mod production_coordinator;
/// Service-level objective types and trackers used by workspace operations.
pub mod slo;
/// State machine defining valid index lifecycle transitions and degraded states.
pub mod state_machine;
/// Core workspace-wide symbol index and lookup/query API.
pub mod workspace_index;
/// Cross-file rename planning and edit-generation helpers.
pub mod workspace_rename;

// Re-export commonly used types at the workspace level for ergonomic access.
// Note: `monitoring` types are intentionally NOT re-exported here — several names
// (e.g. `DegradationReason`, `IndexStateKind`, `ResourceKind`) overlap with those
// from `state_machine`, which would cause ambiguous glob import errors.  Callers
// that need monitoring types use `workspace::monitoring::*` or the top-level
// `crate::monitoring::*` path directly.
pub use cache::{
    AstCacheConfig, BoundedLruCache, CacheConfig, CombinedWorkspaceCacheConfig, EstimateSize,
    SymbolCacheConfig, WorkspaceCacheConfig,
};
pub use production_coordinator::{
    CoordinatorStatistics, ProductionCoordinatorConfig, ProductionIndexCoordinator,
    WorkspaceCacheManager,
};
pub use slo::{OperationResult, OperationType, SloConfig, SloStatistics, SloTracker};
pub use state_machine::{
    BuildPhase, DegradationReason, IndexState, IndexStateKind, IndexStateMachine,
    InvalidationReason, ResourceKind, TransitionResult,
};
pub use workspace_index::{
    CrossFileReferenceQuery, IndexResourceLimits, Location, SymbolIdentity, WorkspaceIndex,
};
