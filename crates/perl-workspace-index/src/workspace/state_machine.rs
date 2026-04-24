//! Index lifecycle state machine for workspace index operations.
//!
//! Re-exports from the internal `state_machine` module to preserve existing caller paths
//! like `perl_workspace::workspace::state_machine::IndexStateMachine`.

pub use crate::state_machine::{
    BuildPhase, BuildPhaseTransition, DegradationReason, IndexState, IndexStateKind,
    IndexStateMachine, IndexStateTransition, InvalidationReason, ResourceKind, TransitionResult,
};
