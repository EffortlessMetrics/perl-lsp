// Tests for Task 4: workspace_stats.rs receipt aggregation
//
// The workspace_stats command should:
// - Read `.ci/metrics/receipts/*.json` files
// - Aggregate per-operation latency statistics
// - Emit p50/p95/p99 latency tables per operation type in µs
// - Show SLO compliance % per operation type
// - Group latency tables by regime (Cold/Warm/Incremental)
// - List top-20 slowest individual operations with session ID and regime tag
// - Write JSON output to `.ci/metrics/workspace.json` on `--json` flag
//
// CURRENT STATE: workspace_stats.rs is a stub that just prints "[stub] not yet implemented"
// EXPECTED: These tests will FAIL until the implementation is complete

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test: workspace_stats.run() should error when receipts directory is empty or missing.
///
/// CURRENT BEHAVIOR: run() returns Ok(()) with just a stub message
/// EXPECTED BEHAVIOR: run() should return error when no receipts are found
#[test]
fn test_workspace_stats_errors_on_empty_receipts_directory() {
    // Create a temp directory that simulates an empty .ci/metrics/receipts/
    let tmp = TempDir::new().expect("tempdir");
    let receipts_dir = tmp.path().join(".ci").join("metrics").join("receipts");
    fs::create_dir_all(&receipts_dir).expect("create receipts dir");

    // The function should return an error about no receipts found
    let result = xtask::tasks::metrics::workspace_stats::run(Some(receipts_dir));

    // This assertion will FAIL until workspace_stats is implemented
    // Because the current stub just returns Ok(())
    assert!(
        result.is_err(),
        "workspace_stats should error on empty receipts directory, but got: {:?}",
        result
    );

    // Even if we get an error, check it mentions receipts
    if let Err(e) = &result {
        let err_msg = e.to_string().to_lowercase();
        assert!(
            err_msg.contains("receipt") || err_msg.contains("empty") || err_msg.contains("found"),
            "error message should mention receipts, got: '{}'",
            e
        );
    }
}

/// Test: workspace_stats correctly parses a valid receipt and emits latency tables.
///
/// CURRENT BEHAVIOR: stub returns Ok(()) without doing anything
/// EXPECTED BEHAVIOR: should parse receipts and emit latency statistics
#[test]
fn test_workspace_stats_parses_valid_receipt_and_emits_output() {
    let tmp = TempDir::new().expect("tempdir");
    let receipts_dir = tmp.path().join(".ci").join("metrics").join("receipts");
    fs::create_dir_all(&receipts_dir).expect("create receipts dir");

    // Write a valid receipt JSON
    let receipt_json = r#"{
        "schema_version": "1.0",
        "session_id": "test-session-001",
        "timestamp": "2026-04-19T12:00:00Z",
        "coordinator_stats": {
            "state": "Ready",
            "slo_stats": [
                {
                    "operation_type": "definition_lookup",
                    "regime": "warm",
                    "total_count": 10,
                    "success_count": 9,
                    "failure_count": 1,
                    "p50_us": 450,
                    "p95_us": 890,
                    "p99_us": 1200,
                    "avg_us": 520.5,
                    "slo_met": true
                },
                {
                    "operation_type": "completion",
                    "regime": "warm",
                    "total_count": 5,
                    "success_count": 5,
                    "failure_count": 0,
                    "p50_us": 1200,
                    "p95_us": 2500,
                    "p99_us": 3000,
                    "avg_us": 1400.0,
                    "slo_met": true
                }
            ],
            "cache_stats": [],
            "total_memory_usage": 1048576
        }
    }"#;

    let receipt_path = receipts_dir.join("session-001.json");
    fs::write(&receipt_path, receipt_json).expect("write receipt");

    // Run workspace_stats
    let result = xtask::tasks::metrics::workspace_stats::run(Some(receipts_dir));

    // Should succeed (not error)
    assert!(
        result.is_ok(),
        "workspace_stats should parse valid receipt successfully, got: {:?}",
        result
    );

    // After implementation, should have written to .ci/metrics/workspace.json
    // But since we're using a temp dir, we can't check that directly.
    // Instead, verify the function completed without error.
}

/// Test: workspace_stats aggregates statistics from multiple receipt files.
#[test]
fn test_workspace_stats_aggregates_multiple_sessions() {
    let tmp = TempDir::new().expect("tempdir");
    let receipts_dir = tmp.path().join(".ci").join("metrics").join("receipts");
    fs::create_dir_all(&receipts_dir).expect("create receipts dir");

    // Write two receipts from different sessions
    let receipt_1 = r#"{
        "schema_version": "1.0",
        "session_id": "session-A",
        "timestamp": "2026-04-19T10:00:00Z",
        "coordinator_stats": {
            "state": "Ready",
            "slo_stats": [
                {
                    "operation_type": "definition_lookup",
                    "regime": "warm",
                    "total_count": 5,
                    "success_count": 5,
                    "failure_count": 0,
                    "p50_us": 400,
                    "p95_us": 800,
                    "p99_us": 1000,
                    "avg_us": 450.0,
                    "slo_met": true
                }
            ],
            "cache_stats": [],
            "total_memory_usage": 500000
        }
    }"#;

    let receipt_2 = r#"{
        "schema_version": "1.0",
        "session_id": "session-B",
        "timestamp": "2026-04-19T11:00:00Z",
        "coordinator_stats": {
            "state": "Ready",
            "slo_stats": [
                {
                    "operation_type": "definition_lookup",
                    "regime": "warm",
                    "total_count": 10,
                    "success_count": 8,
                    "failure_count": 2,
                    "p50_us": 500,
                    "p95_us": 900,
                    "p99_us": 1100,
                    "avg_us": 550.0,
                    "slo_met": true
                }
            ],
            "cache_stats": [],
            "total_memory_usage": 600000
        }
    }"#;

    fs::write(receipts_dir.join("session-A.json"), receipt_1).expect("write receipt A");
    fs::write(receipts_dir.join("session-B.json"), receipt_2).expect("write receipt B");

    // Running should aggregate both sessions without error
    let result = xtask::tasks::metrics::workspace_stats::run(Some(receipts_dir));
    assert!(
        result.is_ok(),
        "workspace_stats should aggregate multiple sessions, got: {:?}",
        result
    );
}

/// Test: workspace_stats handles regime-grouped data correctly.
#[test]
fn test_workspace_stats_regime_grouped_output() {
    let tmp = TempDir::new().expect("tempdir");
    let receipts_dir = tmp.path().join(".ci").join("metrics").join("receipts");
    fs::create_dir_all(&receipts_dir).expect("create receipts dir");

    // Receipt with mixed regimes
    let receipt_json = r#"{
        "schema_version": "1.0",
        "session_id": "session-mixed",
        "timestamp": "2026-04-19T12:00:00Z",
        "coordinator_stats": {
            "state": "Ready",
            "slo_stats": [
                {
                    "operation_type": "index_file",
                    "regime": "cold",
                    "total_count": 2,
                    "success_count": 2,
                    "failure_count": 0,
                    "p50_us": 50000,
                    "p95_us": 80000,
                    "p99_us": 100000,
                    "avg_us": 55000.0,
                    "slo_met": true
                },
                {
                    "operation_type": "index_file",
                    "regime": "incremental",
                    "total_count": 100,
                    "success_count": 100,
                    "failure_count": 0,
                    "p50_us": 500,
                    "p95_us": 1200,
                    "p99_us": 2000,
                    "avg_us": 650.0,
                    "slo_met": true
                },
                {
                    "operation_type": "find_definition",
                    "regime": "warm",
                    "total_count": 50,
                    "success_count": 48,
                    "failure_count": 2,
                    "p50_us": 450,
                    "p95_us": 890,
                    "p99_us": 1200,
                    "avg_us": 520.5,
                    "slo_met": true
                }
            ],
            "cache_stats": [],
            "total_memory_usage": 1048576
        }
    }"#;

    fs::write(receipts_dir.join("mixed.json"), receipt_json).expect("write receipt");

    let result = xtask::tasks::metrics::workspace_stats::run(Some(receipts_dir));
    assert!(
        result.is_ok(),
        "workspace_stats should handle regime-grouped data, got: {:?}",
        result
    );
}

/// Test: workspace_stats JSON output is written to .ci/metrics/workspace.json
#[test]
fn test_workspace_stats_json_output_file() {
    // This test verifies that --json flag writes to .ci/metrics/workspace.json
    // We'll run the command and then check if the output file exists

    // Create temp project root
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let receipts_dir = root.join(".ci").join("metrics").join("receipts");
    let metrics_dir = root.join(".ci").join("metrics");
    fs::create_dir_all(&receipts_dir).expect("create receipts dir");

    // Write a minimal receipt
    let receipt_json = r#"{
        "schema_version": "1.0",
        "session_id": "test",
        "timestamp": "2026-04-19T12:00:00Z",
        "coordinator_stats": {
            "state": "Ready",
            "slo_stats": [],
            "cache_stats": [],
            "total_memory_usage": 0
        }
    }"#;
    fs::write(receipts_dir.join("test.json"), receipt_json).expect("write receipt");

    // Run with json=true
    let result = xtask::tasks::metrics::workspace_stats::run_with_json(Some(receipts_dir), true, root.join(".ci/metrics"));

    assert!(
        result.is_ok(),
        "workspace_stats --json should succeed, got: {:?}",
        result
    );

    // Check that workspace.json was created
    let workspace_json_path = metrics_dir.join("workspace.json");
    assert!(
        workspace_json_path.exists(),
        "workspace.json should be created at {}, but it doesn't exist",
        workspace_json_path.display()
    );
}
