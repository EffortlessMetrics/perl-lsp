use std::time::{Duration, Instant};

use perl_workspace::slo::{OperationResult, OperationType, SloConfig, SloTracker};

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.state = state;
        state
    }
}

fn operation_from_seed(seed: u64) -> OperationType {
    match seed % 8 {
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

#[test]
fn fuzz_seeded_randomized_operations_keep_tracker_invariants() {
    let config = SloConfig { sample_window_size: 32, ..SloConfig::default() };
    let tracker = SloTracker::new(config);
    let mut rng = XorShift64 { state: 0xA24B_6CD0_7EF9_3B5D };
    let mut observed_outcomes: [Vec<bool>; 8] = std::array::from_fn(|_| Vec::new());
    for _ in 0..2500 {
        let seed = rng.next_u64();
        let operation_type = operation_from_seed(seed);
        let should_fail = (seed >> 3) & 1 == 1;
        let start = Instant::now();

        let jitter = Duration::from_nanos((seed % 250) + 1);
        std::thread::sleep(jitter);

        let result = if should_fail {
            OperationResult::Failure("fuzz-induced failure".to_string())
        } else {
            OperationResult::Success
        };
        tracker.record_operation_type(operation_type, start, result);

        let index = operation_index(operation_type);
        observed_outcomes[index].push(should_fail);
        if observed_outcomes[index].len() > 32 {
            observed_outcomes[index].remove(0);
        }
    }

    for operation_type in [
        OperationType::IndexInitialization,
        OperationType::IncrementalUpdate,
        OperationType::DefinitionLookup,
        OperationType::Completion,
        OperationType::Hover,
        OperationType::FindReferences,
        OperationType::WorkspaceSymbols,
        OperationType::FileIndexing,
    ] {
        let stats = tracker.statistics(operation_type);
        let index = operation_index(operation_type);
        let expected_count = observed_outcomes[index].len() as u64;
        let expected_failures = observed_outcomes[index].iter().filter(|&&f| f).count() as u64;
        let expected_successes = expected_count - expected_failures;
        assert_eq!(stats.total_count, expected_count);
        assert_eq!(stats.failure_count, expected_failures);
        assert_eq!(stats.success_count, expected_successes);
        assert!(!stats.avg_ms.is_nan());
    }

    let observed_total: u64 =
        tracker.all_statistics().values().map(|stats| stats.total_count).sum();
    let expected_total: u64 = observed_outcomes.iter().map(|outcomes| outcomes.len() as u64).sum();
    assert_eq!(observed_total, expected_total);

    tracker.reset();
    assert!(tracker.all_statistics().values().all(|stats| stats.total_count == 0));
}
