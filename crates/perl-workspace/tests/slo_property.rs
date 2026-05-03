use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

use perl_workspace::slo::{OperationResult, OperationType, SloTracker};

fn operation_from_code(code: u8) -> OperationType {
    match code % 8 {
        0 => OperationType::IndexInitialization,
        1 => OperationType::IncrementalUpdate,
        2 => OperationType::DefinitionLookup,
        3 => OperationType::Completion,
        4 => OperationType::Hover,
        5 => OperationType::FindReferences,
        6 => OperationType::WorkspaceSymbols,
        _ => OperationType::FileIndexing,
    }
}

fn operation_index(operation_type: OperationType) -> usize {
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

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_generated_operations_record_expected_counts_and_errors(
        ops in prop::collection::vec((0u8..8u8, any::<bool>()), 0..128),
    ) {
        let tracker = SloTracker::default();
        let mut expected_total = [0u64; 8];
        let mut expected_failure = [0u64; 8];

        for (code, is_success) in ops {
            let operation_type = operation_from_code(code);
            let start = tracker.start_operation(operation_type);
            let result = if is_success {
                OperationResult::Success
            } else {
                OperationResult::Failure("randomized failure".to_string())
            };

            tracker.record_operation_type(operation_type, start, result);

            let index = operation_index(operation_type);
            expected_total[index] += 1;
            if !is_success {
                expected_failure[index] += 1;
            }
        }

        for (index, operation_type) in [
            OperationType::IndexInitialization,
            OperationType::IncrementalUpdate,
            OperationType::DefinitionLookup,
            OperationType::Completion,
            OperationType::Hover,
            OperationType::FindReferences,
            OperationType::WorkspaceSymbols,
            OperationType::FileIndexing,
        ].into_iter().enumerate() {
            let stats = tracker.statistics(operation_type);
            let expected = expected_total[index];
            assert_eq!(stats.total_count, expected);
            assert_eq!(stats.success_count, expected - expected_failure[index]);
            assert_eq!(stats.failure_count, expected_failure[index]);
            let expected_error_rate = if expected == 0 {
                0.0
            } else {
                expected_failure[index] as f64 / expected as f64
            };
            assert!((stats.error_rate - expected_error_rate).abs() < 0.000_000_1);
        }

        let observed_total: u64 = tracker.all_statistics().values().map(|stats| stats.total_count).sum();
        let expected_total: u64 = expected_total.iter().sum();
        assert_eq!(observed_total, expected_total);
    }
}
