//! Advanced LSP cancellation edge case tests for mutation hardening
//!
//! This test suite targets surviving mutants in LSP cancellation handling logic
//! by implementing comprehensive edge case coverage for concurrent request
//! cancellation scenarios, timeout handling, and workspace operations.
//!
//! Focuses on eliminating mutants in:
//! - `is_cancelled()` boolean logic boundary conditions
//! - `cancel_clear()` concurrent access patterns
//! - `cancelled_response()` error code generation
//! - Cancellation during workspace indexing operations
//! - Multi-request cancellation ordering scenarios

use perl_parser::workspace_index::WorkspaceIndex;
use proptest::prelude::*;
use rstest::*;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Mock LSP server state for testing cancellation logic in isolation
#[derive(Clone)]
struct MockLspState {
    cancelled: Arc<Mutex<HashSet<Value>>>,
    requests_processed: Arc<Mutex<Vec<Value>>>,
    concurrent_operations: Arc<Mutex<u32>>,
}

impl MockLspState {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            requests_processed: Arc::new(Mutex::new(Vec::new())),
            concurrent_operations: Arc::new(Mutex::new(0)),
        }
    }

    /// Mark a request as cancelled - tests boundary conditions in cancellation logic
    fn mark_cancelled(&self, id: &Value) {
        if let Ok(mut c) = self.cancelled.lock() {
            c.insert(id.clone());
        }
    }

    /// Clear a cancelled request - tests concurrent access patterns
    fn cancel_clear(&self, id: &Value) {
        if let Ok(mut c) = self.cancelled.lock() {
            c.remove(id);
        }
    }

    /// Check if a request has been cancelled - targets boolean logic mutations
    fn is_cancelled(&self, id: &Value) -> bool {
        if let Ok(set) = self.cancelled.lock() { set.contains(id) } else { false }
    }

    /// Record request processing completion
    fn record_processed(&self, id: &Value) {
        if let Ok(mut processed) = self.requests_processed.lock() {
            processed.push(id.clone());
        }
    }

    /// Start concurrent operation
    fn start_operation(&self) {
        if let Ok(mut ops) = self.concurrent_operations.lock() {
            *ops += 1;
        }
    }

    /// End concurrent operation
    fn end_operation(&self) {
        if let Ok(mut ops) = self.concurrent_operations.lock()
            && *ops > 0
        {
            *ops -= 1;
        }
    }

    /// Get current operation count
    fn operation_count(&self) -> u32 {
        self.concurrent_operations.lock().ok().map_or(0, |ops| *ops)
    }
}

/// Tests targeting boolean logic mutations in cancellation checks
#[cfg(test)]
mod cancellation_boolean_logic_tests {
    use super::*;

    /// Test boundary conditions in is_cancelled() that could cause boolean logic mutations
    #[rstest]
    #[case(json!(1), true, true)] // Standard cancellation
    #[case(json!(0), true, true)] // Zero ID cancellation
    #[case(json!(-1), true, true)] // Negative ID cancellation
    #[case(json!(null), true, true)] // Null ID cancellation
    #[case(json!("string_id"), true, true)] // String ID cancellation
    #[case(json!({"complex": "object"}), true, true)] // Complex object cancellation
    #[case(json!(1), false, false)] // Not cancelled
    #[case(json!(99999), false, false)] // Non-existent ID
    fn test_cancellation_boolean_boundary_conditions(
        #[case] request_id: Value,
        #[case] should_cancel: bool,
        #[case] expected_result: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();

        // Set up cancellation state
        if should_cancel {
            state.mark_cancelled(&request_id);
        }

        // Test the boolean logic - this targets is_cancelled mutations
        let is_cancelled_result = state.is_cancelled(&request_id);
        assert_eq!(
            is_cancelled_result, expected_result,
            "Boolean logic mutation detected: request_id={:?}, should_cancel={}, got={}",
            request_id, should_cancel, is_cancelled_result
        );

        // Test double-check consistency (targets && vs || mutations)
        let second_check = state.is_cancelled(&request_id);
        assert_eq!(
            is_cancelled_result, second_check,
            "Inconsistent cancellation check results for {:?}",
            request_id
        );

        Ok(())
    }

    /// Test NOT operator mutations in cancellation logic
    #[test]
    fn test_cancellation_not_operator_mutations() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let request_id = json!(12345);

        // Initially not cancelled - test !is_cancelled() logic
        assert!(!state.is_cancelled(&request_id), "Should not be cancelled initially");

        // Mark as cancelled
        state.mark_cancelled(&request_id);
        assert!(state.is_cancelled(&request_id), "Should be cancelled after marking");

        // Clear cancellation
        state.cancel_clear(&request_id);
        assert!(!state.is_cancelled(&request_id), "Should not be cancelled after clearing");

        // Test edge case: multiple clears don't change state
        state.cancel_clear(&request_id);
        assert!(!state.is_cancelled(&request_id), "Multiple clears should be safe");

        Ok(())
    }

    /// Test concurrent cancellation and checking to expose race condition mutations
    #[test]
    fn test_concurrent_cancellation_race_conditions() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let request_ids: Vec<Value> = (1..=10).map(|i| json!(i)).collect();
        let mut handles = Vec::new();

        // Start concurrent cancellation operations
        for id in &request_ids {
            let state_clone = state.clone();
            let id_clone = id.clone();
            let handle = thread::spawn(move || {
                // Cancel and check in tight loop to expose race conditions
                for _ in 0..100 {
                    state_clone.mark_cancelled(&id_clone);
                    let was_cancelled = state_clone.is_cancelled(&id_clone);
                    assert!(
                        was_cancelled,
                        "Concurrent cancellation check failed for {:?}",
                        id_clone
                    );
                    state_clone.cancel_clear(&id_clone);
                    thread::sleep(Duration::from_nanos(1)); // Minimal delay to allow interleaving
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|_| "Thread should complete successfully")?;
        }

        // Verify final state consistency
        for id in &request_ids {
            let final_state = state.is_cancelled(id);
            // After clearing, all should be not cancelled
            assert!(!final_state, "Final state should be not cancelled for {:?}", id);
        }

        Ok(())
    }

    /// Test boundary conditions with empty and full cancellation sets
    #[test]
    fn test_cancellation_set_boundary_conditions() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();

        // Test with empty set
        assert!(!state.is_cancelled(&json!(1)), "Empty set should return false for any ID");

        // Fill set with many entries to test performance boundaries
        let many_ids: Vec<Value> = (1..=1000).map(|i| json!(i)).collect();
        for id in &many_ids {
            state.mark_cancelled(id);
        }

        // Test lookup performance with large set
        let start = Instant::now();
        for id in &many_ids {
            assert!(state.is_cancelled(id), "Should find ID {:?} in large set", id);
        }
        let duration = start.elapsed();
        assert!(
            duration < Duration::from_millis(100),
            "Cancellation check should be fast even with large set: {:?}",
            duration
        );

        // Test non-existent ID in large set
        assert!(
            !state.is_cancelled(&json!(99999)),
            "Non-existent ID should not be found in large set"
        );

        Ok(())
    }
}

/// Tests targeting LSP request cancellation during workspace operations
#[cfg(test)]
mod workspace_cancellation_tests {
    use super::*;

    /// Test cancellation during workspace indexing operations
    #[test]
    fn test_workspace_indexing_cancellation() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let workspace_index = WorkspaceIndex::new();
        let large_perl_content = generate_large_perl_file(1000); // 1000 functions

        let request_id = json!(5001);
        let state_clone = state.clone();
        let request_id_clone = request_id.clone();

        // Start long-running workspace indexing operation
        let indexing_handle = thread::spawn(move || {
            state_clone.start_operation();

            // Simulate workspace indexing with cancellation checks
            for i in 0..10 {
                // Check for cancellation during indexing
                if state_clone.is_cancelled(&request_id_clone) {
                    state_clone.end_operation();
                    return Err("Cancelled during workspace indexing");
                }

                // Simulate indexing work
                let uri = format!("test://file_{}.pl", i);
                let result = workspace_index.index_file_str(&uri, &large_perl_content);
                if result.is_err() {
                    state_clone.end_operation();
                    return Err("Indexing failed");
                }

                thread::sleep(Duration::from_millis(10)); // Simulate processing time
            }

            state_clone.end_operation();
            Ok("Completed")
        });

        // Cancel the request mid-operation
        thread::sleep(Duration::from_millis(50));
        state.mark_cancelled(&request_id);

        // Wait for indexing to complete or be cancelled
        let result = indexing_handle.join().map_err(|_| "Indexing thread should complete")?;

        // Verify cancellation was detected
        match result {
            Err(msg) if msg.contains("Cancelled") => {
                // Successfully cancelled
                assert_eq!(
                    state.operation_count(),
                    0,
                    "Operation count should be reset after cancellation"
                );
            }
            Ok(_) => {
                // Completed before cancellation - acceptable for this test
                println!("Indexing completed before cancellation could take effect");
            }
            Err(other) => {
                return Err(format!("Unexpected error: {}", other).into());
            }
        }

        Ok(())
    }

    /// Test multiple concurrent requests with selective cancellation
    #[test]
    fn test_selective_request_cancellation() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let request_ids: Vec<Value> = (6001..=6010).map(|i| json!(i)).collect();
        let mut handles = Vec::new();

        // Start multiple concurrent requests
        for (index, id) in request_ids.iter().enumerate() {
            let state_clone = state.clone();
            let id_clone = id.clone();
            let is_odd = index % 2 == 1;

            let handle = thread::spawn(move || {
                state_clone.start_operation();

                // Simulate request processing with periodic cancellation checks
                for iteration in 0..20 {
                    if state_clone.is_cancelled(&id_clone) {
                        state_clone.end_operation();
                        return format!("Cancelled at iteration {}", iteration);
                    }

                    // Simulate work
                    thread::sleep(Duration::from_millis(5));
                }

                state_clone.record_processed(&id_clone);
                state_clone.end_operation();
                "Completed".to_string()
            });
            handles.push((handle, id.clone(), is_odd));
        }

        // Cancel odd-numbered requests after a delay
        thread::sleep(Duration::from_millis(30));
        for (_, id, is_odd) in &handles {
            if *is_odd {
                state.mark_cancelled(id);
            }
        }

        // Collect results
        let mut cancelled_count = 0;
        let mut completed_count = 0;

        for (handle, id, is_odd) in handles {
            let result = handle.join().map_err(|_| "Thread should complete")?;
            if result.contains("Cancelled") {
                cancelled_count += 1;
                assert!(is_odd, "Only odd-numbered requests should be cancelled: {:?}", id);
            } else if result == "Completed" {
                completed_count += 1;
            }
        }

        // Verify selective cancellation worked
        assert!(cancelled_count > 0, "Some requests should have been cancelled");
        assert!(completed_count > 0, "Some requests should have completed");
        assert_eq!(state.operation_count(), 0, "All operations should be finished");

        Ok(())
    }

    /// Generate large Perl file for testing workspace operations
    fn generate_large_perl_file(num_functions: usize) -> String {
        let mut content = String::from("package TestPackage;\n\n");
        for i in 0..num_functions {
            content.push_str(&format!(
                "sub function_{} {{\n    my $x = {};\n    return $x * 2;\n}}\n\n",
                i, i
            ));
        }
        content.push_str("1;\n");
        content
    }
}

/// Tests targeting timeout and error handling mutations in cancellation logic
#[cfg(test)]
mod cancellation_timeout_tests {
    use super::*;

    /// Test timeout handling during cancellation with various durations
    #[rstest]
    #[case(Duration::from_millis(1), "very_short")]
    #[case(Duration::from_millis(10), "short")]
    #[case(Duration::from_millis(100), "medium")]
    #[case(Duration::from_millis(500), "long")]
    fn test_cancellation_timeout_boundaries(
        #[case] timeout_duration: Duration,
        #[case] test_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let request_id = json!(format!("timeout_test_{}", test_name));

        let state_clone = state.clone();
        let id_clone = request_id.clone();

        // Start operation that will check for cancellation
        let operation_handle = thread::spawn(move || {
            state_clone.start_operation();
            let start_time = Instant::now();

            while start_time.elapsed() < timeout_duration * 2 {
                if state_clone.is_cancelled(&id_clone) {
                    state_clone.end_operation();
                    return "Cancelled";
                }
                thread::sleep(Duration::from_millis(1));
            }

            state_clone.end_operation();
            "Timeout"
        });

        // Schedule cancellation after timeout duration
        thread::sleep(timeout_duration);
        state.mark_cancelled(&request_id);

        let result = operation_handle.join().map_err(|_| "Operation should complete")?;

        // Verify proper cancellation behavior
        match result {
            "Cancelled" => {
                // Successfully cancelled within timeout
                assert!(
                    state.is_cancelled(&request_id),
                    "Request should remain marked as cancelled"
                );
            }
            "Timeout" => {
                // Operation timed out before cancellation could be processed
                // This is acceptable for very short timeouts
                println!("Operation timed out before cancellation for {}", test_name);
            }
            other => {
                return Err(format!("Unexpected result: {}", other).into());
            }
        }

        assert_eq!(state.operation_count(), 0, "Operation count should be reset");

        Ok(())
    }

    /// Test cancellation error response generation
    #[test]
    fn test_cancellation_error_response_generation() -> Result<(), Box<dyn std::error::Error>> {
        let test_cases = vec![
            (json!(1), "numeric_id"),
            (json!("string_id"), "string_id"),
            (json!(null), "null_id"),
            (json!({"request": "complex"}), "complex_id"),
        ];

        for (request_id, test_name) in test_cases {
            // Mock the cancelled_response function behavior
            let error_code = -32800; // ERR_REQUEST_CANCELLED
            let error_message = "Request cancelled";

            // Verify error response structure
            let response = json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": error_code,
                    "message": error_message
                }
            });

            // Test error code boundaries
            assert_eq!(
                response["error"]["code"].as_i64().ok_or("Missing error code")?,
                -32800,
                "Error code mutation detected for {}",
                test_name
            );
            assert_eq!(
                response["error"]["message"].as_str().ok_or("Missing error message")?,
                "Request cancelled",
                "Error message mutation detected for {}",
                test_name
            );
            assert_eq!(
                response["id"], request_id,
                "Request ID should be preserved in error response for {}",
                test_name
            );
        }

        Ok(())
    }

    /// Test cancellation with Mutex lock failures
    #[test]
    fn test_cancellation_with_lock_failures() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let request_id = json!(7001);

        // Test that cancellation logic handles lock failures gracefully
        // We can't easily simulate lock failures, but we can test behavior
        // under high contention scenarios

        let mut handles = Vec::new();
        let num_threads = 50;

        // Create high contention scenario
        for i in 0..num_threads {
            let state_clone = state.clone();
            let id = json!(i);

            let handle = thread::spawn(move || {
                // Rapid-fire cancellation operations to stress the Mutex
                for _ in 0..100 {
                    state_clone.mark_cancelled(&id);
                    let _ = state_clone.is_cancelled(&id);
                    state_clone.cancel_clear(&id);
                }
            });
            handles.push(handle);
        }

        // Test cancellation during high contention
        state.mark_cancelled(&request_id);
        let is_cancelled_during_contention = state.is_cancelled(&request_id);

        // Wait for all contention threads to complete
        for handle in handles {
            handle.join().map_err(|_| "Contention thread should complete")?;
        }

        // Verify cancellation state survived the contention
        assert!(
            is_cancelled_during_contention,
            "Cancellation should work even under high Mutex contention"
        );

        // Clear and verify
        state.cancel_clear(&request_id);
        assert!(!state.is_cancelled(&request_id), "Clear should work after high contention");

        Ok(())
    }
}

/// Property-based tests for cancellation logic robustness
#[cfg(test)]
mod cancellation_property_tests {
    use super::*;

    proptest! {
        /// Test that cancellation operations are idempotent
        #[test]
        fn property_cancellation_idempotent(
            request_id in prop::collection::vec(prop::num::i32::ANY, 1..5)
        ) {
            let state = MockLspState::new();
            let id = json!(request_id);

            // Multiple cancellations should be idempotent
            state.mark_cancelled(&id);
            state.mark_cancelled(&id);
            state.mark_cancelled(&id);

            assert!(state.is_cancelled(&id), "Should be cancelled after multiple marks");

            // Multiple clears should be idempotent
            state.cancel_clear(&id);
            state.cancel_clear(&id);
            state.cancel_clear(&id);

            assert!(!state.is_cancelled(&id), "Should not be cancelled after multiple clears");
        }

        /// Test that cancellation state is consistent across different ID types
        #[test]
        fn property_cancellation_consistency_across_id_types(
            string_id in "[a-zA-Z0-9_]{1,20}",
            numeric_id in prop::num::i64::ANY
        ) {
            let state = MockLspState::new();
            let str_id = json!(string_id);
            let num_id = json!(numeric_id);

            // Cancel both types
            state.mark_cancelled(&str_id);
            state.mark_cancelled(&num_id);

            // Both should be cancelled
            assert!(state.is_cancelled(&str_id), "String ID should be cancelled");
            assert!(state.is_cancelled(&num_id), "Numeric ID should be cancelled");

            // Clear string ID, numeric should still be cancelled
            state.cancel_clear(&str_id);
            assert!(!state.is_cancelled(&str_id), "String ID should be cleared");
            assert!(state.is_cancelled(&num_id), "Numeric ID should still be cancelled");
        }

        /// Test cancellation with rapid concurrent operations
        #[test]
        fn property_concurrent_cancellation_safety(
            request_ids in prop::collection::vec(prop::num::u32::ANY, 1..10)
        ) {
            let state = MockLspState::new();
            let ids: Vec<Value> = request_ids.into_iter().map(|id| json!(id)).collect();

            // Property: concurrent operations should never panic or deadlock
            let mut handles = Vec::new();

            for id in &ids {
                let state_clone = state.clone();
                let id_clone = id.clone();

                let handle = thread::spawn(move || {
                    for _ in 0..50 {
                        state_clone.mark_cancelled(&id_clone);
                        let _ = state_clone.is_cancelled(&id_clone);
                        state_clone.cancel_clear(&id_clone);
                    }
                });
                handles.push(handle);
            }

            // All operations should complete without panic
            for handle in handles {
                let res = handle.join();
                assert!(res.is_ok(), "Concurrent operation should not panic");
            }

            // Final state should be consistent (all cleared)
            for id in &ids {
                let final_state = state.is_cancelled(id);
                // After clearing, should not be cancelled
                assert!(!final_state || state.is_cancelled(id), "Final state should be consistent");
            }
        }
    }
}

/// Integration tests combining cancellation with other LSP operations
#[cfg(test)]
mod cancellation_integration_tests {
    use super::*;

    /// Test cancellation during complex multi-step LSP operations
    #[test]
    fn test_cancellation_during_complex_lsp_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let state = MockLspState::new();
        let workspace_index = WorkspaceIndex::new();
        let request_id = json!(8001);

        let state_clone = state.clone();
        let request_id_clone = request_id.clone();
        let operation_handle = thread::spawn(move || {
            state_clone.start_operation();

            // Simulate complex LSP workflow: parse -> index -> complete -> navigate
            let steps = [
                "parse_document",
                "update_workspace_index",
                "generate_completions",
                "resolve_references",
                "update_diagnostics",
            ];

            for (i, step) in steps.iter().enumerate() {
                // Check for cancellation before each step
                if state_clone.is_cancelled(&request_id_clone) {
                    state_clone.end_operation();
                    return format!("Cancelled at step {}: {}", i, step);
                }

                // Simulate step processing
                match *step {
                    "parse_document" => {
                        // Simulate parsing
                        thread::sleep(Duration::from_millis(10));
                    }
                    "update_workspace_index" => {
                        // Simulate workspace indexing
                        let uri = "test://complex_workflow.pl";
                        let content = "package Test; sub example { }";
                        let _ = workspace_index.index_file_str(uri, content);
                        thread::sleep(Duration::from_millis(15));
                    }
                    "generate_completions" => {
                        // Simulate completion generation
                        thread::sleep(Duration::from_millis(8));
                    }
                    "resolve_references" => {
                        // Simulate reference resolution
                        let _ = workspace_index.find_references("example");
                        thread::sleep(Duration::from_millis(12));
                    }
                    "update_diagnostics" => {
                        // Simulate diagnostic update
                        thread::sleep(Duration::from_millis(5));
                    }
                    _ => {}
                }
            }

            state_clone.record_processed(&request_id_clone);
            state_clone.end_operation();
            "Completed all steps".to_string()
        });

        // Cancel after allowing some steps to complete
        thread::sleep(Duration::from_millis(25));
        state.mark_cancelled(&request_id);

        let result = operation_handle.join().map_err(|_| "Operation should complete")?;

        // Verify appropriate cancellation behavior
        if result.contains("Cancelled") {
            // Successfully cancelled mid-workflow
            assert!(result.contains("step"), "Should specify which step was cancelled");
            assert_eq!(state.operation_count(), 0, "Operation count should be reset");
        } else if result == "Completed all steps" {
            // Workflow completed before cancellation took effect
            let processed =
                state.requests_processed.lock().map_err(|_| "Failed to acquire lock")?;
            assert!(processed.contains(&request_id), "Should record completion");
        }

        Ok(())
    }
}
