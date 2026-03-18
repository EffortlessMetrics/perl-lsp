use perl_workspace_index_monitoring::{
    EarlyExitReason, IndexInstrumentation, IndexMetrics, IndexPhase, IndexStateKind,
    IndexStateTransition,
};

#[test]
fn given_parse_metrics_when_incrementing_then_parse_storm_threshold_is_observed() {
    let metrics = IndexMetrics::with_threshold(2);
    assert_eq!(metrics.increment_pending_parses(), 1);
    assert_eq!(metrics.increment_pending_parses(), 2);
    assert!(!metrics.is_parse_storm());
    assert_eq!(metrics.increment_pending_parses(), 3);
    assert!(metrics.is_parse_storm());
    assert_eq!(metrics.decrement_pending_parses(), 2);
}

#[test]
fn given_instrumentation_when_recording_transitions_then_snapshot_reports_counts() {
    let instrumentation = IndexInstrumentation::new();
    instrumentation.record_phase_transition(IndexPhase::Idle, IndexPhase::Scanning);
    instrumentation.record_state_transition(IndexStateKind::Building, IndexStateKind::Ready);
    instrumentation.record_early_exit(perl_workspace_index_monitoring::EarlyExitRecord {
        reason: EarlyExitReason::InitialTimeBudget,
        elapsed_ms: 7,
        indexed_files: 3,
        total_files: 9,
    });

    let snapshot = instrumentation.snapshot();
    assert_eq!(
        snapshot.state_transition_counts.get(&IndexStateTransition {
            from: IndexStateKind::Building,
            to: IndexStateKind::Ready,
        }),
        Some(&1)
    );
    assert_eq!(snapshot.early_exit_counts.get(&EarlyExitReason::InitialTimeBudget), Some(&1));
    assert!(snapshot.last_early_exit.is_some());
}
