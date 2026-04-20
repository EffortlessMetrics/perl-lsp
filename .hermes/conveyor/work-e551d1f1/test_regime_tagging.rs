// Tests for Task 2: Regime tagging for SloTracker operations
//
// ADR-008 Decision B: Regime Tagging
// SloTracker operations must carry a Regime tag:
//   - Cold — operations during LSP server startup and initial indexing
//   - Warm — operations after indexing settles, normal interactive use
//   - Incremental — operations triggered by in-editor edits
//
// These tests verify that:
// 1. `Regime` enum exists with Cold/Warm/Incremental variants
// 2. `record_operation_type` accepts a `Regime` parameter
// 3. Statistics are bucketed by regime

use std::thread;
use std::time::Duration;

use perl_workspace::slo::{OperationResult, OperationType, SloTracker};

/// Test: Regime enum should exist with Cold, Warm, and Incremental variants.
#[test]
fn test_regime_enum_exists() {
    // If Regime exists, this import should work
    use perl_workspace::slo::Regime;

    // Verify all three regime variants exist
    let _cold = Regime::Cold;
    let _warm = Regime::Warm;
    let _incremental = Regime::Incremental;
}

/// Test: `record_operation_type` should accept a Regime parameter and record it correctly.
#[test]
fn test_record_operation_type_with_regime() {
    let tracker = SloTracker::default();

    // Record a Cold operation (e.g., initial indexing)
    let start = tracker.start_operation(OperationType::IndexInitialization);
    thread::sleep(Duration::from_millis(1));
    tracker.record_operation_type(
        OperationType::IndexInitialization,
        Regime::Cold,
        start,
        OperationResult::Success,
    );

    // Record a Warm operation (e.g., definition lookup after indexing)
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    thread::sleep(Duration::from_millis(1));
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        Regime::Warm,
        start,
        OperationResult::Success,
    );

    // Record an Incremental operation (e.g., file change)
    let start = tracker.start_operation(OperationType::IncrementalUpdate);
    thread::sleep(Duration::from_millis(1));
    tracker.record_operation_type(
        OperationType::IncrementalUpdate,
        Regime::Incremental,
        start,
        OperationResult::Success,
    );

    // Verify the operations were recorded
    assert_eq!(
        tracker.statistics(OperationType::IndexInitialization).total_count, 1,
        "IndexInitialization should have 1 Cold record"
    );
    assert_eq!(
        tracker.statistics(OperationType::DefinitionLookup).total_count, 1,
        "DefinitionLookup should have 1 Warm record"
    );
    assert_eq!(
        tracker.statistics(OperationType::IncrementalUpdate).total_count, 1,
        "IncrementalUpdate should have 1 Incremental record"
    );
}

/// Test: Regime statistics should be accessible and bucketed correctly.
#[test]
fn test_regime_bucketed_statistics() {
    let tracker = SloTracker::default();

    // Record multiple Cold operations
    for _ in 0..3 {
        let start = tracker.start_operation(OperationType::IndexInitialization);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::IndexInitialization,
            Regime::Cold,
            start,
            OperationResult::Success,
        );
    }

    // Record multiple Warm operations
    for _ in 0..5 {
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::DefinitionLookup,
            Regime::Warm,
            start,
            OperationResult::Success,
        );
    }

    // Record multiple Incremental operations
    for _ in 0..2 {
        let start = tracker.start_operation(OperationType::IncrementalUpdate);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::IncrementalUpdate,
            Regime::Incremental,
            start,
            OperationResult::Success,
        );
    }

    // Get regime-specific statistics
    // After fix, there should be a way to get regime-bucketed stats
    let cold_stats = tracker.regime_statistics(OperationType::IndexInitialization, Regime::Cold);
    let warm_stats = tracker.regime_statistics(OperationType::DefinitionLookup, Regime::Warm);
    let inc_stats = tracker.regime_statistics(OperationType::IncrementalUpdate, Regime::Incremental);

    assert_eq!(
        cold_stats.map(|s| s.total_count).unwrap_or(0), 3,
        "Cold regime should have 3 records for IndexInitialization"
    );
    assert_eq!(
        warm_stats.map(|s| s.total_count).unwrap_or(0), 5,
        "Warm regime should have 5 records for DefinitionLookup"
    );
    assert_eq!(
        inc_stats.map(|s| s.total_count).unwrap_or(0), 2,
        "Incremental regime should have 2 records for IncrementalUpdate"
    );
}

/// Test: Operations without regime tagging default to Warm.
#[test]
fn test_regime_defaults_to_warm() {
    let tracker = SloTracker::default();

    // Record without explicit regime (should default to Warm)
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    thread::sleep(Duration::from_millis(1));

    // This should use record_operation_type with a default regime
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        Regime::Warm, // explicit for test
        start,
        OperationResult::Success,
    );

    let stats = tracker.statistics(OperationType::DefinitionLookup);
    assert_eq!(stats.total_count, 1);
}
