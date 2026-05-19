use perl_workspace::workspace::slo::{OperationResult, OperationType, Regime, SloTracker};

#[test]
fn integration_given_workspace_index_reexports_are_used_when_api_is_exercised_then_counts_are_stable()
 {
    let tracker = SloTracker::default();

    for (index, operation_type) in [
        OperationType::IndexInitialization,
        OperationType::IncrementalUpdate,
        OperationType::DefinitionLookup,
        OperationType::Completion,
        OperationType::Hover,
        OperationType::FindReferences,
        OperationType::WorkspaceSymbols,
        OperationType::FileIndexing,
    ]
    .iter()
    .enumerate()
    {
        for _ in 0..=index {
            let start = tracker.start_operation(*operation_type);
            tracker.record_operation_type(
                *operation_type,
                start,
                if index == 7 {
                    OperationResult::Failure("index failure".into())
                } else {
                    OperationResult::Success
                },
            );
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
    ]
    .iter()
    .enumerate()
    {
        let stats = tracker.statistics(*operation_type);
        assert_eq!(stats.total_count, (index + 1) as u64);
        assert_eq!(stats.success_count, if index == 7 { 0 } else { (index + 1) as u64 });
        assert_eq!(stats.failure_count, if index == 7 { 8 } else { 0 });
    }

    let all_stats = tracker.all_statistics();
    assert_eq!(all_stats.len(), 8);
    assert!(all_stats.contains_key(&OperationType::FileIndexing));
}

#[test]
fn integration_given_workspace_slo_reexports_when_regime_api_is_used_then_counts_are_available() {
    let tracker = SloTracker::default();
    let start = tracker.start_operation(OperationType::IncrementalUpdate);

    tracker.record_operation_type_with_regime(
        OperationType::IncrementalUpdate,
        start,
        OperationResult::Success,
        Regime::Incremental,
    );

    assert_eq!(Regime::Incremental.name(), "incremental");
    assert_eq!(
        tracker.sample_count_by_regime(OperationType::IncrementalUpdate, Regime::Incremental),
        1
    );
    assert_eq!(tracker.sample_count_by_regime(OperationType::IncrementalUpdate, Regime::Warm), 0);
}
