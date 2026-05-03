//! Service-level objective tracking for workspace index operations.
//!
//! Re-exports from the internal `slo` module to preserve existing caller paths
//! like `perl_workspace::workspace::slo::SloTracker`.

pub use crate::slo::{OperationResult, OperationType, SloConfig, SloStatistics, SloTracker};
