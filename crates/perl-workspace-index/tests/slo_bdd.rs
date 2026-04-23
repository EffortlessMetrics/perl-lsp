use std::thread;
use std::time::Duration;

use perl_workspace::slo::{OperationResult, OperationType, SloConfig, SloTracker};

fn operation_type_index(operation_type: OperationType) -> usize {
    match operation_type {
        OperationType::IndexInitialization => 0,
        OperationType::IncrementalUpdate => 1,
        OperationType::DefinitionLookup => 2,
        OperationType::Completion => 3,
        OperationType::Hover => 4,
        OperationType::FindReferences => 5,
        OperationType::WorkspaceSymbols => 6,
        OperationType::FileIndexing => 7,
    }
}

#[test]
fn given_a_default_config_when_succesful_definition_lookups_are_recorded_then_slo_statistics_are_healthy()
 {
    let tracker = SloTracker::default();

    for _ in 0..10 {
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::DefinitionLookup,
            start,
            OperationResult::Success,
        );
    }

    let stats = tracker.statistics(OperationType::DefinitionLookup);

    assert_eq!(stats.total_count, 10);
    assert_eq!(stats.success_count, 10);
    assert_eq!(stats.failure_count, 0);
    assert!(stats.slo_met);
}

#[test]
fn given_a_failing_operation_sequence_when_failure_rates_are_recorded_then_error_counts_reflect_the_source()
 {
    let tracker = SloTracker::default();

    for (index, should_fail) in [false, false, true, false, true].into_iter().enumerate() {
        let start = tracker.start_operation(OperationType::IndexInitialization);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::IndexInitialization,
            start,
            if should_fail {
                OperationResult::Failure(format!("failure {}", index))
            } else {
                OperationResult::Success
            },
        );
    }

    let stats = tracker.statistics(OperationType::IndexInitialization);
    assert_eq!(stats.total_count, 5);
    assert_eq!(stats.success_count, 3);
    assert_eq!(stats.failure_count, 2);
    assert_eq!(stats.error_rate, 2.0 / 5.0);
}

#[test]
fn given_a_mix_of_operation_types_when_tracking_across_the_tracker_then_all_statistics_are_isolated()
 {
    let tracker = SloTracker::new(SloConfig { sample_window_size: 16, ..SloConfig::default() });

    let operation_types = [
        OperationType::IndexInitialization,
        OperationType::IncrementalUpdate,
        OperationType::DefinitionLookup,
        OperationType::Completion,
        OperationType::Hover,
        OperationType::FindReferences,
        OperationType::WorkspaceSymbols,
        OperationType::FileIndexing,
    ];

    for (index, operation_type) in operation_types.into_iter().enumerate() {
        for _ in 0..=index {
            let start = tracker.start_operation(operation_type);
            tracker.record_operation_type(
                operation_type,
                start,
                if index == operation_types.len() - 1 {
                    OperationResult::Failure("seed".to_string())
                } else {
                    OperationResult::Success
                },
            );
        }
    }

    let all_statistics = tracker.all_statistics();
    assert_eq!(all_statistics.len(), operation_types.len());

    let expected_counts: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let expected_failures: [u64; 8] = [0, 0, 0, 0, 0, 0, 0, 8];

    for operation_type in operation_types {
        assert!(
            all_statistics.contains_key(&operation_type),
            "operation type should always be present"
        );
        let stats = &all_statistics[&operation_type];
        let expected_index = operation_type_index(operation_type);
        assert_eq!(stats.total_count, expected_counts[expected_index]);
        assert_eq!(stats.failure_count, expected_failures[expected_index]);
        assert_eq!(
            stats.success_count,
            expected_counts[expected_index] - expected_failures[expected_index]
        );
    }
}

#[test]
fn given_all_statistics_are_collected_when_reset_is_called_then_tracker_state_is_empty() {
    let tracker = SloTracker::default();
    let _ = tracker.start_operation(OperationType::Completion);
    tracker.record_operation(
        tracker.start_operation(OperationType::Completion),
        OperationResult::Success,
    );
    let before_reset = tracker.all_statistics();
    assert!(before_reset.values().next().is_some(), "at least one statistic should exist");

    tracker.reset();
    assert!(tracker.all_statistics().values().all(|stats| stats.total_count == 0));
}
