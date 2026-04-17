//! Workspace indexing and refactoring orchestration.

pub mod cache;
pub mod document_store;
#[cfg(feature = "memory-profiling")]
pub mod memory;
pub mod monitoring;
pub mod production_coordinator;
pub mod slo;
pub mod state_machine;
pub mod workspace_index;
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
pub use workspace_index::{IndexResourceLimits, Location, WorkspaceIndex};
