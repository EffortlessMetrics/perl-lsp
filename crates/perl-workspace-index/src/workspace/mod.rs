//! Workspace indexing and refactoring orchestration.

pub mod cache;
pub mod document_store;
pub mod slo;
pub mod state_machine;
pub mod workspace_index;
pub mod workspace_rename;

// Re-export commonly used types
pub use cache::{
    AstCacheConfig, BoundedLruCache, CacheConfig, CombinedWorkspaceCacheConfig,
    EstimateSize, SymbolCacheConfig, WorkspaceCacheConfig,
};
pub use slo::{OperationResult, OperationType, SloConfig, SloStatistics, SloTracker};
pub use state_machine::{
    BuildPhase, DegradationReason, IndexState, IndexStateMachine, IndexStateKind,
    InvalidationReason, ResourceKind, TransitionResult,
};
