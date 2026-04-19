//! Mutation-killing tests for IndexState predicates and kind() method.
//!
//! The inline tests only exercise state transitions.
//! These tests target the predicate methods that are never called in the tests:
//!
//! - `is_ready()`: true only for Ready, false for all other states
//! - `is_error()`: true only for Error, false for all other states
//! - `is_transitional()`: true for Initializing/Building/Updating/Invalidating,
//!   false for Idle/Ready/Degraded/Error
//! - `kind()`: returns correct IndexStateKind variant for each state

use perl_workspace::state_machine::{
    DegradationReason, IndexStateKind, IndexStateMachine, InvalidationReason,
};

// ---------------------------------------------------------------------------
// is_ready(): only Ready state returns true
// ---------------------------------------------------------------------------

#[test]
fn is_ready_returns_false_for_idle_state() {
    let machine = IndexStateMachine::new();
    assert!(!machine.state().is_ready(), "Idle state must not be ready");
}

#[test]
fn is_ready_returns_false_for_initializing_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    assert!(
        !machine.state().is_ready(),
        "Initializing state must not be ready"
    );
}

#[test]
fn is_ready_returns_false_for_building_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(50);
    assert!(
        !machine.state().is_ready(),
        "Building state must not be ready"
    );
}

#[test]
fn is_ready_returns_true_only_after_transition_to_ready() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    assert!(
        machine.state().is_ready(),
        "Ready state must return is_ready() = true"
    );
}

#[test]
fn is_ready_returns_false_for_error_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_error("test error".to_string());
    assert!(!machine.state().is_ready(), "Error state must not be ready");
}

#[test]
fn is_ready_returns_false_for_degraded_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_degraded(DegradationReason::IoError {
        message: "disk error".to_string(),
    });
    assert!(
        !machine.state().is_ready(),
        "Degraded state must not be ready"
    );
}

// ---------------------------------------------------------------------------
// is_error(): only Error state returns true
// ---------------------------------------------------------------------------

#[test]
fn is_error_returns_false_for_idle_state() {
    let machine = IndexStateMachine::new();
    assert!(!machine.state().is_error(), "Idle state must not be error");
}

#[test]
fn is_error_returns_false_for_ready_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    assert!(!machine.state().is_error(), "Ready state must not be error");
}

#[test]
fn is_error_returns_true_only_for_error_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_error("something failed".to_string());
    assert!(
        machine.state().is_error(),
        "Error state must return is_error() = true"
    );
}

#[test]
fn is_error_returns_false_for_degraded_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_degraded(DegradationReason::ParseStorm {
        pending_parses: 100,
    });
    assert!(
        !machine.state().is_error(),
        "Degraded state must not be error"
    );
}

// ---------------------------------------------------------------------------
// is_transitional(): only transitional states return true
// ---------------------------------------------------------------------------

#[test]
fn is_transitional_returns_false_for_idle() {
    let machine = IndexStateMachine::new();
    assert!(
        !machine.state().is_transitional(),
        "Idle is not a transitional state"
    );
}

#[test]
fn is_transitional_returns_true_for_initializing() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    assert!(
        machine.state().is_transitional(),
        "Initializing must be transitional"
    );
}

#[test]
fn is_transitional_returns_true_for_building() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(100);
    assert!(
        machine.state().is_transitional(),
        "Building must be transitional"
    );
}

#[test]
fn is_transitional_returns_true_for_updating() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_updating(5);
    assert!(
        machine.state().is_transitional(),
        "Updating must be transitional"
    );
}

#[test]
fn is_transitional_returns_true_for_invalidating() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_invalidating(InvalidationReason::ManualRequest);
    assert!(
        machine.state().is_transitional(),
        "Invalidating must be transitional"
    );
}

#[test]
fn is_transitional_returns_false_for_ready() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    assert!(
        !machine.state().is_transitional(),
        "Ready is not a transitional state"
    );
}

#[test]
fn is_transitional_returns_false_for_error() {
    let machine = IndexStateMachine::new();
    machine.transition_to_error("broken".to_string());
    assert!(
        !machine.state().is_transitional(),
        "Error is not a transitional state"
    );
}

#[test]
fn is_transitional_returns_false_for_degraded() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: 5000 });
    assert!(
        !machine.state().is_transitional(),
        "Degraded is not a transitional state"
    );
}

// ---------------------------------------------------------------------------
// kind(): correct IndexStateKind for each state
// ---------------------------------------------------------------------------

#[test]
fn kind_returns_idle_for_new_machine() {
    let machine = IndexStateMachine::new();
    assert_eq!(machine.state().kind(), IndexStateKind::Idle);
}

#[test]
fn kind_returns_initializing_during_initialization() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    assert_eq!(machine.state().kind(), IndexStateKind::Initializing);
}

#[test]
fn kind_returns_building_during_build() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(50);
    assert_eq!(machine.state().kind(), IndexStateKind::Building);
}

#[test]
fn kind_returns_ready_when_index_complete() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    assert_eq!(machine.state().kind(), IndexStateKind::Ready);
}

#[test]
fn kind_returns_error_for_error_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_error("something failed".to_string());
    assert_eq!(machine.state().kind(), IndexStateKind::Error);
}

#[test]
fn kind_returns_degraded_for_degraded_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(10);
    machine.transition_to_ready(10, 100);
    machine.transition_to_degraded(DegradationReason::IoError {
        message: "disk full".to_string(),
    });
    assert_eq!(machine.state().kind(), IndexStateKind::Degraded);
}
