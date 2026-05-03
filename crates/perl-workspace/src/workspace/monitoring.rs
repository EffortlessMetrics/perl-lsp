//! Monitoring and lifecycle support types for workspace indexing.
//!
//! Re-exports from the internal `monitoring` module to preserve existing caller paths
//! like `perl_workspace::workspace::monitoring::IndexPhase`.

pub use crate::monitoring::{
    DegradationReason, EarlyExitReason, EarlyExitRecord, IndexInstrumentation,
    IndexInstrumentationSnapshot, IndexMetrics, IndexPerformanceCaps, IndexPhase,
    IndexPhaseTransition, IndexResourceLimits, IndexStateKind, IndexStateTransition, ResourceKind,
};
