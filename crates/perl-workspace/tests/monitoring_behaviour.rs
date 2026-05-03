use perl_tdd_support::must_some;
use perl_workspace::monitoring::{
    EarlyExitReason, EarlyExitRecord, IndexInstrumentation, IndexMetrics, IndexPerformanceCaps,
    IndexPhase, IndexPhaseTransition, IndexResourceLimits, IndexStateKind, IndexStateTransition,
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
    instrumentation.record_early_exit(perl_workspace::monitoring::EarlyExitRecord {
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

// ---- IndexMetrics ----------------------------------------------------------

#[test]
fn test_metrics_default_threshold_is_10() {
    let metrics = IndexMetrics::new();
    assert_eq!(metrics.parse_storm_threshold(), 10);
}

#[test]
fn test_metrics_default_pending_count_is_zero() {
    let metrics = IndexMetrics::default();
    assert_eq!(metrics.pending_count(), 0);
}

#[test]
fn test_metrics_no_parse_storm_at_threshold() {
    let metrics = IndexMetrics::with_threshold(3);
    metrics.increment_pending_parses();
    metrics.increment_pending_parses();
    metrics.increment_pending_parses();
    // exactly at threshold — not a storm (must be *greater than*)
    assert!(!metrics.is_parse_storm());
    metrics.increment_pending_parses();
    assert!(metrics.is_parse_storm());
}

// ---- EarlyExitRecord -------------------------------------------------------

#[test]
fn test_early_exit_record_fields() {
    let record = EarlyExitRecord {
        reason: EarlyExitReason::FileLimit,
        elapsed_ms: 500,
        indexed_files: 100,
        total_files: 200,
    };
    assert_eq!(record.reason, EarlyExitReason::FileLimit);
    assert_eq!(record.elapsed_ms, 500);
    assert_eq!(record.indexed_files, 100);
    assert_eq!(record.total_files, 200);
}

#[test]
fn test_early_exit_record_incremental_budget_reason() {
    let record = EarlyExitRecord {
        reason: EarlyExitReason::IncrementalTimeBudget,
        elapsed_ms: 15,
        indexed_files: 1,
        total_files: 50,
    };
    assert_eq!(record.reason, EarlyExitReason::IncrementalTimeBudget);
}

// ---- IndexResourceLimits ---------------------------------------------------

#[test]
fn test_resource_limits_defaults() {
    let limits = IndexResourceLimits::default();
    assert_eq!(limits.max_files, 10_000);
    assert_eq!(limits.max_symbols_per_file, 5_000);
    assert_eq!(limits.max_total_symbols, 500_000);
    assert_eq!(limits.max_ast_cache_items, 100);
    assert_eq!(limits.max_scan_duration_ms, 30_000);
}

// ---- IndexPerformanceCaps --------------------------------------------------

#[test]
fn test_performance_caps_defaults() {
    let caps = IndexPerformanceCaps::default();
    assert_eq!(caps.initial_scan_budget_ms, 500);
    assert_eq!(caps.incremental_budget_ms, 10);
}

// ---- IndexInstrumentation --------------------------------------------------

#[test]
fn test_instrumentation_phase_transition_count() {
    let inst = IndexInstrumentation::new();
    inst.record_phase_transition(IndexPhase::Idle, IndexPhase::Scanning);
    inst.record_phase_transition(IndexPhase::Scanning, IndexPhase::Indexing);
    inst.record_phase_transition(IndexPhase::Idle, IndexPhase::Scanning);

    let snap = inst.snapshot();
    assert_eq!(
        snap.phase_transition_counts
            .get(&IndexPhaseTransition { from: IndexPhase::Idle, to: IndexPhase::Scanning }),
        Some(&2)
    );
    assert_eq!(
        snap.phase_transition_counts
            .get(&IndexPhaseTransition { from: IndexPhase::Scanning, to: IndexPhase::Indexing }),
        Some(&1)
    );
}

#[test]
fn test_instrumentation_multiple_early_exits_accumulate() {
    let inst = IndexInstrumentation::new();
    inst.record_early_exit(EarlyExitRecord {
        reason: EarlyExitReason::FileLimit,
        elapsed_ms: 100,
        indexed_files: 10,
        total_files: 20,
    });
    inst.record_early_exit(EarlyExitRecord {
        reason: EarlyExitReason::FileLimit,
        elapsed_ms: 200,
        indexed_files: 15,
        total_files: 20,
    });

    let snap = inst.snapshot();
    assert_eq!(snap.early_exit_counts.get(&EarlyExitReason::FileLimit), Some(&2));
    // last_early_exit should be the most recent one
    let last = must_some(snap.last_early_exit);
    assert_eq!(last.elapsed_ms, 200);
}

#[test]
fn test_instrumentation_snapshot_contains_current_state_duration() {
    let inst = IndexInstrumentation::new();
    // Even without any transitions, the snapshot should include time in Building state
    let snap = inst.snapshot();
    assert!(
        snap.state_durations_ms.contains_key(&IndexStateKind::Building),
        "snapshot must contain Building key in state_durations_ms"
    );
    let building_ms = must_some(snap.state_durations_ms.get(&IndexStateKind::Building).copied());
    // We can't assert an exact value, but it should be small in a unit test
    assert!(building_ms < 10_000, "duration should be small in a unit test: {building_ms}ms");
}

#[test]
fn test_state_transition_degraded_to_ready() {
    let inst = IndexInstrumentation::new();
    inst.record_state_transition(IndexStateKind::Building, IndexStateKind::Degraded);
    inst.record_state_transition(IndexStateKind::Degraded, IndexStateKind::Ready);

    let snap = inst.snapshot();
    assert_eq!(
        snap.state_transition_counts.get(&IndexStateTransition {
            from: IndexStateKind::Degraded,
            to: IndexStateKind::Ready,
        }),
        Some(&1)
    );
}
