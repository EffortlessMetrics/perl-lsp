// Tests for Task 1: SloTracker broadcast bug fix
//
// The bug: `record_operation` broadcasts to ALL 8 operation trackers simultaneously
// instead of recording only to the matching operation type tracker.
//
// These tests verify that:
// 1. `record_operation` accepts an `OperationType` parameter
// 2. Calling `record_operation` for one operation type ONLY updates that tracker
// 3. Other trackers remain unaffected

use std::thread;
use std::time::Duration;

use perl_workspace::slo::{OperationResult, OperationType, SloConfig, SloTracker};

/// Test: `record_operation` should only record to the specified operation type tracker.
///
/// CURRENT BUG: `record_operation` broadcasts to all 8 trackers, corrupting statistics.
/// EXPECTED: Only the tracker matching the operation type should be updated.
#[test]
fn test_record_operation_only_updates_matching_tracker() {
    let tracker = SloTracker::default();

    // Record a DefinitionLookup operation using record_operation
    // After the fix, record_operation should accept OperationType
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    thread::sleep(Duration::from_millis(1));

    // This should record ONLY to DefinitionLookup tracker, not all trackers
    tracker.record_operation(OperationType::DefinitionLookup, start, OperationResult::Success);

    let def_stats = tracker.statistics(OperationType::DefinitionLookup);
    let inc_stats = tracker.statistics(OperationType::IncrementalUpdate);
    let comp_stats = tracker.statistics(OperationType::Completion);
    let hover_stats = tracker.statistics(OperationType::Hover);
    let find_refs_stats = tracker.statistics(OperationType::FindReferences);
    let ws_sym_stats = tracker.statistics(OperationType::WorkspaceSymbols);
    let file_idx_stats = tracker.statistics(OperationType::FileIndexing);
    let init_stats = tracker.statistics(OperationType::IndexInitialization);

    // ONLY DefinitionLookup should have count = 1
    assert_eq!(
        def_stats.total_count, 1,
        "DefinitionLookup tracker should have exactly 1 record, got {}",
        def_stats.total_count
    );

    // All OTHER trackers should have count = 0 (not corrupted by broadcast)
    assert_eq!(
        inc_stats.total_count, 0,
        "IncrementalUpdate tracker should have 0 records (not corrupted by broadcast), got {}",
        inc_stats.total_count
    );
    assert_eq!(
        comp_stats.total_count, 0,
        "Completion tracker should have 0 records (not corrupted by broadcast), got {}",
        comp_stats.total_count
    );
    assert_eq!(
        hover_stats.total_count, 0,
        "Hover tracker should have 0 records (not corrupted by broadcast), got {}",
        hover_stats.total_count
    );
    assert_eq!(
        find_refs_stats.total_count, 0,
        "FindReferences tracker should have 0 records (not corrupted by broadcast), got {}",
        find_refs_stats.total_count
    );
    assert_eq!(
        ws_sym_stats.total_count, 0,
        "WorkspaceSymbols tracker should have 0 records (not corrupted by broadcast), got {}",
        ws_sym_stats.total_count
    );
    assert_eq!(
        file_idx_stats.total_count, 0,
        "FileIndexing tracker should have 0 records (not corrupted by broadcast), got {}",
        file_idx_stats.total_count
    );
    assert_eq!(
        init_stats.total_count, 0,
        "IndexInitialization tracker should have 0 records (not corrupted by broadcast), got {}",
        init_stats.total_count
    );
}

/// Test: Multiple calls to `record_operation` for different operation types
/// should each only update their respective trackers.
#[test]
fn test_record_operation_type_isolation_across_operations() {
    let tracker = SloTracker::new(SloConfig {
        sample_window_size: 100,
        ..SloConfig::default()
    });

    // Record 3 DefinitionLookup operations
    for _ in 0..3 {
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation(OperationType::DefinitionLookup, start, OperationResult::Success);
    }

    // Record 5 Completion operations
    for _ in 0..5 {
        let start = tracker.start_operation(OperationType::Completion);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation(OperationType::Completion, start, OperationResult::Success);
    }

    // Record 2 Hover operations
    for _ in 0..2 {
        let start = tracker.start_operation(OperationType::Hover);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation(OperationType::Hover, start, OperationResult::Success);
    }

    // Verify each tracker has ONLY its own records
    assert_eq!(
        tracker.statistics(OperationType::DefinitionLookup).total_count, 3,
        "DefinitionLookup should have 3 records, got {}",
        tracker.statistics(OperationType::DefinitionLookup).total_count
    );
    assert_eq!(
        tracker.statistics(OperationType::Completion).total_count, 5,
        "Completion should have 5 records, got {}",
        tracker.statistics(OperationType::Completion).total_count
    );
    assert_eq!(
        tracker.statistics(OperationType::Hover).total_count, 2,
        "Hover should have 2 records, got {}",
        tracker.statistics(OperationType::Hover).total_count
    );

    // All other trackers should be 0
    assert_eq!(tracker.statistics(OperationType::IncrementalUpdate).total_count, 0);
    assert_eq!(tracker.statistics(OperationType::FindReferences).total_count, 0);
    assert_eq!(tracker.statistics(OperationType::WorkspaceSymbols).total_count, 0);
    assert_eq!(tracker.statistics(OperationType::FileIndexing).total_count, 0);
    assert_eq!(tracker.statistics(OperationType::IndexInitialization).total_count, 0);
}

/// Test: `record_operation` with failure result should only affect matching tracker.
#[test]
fn test_record_operation_failure_only_affects_matching_tracker() {
    let tracker = SloTracker::default();

    // Record a failed DefinitionLookup
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    tracker.record_operation(
        OperationType::DefinitionLookup,
        start,
        OperationResult::Failure("symbol not found".to_string()),
    );

    let def_stats = tracker.statistics(OperationType::DefinitionLookup);
    let comp_stats = tracker.statistics(OperationType::Completion);

    // Only DefinitionLookup should have the failure
    assert_eq!(def_stats.total_count, 1);
    assert_eq!(def_stats.success_count, 0);
    assert_eq!(def_stats.failure_count, 1);

    // Completion should be untouched
    assert_eq!(comp_stats.total_count, 0);
}
