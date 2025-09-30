//! Targeted mutation hardening tests for LSP cancellation atomic operations
//!
//! This test suite specifically targets surviving mutants in the cancellation.rs
//! module by focusing on atomic operations, state transitions, and performance
//! optimization paths that are not adequately covered by existing integration tests.
//!
//! **Target Mutation Patterns:**
//! 1. Core atomic operations (AtomicBool load/store, Ordering mutations)
//! 2. Registry thread-safe coordination (RwLock/Mutex operation replacements)
//! 3. Performance optimization hot paths (branch prediction, relaxed checks)
//! 4. Metrics tracking atomic counter operations (fetch_add mutations)
//!
//! **Strategy:** Unit-level testing of individual methods and state transitions
//! rather than integration workflows to ensure mutants cannot survive by
//! being masked by higher-level error handling.

use perl_parser::cancellation::{
    CancellationMetrics, CancellationRegistry, GLOBAL_CANCELLATION_REGISTRY,
    PerlLspCancellationToken,
};
use proptest::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Direct atomic operations testing - targets AtomicBool load/store mutations
#[cfg(test)]
mod atomic_state_transition_tests {
    use super::*;

    /// Test atomic boolean state transitions through public API
    #[test]
    fn test_atomic_cancelled_flag_state_transitions() {
        let token = PerlLspCancellationToken::new(json!(1), "test".to_string());

        // Test initial state - targets AtomicBool::new(false) mutations
        assert!(!token.is_cancelled(), "Initial state must be not cancelled");
        assert!(!token.is_cancelled_relaxed(), "Initial relaxed state must be not cancelled");
        assert!(!token.is_cancelled_hot_path(), "Initial hot path state must be not cancelled");

        // Test state transition through cancel() - targets store(true) mutations
        token.cancel();
        assert!(token.is_cancelled(), "State must be cancelled after cancel()");
        assert!(token.is_cancelled_relaxed(), "Relaxed state must be cancelled after cancel()");
        assert!(token.is_cancelled_hot_path(), "Hot path state must be cancelled after cancel()");

        // Test state persistence - all methods must agree consistently
        for _ in 0..100 {
            assert!(token.is_cancelled(), "Cancelled state must persist across checks");
            assert!(token.is_cancelled_relaxed(), "Relaxed cancelled state must persist");
            assert!(token.is_cancelled_hot_path(), "Hot path cancelled state must persist");
        }

        // All methods must return identical results - targets ordering consistency
        let standard = token.is_cancelled();
        let relaxed = token.is_cancelled_relaxed();
        let hot_path = token.is_cancelled_hot_path();
        assert_eq!(standard, relaxed, "Standard and relaxed methods must agree");
        assert_eq!(relaxed, hot_path, "Relaxed and hot path methods must agree");
        assert_eq!(standard, hot_path, "Standard and hot path methods must agree");
    }

    /// Test is_cancelled() method consistency across multiple calls
    #[test]
    fn test_is_cancelled_consistency_mutations() {
        let token = PerlLspCancellationToken::new(json!(2), "test".to_string());

        // Test consistent false state - targets load operation stability
        for _ in 0..1000 {
            assert!(
                !token.is_cancelled(),
                "is_cancelled() must consistently return false initially"
            );
        }

        // Cancel and test consistent true state - targets atomic store/load coordination
        token.cancel();
        for _ in 0..1000 {
            assert!(
                token.is_cancelled(),
                "is_cancelled() must consistently return true after cancel"
            );
        }

        // Test that cancellation is irreversible through public API - targets state immutability
        // (There's no public method to reset cancellation, which is by design)
        assert!(token.is_cancelled(), "Cancellation should be irreversible");
    }

    /// Test cancel() method atomic store operations
    #[test]
    fn test_cancel_method_atomic_store_mutations() {
        let token = PerlLspCancellationToken::new(json!(3), "test".to_string());

        // Verify initial state
        assert!(!token.is_cancelled(), "Initial state must be not cancelled");

        // Test cancel() method - targets store(true, Ordering::Release)
        token.cancel();
        assert!(token.is_cancelled(), "cancel() must set cancelled state to true");

        // Verify state persists across all check methods - targets Release/Acquire consistency
        assert!(token.is_cancelled(), "cancel() state must be visible to is_cancelled()");
        assert!(token.is_cancelled_relaxed(), "cancel() state must be visible to relaxed check");
        assert!(token.is_cancelled_hot_path(), "cancel() state must be visible to hot path check");

        // Test idempotence - targets multiple store operations
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled(), "Multiple cancel() calls must be idempotent");
    }

    /// Test is_cancelled_relaxed() performance optimization mutations
    #[test]
    fn test_is_cancelled_relaxed_optimization_mutations() {
        let token = PerlLspCancellationToken::new(json!(4), "test".to_string());

        // Test initial state with relaxed check - targets likely() optimization
        assert!(
            !token.is_cancelled_relaxed(),
            "is_cancelled_relaxed() must return false initially"
        );

        // Cancel and test relaxed check - targets optimization path
        token.cancel();
        assert!(
            token.is_cancelled_relaxed(),
            "is_cancelled_relaxed() must return true after cancellation"
        );

        // Compare with regular check - targets optimization equivalence
        assert_eq!(
            token.is_cancelled(),
            token.is_cancelled_relaxed(),
            "Relaxed and regular checks must be equivalent"
        );
    }

    /// Test is_cancelled_hot_path() ultra-fast optimization mutations
    #[test]
    fn test_is_cancelled_hot_path_optimization_mutations() {
        let token = PerlLspCancellationToken::new(json!(5), "test".to_string());

        // Test hot path initial state
        assert!(
            !token.is_cancelled_hot_path(),
            "is_cancelled_hot_path() must return false initially"
        );

        // Cancel and test hot path
        token.cancel();
        assert!(
            token.is_cancelled_hot_path(),
            "is_cancelled_hot_path() must return true after cancellation"
        );

        // Verify all check methods are consistent - targets optimization correctness
        assert_eq!(
            token.is_cancelled(),
            token.is_cancelled_hot_path(),
            "Hot path must be equivalent to regular check"
        );
        assert_eq!(
            token.is_cancelled_relaxed(),
            token.is_cancelled_hot_path(),
            "Hot path must be equivalent to relaxed check"
        );
    }

    /// Test atomic ordering mutations under concurrent access
    #[test]
    fn test_atomic_ordering_mutations_concurrent() {
        let token = Arc::new(PerlLspCancellationToken::new(json!(6), "concurrent".to_string()));
        let num_threads = 10;
        let iterations = 100;

        let mut handles = Vec::new();

        // Spawn threads that stress atomic operations
        for thread_id in 0..num_threads {
            let token_clone = Arc::clone(&token);
            let handle = thread::spawn(move || {
                for i in 0..iterations {
                    if thread_id % 2 == 0 {
                        // Canceller threads
                        token_clone.cancel();
                        assert!(
                            token_clone.is_cancelled(),
                            "Cancel must be visible immediately in thread {}, iteration {}",
                            thread_id,
                            i
                        );
                    } else {
                        // Checker threads
                        let state1 = token_clone.is_cancelled();
                        let state2 = token_clone.is_cancelled_relaxed();
                        let state3 = token_clone.is_cancelled_hot_path();

                        // All checks must be consistent - targets ordering mutations
                        assert_eq!(
                            state1, state2,
                            "Regular and relaxed checks inconsistent in thread {}, iteration {}",
                            thread_id, i
                        );
                        assert_eq!(
                            state2, state3,
                            "Relaxed and hot path checks inconsistent in thread {}, iteration {}",
                            thread_id, i
                        );
                    }

                    // Small delay to allow interleaving
                    if i % 10 == 0 {
                        thread::sleep(Duration::from_nanos(100));
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread should complete without panic");
        }

        // Final state should be cancelled (since we have canceller threads)
        assert!(token.is_cancelled(), "Final state should be cancelled");
    }
}

/// Registry operations testing - targets RwLock/Mutex coordination mutations
#[cfg(test)]
mod registry_coordination_hardening_tests {
    use super::*;

    /// Test register_token RwLock write operations mutations
    #[test]
    fn test_register_token_rwlock_mutations() {
        let registry = CancellationRegistry::new();
        let token = PerlLspCancellationToken::new(json!(100), "register_test".to_string());

        // Test successful registration - targets RwLock::write() success path
        let result = registry.register_token(token.clone());
        assert!(result.is_ok(), "Token registration must succeed");

        // Verify active count - targets HashMap::insert() success
        assert_eq!(registry.active_count(), 1, "Active count must be 1 after registration");

        // Test duplicate registration - targets HashMap key handling
        let duplicate_token =
            PerlLspCancellationToken::new(json!(100), "duplicate_test".to_string());
        let result2 = registry.register_token(duplicate_token);
        assert!(result2.is_ok(), "Duplicate registration should overwrite");
        assert_eq!(registry.active_count(), 1, "Active count should still be 1 after duplicate");
    }

    /// Test get_token RwLock read operations mutations
    #[test]
    fn test_get_token_rwlock_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(101);
        let token = PerlLspCancellationToken::new(request_id.clone(), "get_test".to_string());

        // Test get on empty registry - targets RwLock::read() empty case
        let result = registry.get_token(&request_id);
        assert!(result.is_none(), "get_token must return None for non-existent token");

        // Register and test get - targets RwLock::read() success case
        registry.register_token(token.clone()).unwrap();
        let result = registry.get_token(&request_id);
        assert!(result.is_some(), "get_token must return Some for existing token");

        let retrieved = result.unwrap();
        assert_eq!(retrieved.request_id(), &request_id, "Retrieved token must match request ID");
        assert_eq!(retrieved.provider(), "get_test", "Retrieved token must match provider");
    }

    /// Test is_cancelled registry lookup mutations
    #[test]
    fn test_is_cancelled_registry_lookup_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(102);
        let token = PerlLspCancellationToken::new(request_id.clone(), "lookup_test".to_string());

        // Test lookup on empty registry - targets try_read() empty case
        assert!(
            !registry.is_cancelled(&request_id),
            "is_cancelled must return false for non-existent token"
        );

        // Register uncancelled token and test
        registry.register_token(token.clone()).unwrap();
        assert!(
            !registry.is_cancelled(&request_id),
            "is_cancelled must return false for uncancelled token"
        );

        // Cancel token and test - targets atomic state with registry lookup
        token.cancel();
        assert!(
            registry.is_cancelled(&request_id),
            "is_cancelled must return true after token cancellation"
        );
    }

    /// Test cancel_request coordination mutations
    #[test]
    fn test_cancel_request_coordination_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(103);
        let token = PerlLspCancellationToken::new(request_id.clone(), "cancel_test".to_string());

        // Register token
        registry.register_token(token.clone()).unwrap();

        // Test cancel_request - targets RwLock::read() + token.cancel() coordination
        let result = registry.cancel_request(&request_id);
        assert!(result.is_ok(), "cancel_request must succeed for existing token");

        // Verify token is cancelled - targets coordination success
        assert!(token.is_cancelled(), "Token must be cancelled after cancel_request");
        assert!(registry.is_cancelled(&request_id), "Registry must report token as cancelled");

        // Test cancel non-existent request - targets error handling path
        let non_existent_id = json!(999999);
        let result = registry.cancel_request(&non_existent_id);
        assert!(result.is_ok(), "cancel_request must succeed even for non-existent token");
    }

    /// Test remove_request cleanup mutations
    #[test]
    fn test_remove_request_cleanup_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(104);
        let token = PerlLspCancellationToken::new(request_id.clone(), "remove_test".to_string());

        // Register and verify presence
        registry.register_token(token.clone()).unwrap();
        assert_eq!(registry.active_count(), 1, "Token must be registered");

        // Remove and verify absence - targets HashMap::remove() success
        registry.remove_request(&request_id);
        assert_eq!(registry.active_count(), 0, "Token must be removed");

        // Verify get_token returns None - targets removal completeness
        let result = registry.get_token(&request_id);
        assert!(result.is_none(), "get_token must return None after removal");

        // Test remove non-existent - targets removal idempotence
        registry.remove_request(&request_id);
        assert_eq!(registry.active_count(), 0, "Remove non-existent should be safe");
    }

    /// Test registry caching mutations
    #[test]
    fn test_registry_cache_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(105);
        let token = PerlLspCancellationToken::new(request_id.clone(), "cache_test".to_string());

        // Register token
        registry.register_token(token.clone()).unwrap();

        // First access - cache miss, should populate cache
        let result1 = registry.get_token(&request_id);
        assert!(result1.is_some(), "First access should succeed");

        // Second access - cache hit, targets cache lookup path
        let result2 = registry.get_token(&request_id);
        assert!(result2.is_some(), "Second access should hit cache");

        // Verify both results are equivalent - targets cache correctness
        let token1 = result1.unwrap();
        let token2 = result2.unwrap();
        assert_eq!(token1.request_id(), token2.request_id(), "Cached token must match original");
        assert_eq!(token1.provider(), token2.provider(), "Cached token provider must match");

        // Test cache with cancellation state
        token.cancel();
        let cached_cancelled = registry.is_cancelled(&request_id);
        assert!(cached_cancelled, "Cached token must reflect cancellation state");
    }

    /// Test concurrent registry operations mutations
    #[test]
    fn test_concurrent_registry_operations_mutations() {
        let registry = Arc::new(CancellationRegistry::new());
        let base_id = 200;
        let num_threads = 8;
        let ops_per_thread = 50;

        let mut handles = Vec::new();

        // Spawn threads performing different registry operations
        for thread_id in 0..num_threads {
            let registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let request_id = json!(base_id + thread_id * 100 + i);
                    let token = PerlLspCancellationToken::new(
                        request_id.clone(),
                        format!("thread_{}", thread_id),
                    );

                    match thread_id % 4 {
                        0 => {
                            // Register operations
                            let _ = registry_clone.register_token(token);
                        }
                        1 => {
                            // Cancel operations
                            let _ = registry_clone.cancel_request(&request_id);
                        }
                        2 => {
                            // Lookup operations
                            let _ = registry_clone.get_token(&request_id);
                            let _ = registry_clone.is_cancelled(&request_id);
                        }
                        3 => {
                            // Remove operations
                            registry_clone.remove_request(&request_id);
                        }
                        _ => unreachable!(),
                    }

                    // Occasional yield to promote interleaving
                    if i % 10 == 0 {
                        thread::yield_now();
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.join().expect("Concurrent operation should not panic");
        }

        // Registry should be in a consistent state (no panics = success)
        let final_count = registry.active_count();
        println!("Final active count after concurrent operations: {}", final_count);

        // Any final count is acceptable as long as no panics occurred
        // This test primarily targets thread safety mutations
    }
}

/// Performance optimization testing - targets branch prediction and hot path mutations
#[cfg(test)]
mod performance_optimization_hardening_tests {
    use super::*;

    /// Test likely() branch prediction hint mutations
    #[test]
    fn test_branch_prediction_likely_mutations() {
        let token = PerlLspCancellationToken::new(json!(300), "branch_test".to_string());

        // Test the common case (not cancelled) - should be optimized with likely()
        for _ in 0..1000 {
            assert!(
                !token.is_cancelled_relaxed(),
                "Uncancelled state should be fast (likely branch)"
            );
        }

        // Cancel and test the uncommon case - should be handled correctly despite unlikely()
        token.cancel();
        for _ in 0..1000 {
            assert!(
                token.is_cancelled_relaxed(),
                "Cancelled state should work correctly (unlikely branch)"
            );
        }
    }

    /// Test performance difference between check methods
    #[test]
    fn test_performance_method_variations() {
        let token = PerlLspCancellationToken::new(json!(301), "perf_test".to_string());
        let iterations = 10000;

        // Benchmark regular is_cancelled()
        let start1 = Instant::now();
        for _ in 0..iterations {
            let _ = token.is_cancelled();
        }
        let duration1 = start1.elapsed();

        // Benchmark is_cancelled_relaxed()
        let start2 = Instant::now();
        for _ in 0..iterations {
            let _ = token.is_cancelled_relaxed();
        }
        let duration2 = start2.elapsed();

        // Benchmark is_cancelled_hot_path()
        let start3 = Instant::now();
        for _ in 0..iterations {
            let _ = token.is_cancelled_hot_path();
        }
        let duration3 = start3.elapsed();

        // All methods should be very fast (< 1ms for 10k iterations)
        assert!(
            duration1 < Duration::from_millis(1),
            "Regular check should be fast: {:?}",
            duration1
        );
        assert!(
            duration2 < Duration::from_millis(1),
            "Relaxed check should be fast: {:?}",
            duration2
        );
        assert!(
            duration3 < Duration::from_millis(1),
            "Hot path check should be fast: {:?}",
            duration3
        );

        // Print performance comparison for visibility
        println!("Performance comparison ({}k iterations):", iterations / 1000);
        println!("  Regular: {:?}", duration1);
        println!("  Relaxed: {:?}", duration2);
        println!("  Hot path: {:?}", duration3);
    }

    /// Test registry fast path vs slow path mutations
    #[test]
    fn test_registry_fast_slow_path_mutations() {
        let registry = CancellationRegistry::new();
        let request_id = json!(302);
        let token = PerlLspCancellationToken::new(request_id.clone(), "path_test".to_string());

        // Register token
        registry.register_token(token.clone()).unwrap();

        // First call - slow path (cache miss)
        let start_slow = Instant::now();
        let result_slow = registry.get_token(&request_id);
        let duration_slow = start_slow.elapsed();

        // Second call - fast path (cache hit)
        let start_fast = Instant::now();
        let result_fast = registry.get_token(&request_id);
        let duration_fast = start_fast.elapsed();

        // Both should succeed
        assert!(result_slow.is_some(), "Slow path should succeed");
        assert!(result_fast.is_some(), "Fast path should succeed");

        // Results should be equivalent - targets cache correctness
        let token_slow = result_slow.unwrap();
        let token_fast = result_fast.unwrap();
        assert_eq!(token_slow.request_id(), token_fast.request_id(), "Path results must match");

        // Fast path should generally be faster, but we won't assert on timing
        // since it's unreliable in test environments
        println!("Path performance: slow={:?}, fast={:?}", duration_slow, duration_fast);
    }

    /// Test cache size limit and eviction mutations
    #[test]
    fn test_cache_eviction_mutations() {
        let registry = CancellationRegistry::new();
        let cache_size_limit = 100; // Matches registry.max_cache_size

        // Fill cache beyond limit
        for i in 0..cache_size_limit + 20 {
            let request_id = json!(400 + i);
            let token =
                PerlLspCancellationToken::new(request_id.clone(), "eviction_test".to_string());
            registry.register_token(token).unwrap();

            // Access to populate cache
            let _ = registry.get_token(&request_id);
        }

        // Cache should have been cleared at least once - targets eviction logic
        // We can't directly test the cache size, but we can verify operations still work

        // Test that operations still work after cache eviction
        let test_id = json!(400 + cache_size_limit + 50);
        let test_token =
            PerlLspCancellationToken::new(test_id.clone(), "post_eviction".to_string());
        registry.register_token(test_token).unwrap();

        let result = registry.get_token(&test_id);
        assert!(result.is_some(), "Operations must work after cache eviction");

        // Test cancellation still works
        registry.cancel_request(&test_id).unwrap();
        assert!(registry.is_cancelled(&test_id), "Cancellation must work after cache eviction");
    }
}

/// Metrics tracking testing - targets atomic counter mutations
#[cfg(test)]
mod metrics_atomic_counter_hardening_tests {
    use super::*;

    /// Test CancellationMetrics atomic counter mutations
    #[test]
    fn test_metrics_counter_mutations() {
        let metrics = CancellationMetrics::new();

        // Test initial state - targets AtomicU64::new(0) mutations
        assert_eq!(metrics.registered_count(), 0, "Initial registered count must be 0");
        assert_eq!(metrics.cancelled_count(), 0, "Initial cancelled count must be 0");
        assert_eq!(metrics.completed_count(), 0, "Initial completed count must be 0");

        // Test increment operations - targets fetch_add(1, Ordering::Relaxed) mutations
        metrics.increment_registered();
        assert_eq!(metrics.registered_count(), 1, "Registered count must increment to 1");

        metrics.increment_cancelled();
        assert_eq!(metrics.cancelled_count(), 1, "Cancelled count must increment to 1");

        metrics.increment_completed();
        assert_eq!(metrics.completed_count(), 1, "Completed count must increment to 1");

        // Test multiple increments - targets atomic accumulation
        for i in 2..=10 {
            metrics.increment_registered();
            assert_eq!(metrics.registered_count(), i, "Registered count must increment to {}", i);
        }

        // Test independent counter mutations
        let initial_cancelled = metrics.cancelled_count();
        let initial_completed = metrics.completed_count();

        metrics.increment_registered();
        assert_eq!(
            metrics.cancelled_count(),
            initial_cancelled,
            "Cancelled count must not change when incrementing registered"
        );
        assert_eq!(
            metrics.completed_count(),
            initial_completed,
            "Completed count must not change when incrementing registered"
        );
    }

    /// Test metrics atomic load operations mutations
    #[test]
    fn test_metrics_load_operations_mutations() {
        let metrics = CancellationMetrics::new();

        // Increment counters to specific values
        for _ in 0..5 {
            metrics.increment_registered();
        }
        for _ in 0..3 {
            metrics.increment_cancelled();
        }
        for _ in 0..7 {
            metrics.increment_completed();
        }

        // Test load operations - targets load(Ordering::Relaxed) mutations
        assert_eq!(metrics.registered_count(), 5, "Registered load must return 5");
        assert_eq!(metrics.cancelled_count(), 3, "Cancelled load must return 3");
        assert_eq!(metrics.completed_count(), 7, "Completed load must return 7");

        // Test load consistency - multiple loads should return same value
        for _ in 0..10 {
            assert_eq!(metrics.registered_count(), 5, "Multiple loads must be consistent");
            assert_eq!(metrics.cancelled_count(), 3, "Multiple loads must be consistent");
            assert_eq!(metrics.completed_count(), 7, "Multiple loads must be consistent");
        }
    }

    /// Test concurrent metrics operations mutations
    #[test]
    fn test_concurrent_metrics_mutations() {
        let metrics = Arc::new(CancellationMetrics::new());
        let num_threads = 10;
        let increments_per_thread = 100;

        let mut handles = Vec::new();

        // Spawn threads that increment different counters concurrently
        for thread_id in 0..num_threads {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..increments_per_thread {
                    match thread_id % 3 {
                        0 => metrics_clone.increment_registered(),
                        1 => metrics_clone.increment_cancelled(),
                        2 => metrics_clone.increment_completed(),
                        _ => unreachable!(),
                    }

                    // Occasionally read values to test concurrent load/store
                    if thread_id % 10 == 0 {
                        let _ = metrics_clone.registered_count();
                        let _ = metrics_clone.cancelled_count();
                        let _ = metrics_clone.completed_count();
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Metrics thread should not panic");
        }

        // Calculate expected counts
        let threads_per_counter = num_threads / 3 + if num_threads % 3 > 0 { 1 } else { 0 };
        let expected_registered = if num_threads > 0 {
            (num_threads - num_threads / 3 - num_threads / 3) * increments_per_thread
        } else {
            0
        };
        let expected_cancelled = (num_threads / 3) * increments_per_thread;
        let expected_completed = (num_threads / 3) * increments_per_thread;

        // Verify final counts - targets atomic accumulation correctness
        let final_registered = metrics.registered_count();
        let final_cancelled = metrics.cancelled_count();
        let final_completed = metrics.completed_count();

        // We calculate expected more simply: threads 0,3,6,9 increment registered, etc.
        let registered_threads = (0..num_threads).filter(|&i| i % 3 == 0).count();
        let cancelled_threads = (0..num_threads).filter(|&i| i % 3 == 1).count();
        let completed_threads = (0..num_threads).filter(|&i| i % 3 == 2).count();

        assert_eq!(
            final_registered,
            registered_threads as u64 * increments_per_thread as u64,
            "Concurrent registered increments must be correct"
        );
        assert_eq!(
            final_cancelled,
            cancelled_threads as u64 * increments_per_thread as u64,
            "Concurrent cancelled increments must be correct"
        );
        assert_eq!(
            final_completed,
            completed_threads as u64 * increments_per_thread as u64,
            "Concurrent completed increments must be correct"
        );

        println!(
            "Concurrent metrics final counts: registered={}, cancelled={}, completed={}",
            final_registered, final_cancelled, final_completed
        );
    }

    /// Test registry metrics integration mutations
    #[test]
    fn test_registry_metrics_integration_mutations() {
        let registry = CancellationRegistry::new();
        let initial_metrics = registry.metrics();

        // Test initial metrics state
        assert_eq!(initial_metrics.registered_count(), 0, "Initial registered count must be 0");
        assert_eq!(initial_metrics.cancelled_count(), 0, "Initial cancelled count must be 0");
        assert_eq!(initial_metrics.completed_count(), 0, "Initial completed count must be 0");

        // Register a token - should increment registered counter
        let token = PerlLspCancellationToken::new(json!(500), "metrics_test".to_string());
        registry.register_token(token.clone()).unwrap();

        assert_eq!(
            initial_metrics.registered_count(),
            1,
            "Registered count must increment after token registration"
        );

        // Cancel request - should increment cancelled counter
        registry.cancel_request(&json!(500)).unwrap();
        assert_eq!(
            initial_metrics.cancelled_count(),
            1,
            "Cancelled count must increment after request cancellation"
        );

        // Remove request - should increment completed counter
        registry.remove_request(&json!(500));
        assert_eq!(
            initial_metrics.completed_count(),
            1,
            "Completed count must increment after request removal"
        );

        // Test metrics persistence - counters should not reset
        assert_eq!(initial_metrics.registered_count(), 1, "Registered count must persist");
        assert_eq!(initial_metrics.cancelled_count(), 1, "Cancelled count must persist");
        assert_eq!(initial_metrics.completed_count(), 1, "Completed count must persist");
    }

    /// Test memory overhead calculation mutations
    #[test]
    fn test_memory_overhead_calculation_mutations() {
        let metrics = CancellationMetrics::new();

        // Test memory overhead calculation - targets arithmetic mutations
        let overhead = metrics.memory_overhead_bytes();
        assert!(overhead > 0, "Memory overhead must be positive");
        assert!(overhead < 1024 * 1024, "Memory overhead must be < 1MB");

        // Test consistency - multiple calls should return same value
        for _ in 0..10 {
            let overhead_check = metrics.memory_overhead_bytes();
            assert_eq!(overhead, overhead_check, "Memory overhead calculation must be consistent");
        }

        // Test bounds - targets size_of + buffer calculation
        let min_expected = std::mem::size_of::<CancellationMetrics>();
        assert!(
            overhead >= min_expected,
            "Memory overhead must be at least size of struct: {} >= {}",
            overhead,
            min_expected
        );

        let max_expected = min_expected + 2048; // Buffer should not exceed reasonable bounds
        assert!(
            overhead <= max_expected,
            "Memory overhead should not exceed reasonable bounds: {} <= {}",
            overhead,
            max_expected
        );
    }
}

/// Global registry testing - targets singleton pattern mutations
#[cfg(test)]
mod global_registry_hardening_tests {
    use super::*;

    /// Test GLOBAL_CANCELLATION_REGISTRY initialization mutations
    #[test]
    fn test_global_registry_initialization_mutations() {
        // Test that global registry is properly initialized - targets lazy_static! mutations
        let active_count_1 = GLOBAL_CANCELLATION_REGISTRY.active_count();
        let active_count_2 = GLOBAL_CANCELLATION_REGISTRY.active_count();

        // Multiple accesses should return consistent results
        assert_eq!(
            active_count_1, active_count_2,
            "Global registry must be consistently initialized"
        );

        // Test that global registry operations work
        let test_token = PerlLspCancellationToken::new(json!(600), "global_test".to_string());
        let register_result = GLOBAL_CANCELLATION_REGISTRY.register_token(test_token.clone());
        assert!(register_result.is_ok(), "Global registry registration must succeed");

        // Verify the token is accessible
        let retrieved = GLOBAL_CANCELLATION_REGISTRY.get_token(&json!(600));
        assert!(retrieved.is_some(), "Global registry must provide registered tokens");

        // Clean up
        GLOBAL_CANCELLATION_REGISTRY.remove_request(&json!(600));
    }

    /// Test global registry thread safety mutations
    #[test]
    fn test_global_registry_thread_safety_mutations() {
        let num_threads = 5;
        let ops_per_thread = 20;
        let mut handles = Vec::new();

        // Spawn threads that use global registry concurrently
        for thread_id in 0..num_threads {
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let request_id = json!(700 + thread_id * 100 + i);
                    let token = PerlLspCancellationToken::new(
                        request_id.clone(),
                        format!("global_thread_{}", thread_id),
                    );

                    // Register token
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token);

                    // Test operations
                    let _ = GLOBAL_CANCELLATION_REGISTRY.get_token(&request_id);
                    let _ = GLOBAL_CANCELLATION_REGISTRY.is_cancelled(&request_id);
                    let _ = GLOBAL_CANCELLATION_REGISTRY.cancel_request(&request_id);

                    // Clean up
                    GLOBAL_CANCELLATION_REGISTRY.remove_request(&request_id);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads - tests concurrent access safety
        for handle in handles {
            handle.join().expect("Global registry thread should not panic");
        }

        // Global registry should remain in consistent state
        let final_count = GLOBAL_CANCELLATION_REGISTRY.active_count();
        println!("Global registry final active count: {}", final_count);

        // Should be 0 or close to 0 since we cleaned up
        assert!(final_count < 10, "Global registry should be mostly clean after operations");
    }
}

/// Property-based testing for comprehensive mutation coverage
#[cfg(test)]
mod cancellation_property_hardening_tests {
    use super::*;

    // Import property test utilities
    use proptest::collection::vec;

    proptest! {
        /// Property: Token cancellation is atomic and consistent
        #[test]
        fn prop_token_cancellation_atomic(
            request_id in prop::num::i64::ANY,
            provider in "[a-zA-Z0-9_]{1,20}"
        ) {
            let token = PerlLspCancellationToken::new(json!(request_id), provider);

            // Initially not cancelled
            prop_assert!(!token.is_cancelled());
            prop_assert!(!token.is_cancelled_relaxed());
            prop_assert!(!token.is_cancelled_hot_path());

            // After cancellation, all methods must agree
            token.cancel();
            prop_assert!(token.is_cancelled());
            prop_assert!(token.is_cancelled_relaxed());
            prop_assert!(token.is_cancelled_hot_path());
        }

        /// Property: Registry operations preserve token state
        #[test]
        fn prop_registry_operations_preserve_state(
            request_ids in vec(prop::num::i64::ANY, 1..20)
        ) {
            let registry = CancellationRegistry::new();
            let mut tokens = Vec::new();

            // Register all tokens
            for &id in &request_ids {
                let token = PerlLspCancellationToken::new(json!(id), "prop_test".to_string());
                let _ = registry.register_token(token.clone());
                tokens.push(token);
            }

            // Verify all tokens are retrievable
            for &id in &request_ids {
                let retrieved = registry.get_token(&json!(id));
                prop_assert!(retrieved.is_some(), "Token {} must be retrievable", id);
                prop_assert!(!registry.is_cancelled(&json!(id)), "Token {} must not be cancelled initially", id);
            }

            // Cancel half the tokens
            for (i, token) in tokens.iter().enumerate() {
                if i % 2 == 0 {
                    token.cancel();
                }
            }

            // Verify cancellation state is preserved
            for (i, &id) in request_ids.iter().enumerate() {
                let should_be_cancelled = i % 2 == 0;
                let is_cancelled = registry.is_cancelled(&json!(id));
                prop_assert_eq!(is_cancelled, should_be_cancelled,
                              "Token {} cancellation state mismatch", id);
            }
        }

        /// Property: Metrics counters are monotonic and accurate
        #[test]
        fn prop_metrics_monotonic_accurate(
            operations in vec(prop::num::u8::ANY, 1..100)
        ) {
            let registry = CancellationRegistry::new();
            let metrics = registry.metrics();

            let mut expected_registered = 0u64;
            let mut expected_cancelled = 0u64;
            let mut expected_completed = 0u64;

            for (i, &op) in operations.iter().enumerate() {
                let request_id = json!(i);
                let token = PerlLspCancellationToken::new(request_id.clone(), "prop_metrics".to_string());

                match op % 4 {
                    0 => {
                        // Register operation
                        let _ = registry.register_token(token);
                        expected_registered += 1;
                    },
                    1 => {
                        // Cancel operation (only if registered)
                        let _ = registry.register_token(token);
                        let _ = registry.cancel_request(&request_id);
                        expected_registered += 1;
                        expected_cancelled += 1;
                    },
                    2 => {
                        // Complete operation
                        let _ = registry.register_token(token);
                        registry.remove_request(&request_id);
                        expected_registered += 1;
                        expected_completed += 1;
                    },
                    3 => {
                        // Combined operation
                        let _ = registry.register_token(token);
                        let _ = registry.cancel_request(&request_id);
                        registry.remove_request(&request_id);
                        expected_registered += 1;
                        expected_cancelled += 1;
                        expected_completed += 1;
                    },
                    _ => unreachable!(),
                }
            }

            // Verify final counts match expected
            prop_assert_eq!(metrics.registered_count(), expected_registered, "Registered count mismatch");
            prop_assert_eq!(metrics.cancelled_count(), expected_cancelled, "Cancelled count mismatch");
            prop_assert_eq!(metrics.completed_count(), expected_completed, "Completed count mismatch");
        }
    }
}

// Import proptest for property-based tests
use proptest::test_runner::{Config, RngAlgorithm};

/// Helper to configure proptest for more thorough testing
fn proptest_config() -> Config {
    Config {
        cases: 1000, // Increase test cases for better mutation coverage
        max_shrink_iters: 10000,
        rng_algorithm: RngAlgorithm::ChaCha,
        ..Config::default()
    }
}
