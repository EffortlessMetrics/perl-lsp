//! Comprehensive tests for Phase 2 production readiness features.
//!
//! Tests for:
//! - Index lifecycle state machine
//! - Bounded caches with LRU eviction
//! - Service Level Objectives (SLOs)
//! - Performance optimization for large workspaces
//! - Production coordinator integration

use perl_workspace_index::workspace::cache::{
    AstCacheConfig, BoundedLruCache, CacheConfig, EstimateSize, SymbolCacheConfig,
    WorkspaceCacheConfig, CombinedWorkspaceCacheConfig,
};
use perl_workspace_index::workspace::slo::{
    OperationResult, OperationType, SloConfig, SloStatistics, SloTracker,
};
use perl_workspace_index::workspace::state_machine::{
    BuildPhase, DegradationReason, IndexState, IndexStateMachine, IndexStateKind,
    InvalidationReason, ResourceKind, TransitionResult,
};
use perl_workspace_index::workspace::production_coordinator::{
    ProductionIndexCoordinator, ProductionCoordinatorConfig, CoordinatorStatistics,
};
use std::thread;
use std::time::Duration;
use url::Url;

// ============================================================================
// State Machine Tests
// ============================================================================

#[test]
fn test_state_machine_initial_state() {
    let machine = IndexStateMachine::new();
    let state = machine.state();

    assert!(matches!(state, IndexState::Idle { .. }));
    assert!(!state.is_ready());
    assert!(!state.is_error());
    assert!(!state.is_transitional());
}

#[test]
fn test_state_machine_full_lifecycle() {
    let machine = IndexStateMachine::new();

    // Idle → Initializing
    assert!(matches!(
        machine.transition_to_initializing(),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Initializing { .. }));
    assert!(machine.state().is_transitional());

    // Update initialization progress
    assert!(matches!(
        machine.update_initialization_progress(50),
        TransitionResult::Success
    ));

    // Initializing → Building
    assert!(matches!(
        machine.transition_to_building(100),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Building { .. }));

    // Update building progress
    assert!(matches!(
        machine.update_building_progress(50, BuildPhase::Indexing),
        TransitionResult::Success
    ));

    // Building → Ready
    assert!(matches!(
        machine.transition_to_ready(100, 5000),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Ready { .. }));
    assert!(machine.state().is_ready());
    assert!(!machine.state().is_transitional());
}

#[test]
fn test_state_machine_degradation() {
    let machine = IndexStateMachine::new();

    // Build to Ready state
    machine.transition_to_initializing();
    machine.transition_to_building(100);
    machine.transition_to_ready(100, 5000);

    // Ready → Degraded
    assert!(matches!(
        machine.transition_to_degraded(DegradationReason::IoError {
            message: "IO error".to_string()
        }),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Degraded { .. }));

    // Degraded → Ready (recovery)
    assert!(matches!(
        machine.transition_to_ready(100, 5000),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Ready { .. }));
}

#[test]
fn test_state_machine_error_recovery() {
    let machine = IndexStateMachine::new();

    // Any state → Error
    assert!(matches!(
        machine.transition_to_error("Critical error".to_string()),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Error { .. }));
    assert!(machine.state().is_error());

    // Error → Initializing (recovery)
    assert!(matches!(
        machine.transition_to_initializing(),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Initializing { .. }));
}

#[test]
fn test_state_machine_invalid_transitions() {
    let machine = IndexStateMachine::new();

    // Can't go from Idle to Ready without building
    assert!(matches!(
        machine.transition_to_ready(0, 0),
        TransitionResult::InvalidTransition { .. }
    ));

    // Can't go from Idle to Updating
    assert!(matches!(
        machine.transition_to_updating(5),
        TransitionResult::InvalidTransition { .. }
    ));
}

#[test]
fn test_state_machine_concurrent_access() {
    let machine = IndexStateMachine::new();
    let machine_arc = std::sync::Arc::new(machine);

    // Spawn multiple threads accessing the state machine
    let mut handles = vec![];

    for i in 0..10 {
        let machine_clone = std::sync::Arc::clone(&machine_arc);
        let handle = thread::spawn(move || {
            let _state = machine_clone.state();
            // Simulate some work
            thread::sleep(Duration::from_millis(1));
            if i == 0 {
                machine_clone.transition_to_initializing();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // State machine should still be functional
    let state = machine_arc.state();
    assert!(!matches!(state, IndexState::Error { .. }));
}

#[test]
fn test_state_machine_update_state() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(100);

    // Ready → Updating
    machine.transition_to_ready(100, 5000);
    assert!(matches!(
        machine.transition_to_updating(5),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Updating { .. }));

    // Updating → Ready
    assert!(matches!(
        machine.transition_to_ready(100, 5000),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Ready { .. }));
}

#[test]
fn test_state_machine_invalidation() {
    let machine = IndexStateMachine::new();
    machine.transition_to_initializing();
    machine.transition_to_building(100);
    machine.transition_to_ready(100, 5000);

    // Ready → Invalidating
    assert!(matches!(
        machine.transition_to_invalidating(InvalidationReason::ManualRequest),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Invalidating { .. }));

    // Invalidating → Ready
    assert!(matches!(
        machine.transition_to_ready(100, 5000),
        TransitionResult::Success
    ));
    assert!(matches!(machine.state(), IndexState::Ready { .. }));
}

// ============================================================================
// Cache Tests
// ============================================================================

#[test]
fn test_cache_basic_operations() {
    let cache = BoundedLruCache::<String, String>::default();

    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());

    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
    assert_eq!(cache.get(&"key3".to_string()), None);
}

#[test]
fn test_cache_lru_eviction() {
    let config = CacheConfig {
        max_items: 2,
        max_bytes: 100,
        ttl: None,
    };
    let cache = BoundedLruCache::<String, String>::new(config);

    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());

    // Access key1 to make it more recent
    cache.get(&"key1".to_string());

    // Insert key3 - should evict key2 (LRU)
    cache.insert("key3".to_string(), "value3".to_string());

    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    assert_eq!(cache.get(&"key2".to_string()), None); // Evicted
    assert_eq!(cache.get(&"key3".to_string()), Some("value3".to_string()));
}

#[test]
fn test_cache_memory_limit() {
    let config = CacheConfig {
        max_items: 1000,
        max_bytes: 20, // Very small limit
        ttl: None,
    };
    let cache = BoundedLruCache::<String, String>::new(config);

    // First insert should succeed
    assert!(cache.insert_with_size("key1".to_string(), "value1".to_string(), 10));

    // Second insert should fail due to memory limit
    assert!(!cache.insert_with_size("key2".to_string(), "value2".to_string(), 15));
}

#[test]
fn test_cache_stats() {
    let cache = BoundedLruCache::<String, String>::default();

    cache.insert("key1".to_string(), "value1".to_string());

    cache.get(&"key1".to_string()); // Hit
    cache.get(&"key2".to_string()); // Miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hit_rate, 0.5);
}

#[test]
fn test_cache_clear() {
    let cache = BoundedLruCache::<String, String>::default();

    cache.insert("key1".to_string(), "value1".to_string());
    cache.clear();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.get(&"key1".to_string()), None);
}

#[test]
fn test_cache_remove() {
    let cache = BoundedLruCache::<String, String>::default();

    cache.insert("key1".to_string(), "value1".to_string());
    assert_eq!(cache.remove(&"key1".to_string()), Some("value1".to_string()));
    assert_eq!(cache.get(&"key1".to_string()), None);
}

#[test]
fn test_cache_concurrent_access() {
    let cache = BoundedLruCache::<String, String>::default();
    let cache_arc = std::sync::Arc::new(cache);

    let mut handles = vec![];

    for i in 0..10 {
        let cache_clone = std::sync::Arc::clone(&cache_arc);
        let handle = thread::spawn(move || {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            cache_clone.insert(key, value);
            cache_clone.get(&key);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Cache should still be functional
    assert!(!cache_arc.is_empty());
}

#[test]
fn test_cache_ttl_expiration() {
    let config = CacheConfig {
        max_items: 100,
        max_bytes: 10_000,
        ttl: Some(Duration::from_millis(10)), // Short TTL
    };
    let cache = BoundedLruCache::<String, String>::new(config);

    cache.insert("key1".to_string(), "value1".to_string());

    // Should be available immediately
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

    // Wait for TTL to expire
    thread::sleep(Duration::from_millis(20));

    // Should be expired now
    assert_eq!(cache.get(&"key1".to_string()), None);
}

#[test]
fn test_cache_configs() {
    let ast_config = AstCacheConfig::default();
    assert_eq!(ast_config.max_nodes, 10_000);
    assert_eq!(ast_config.max_bytes, 50 * 1024 * 1024);

    let symbol_config = SymbolCacheConfig::default();
    assert_eq!(symbol_config.max_symbols, 50_000);
    assert_eq!(symbol_config.max_bytes, 30 * 1024 * 1024);

    let workspace_config = WorkspaceCacheConfig::default();
    assert_eq!(workspace_config.max_files, 1_000);
    assert_eq!(workspace_config.max_bytes, 20 * 1024 * 1024);

    let combined_config = CombinedWorkspaceCacheConfig::default();
    assert_eq!(combined_config.ast.max_nodes, 10_000);
    assert_eq!(combined_config.symbol.max_symbols, 50_000);
    assert_eq!(combined_config.workspace.max_files, 1_000);
}

// ============================================================================
// SLO Tests
// ============================================================================

#[test]
fn test_slo_tracker_basic() {
    let tracker = SloTracker::default();

    let start = tracker.start_operation(OperationType::DefinitionLookup);
    thread::sleep(Duration::from_millis(1));
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        start,
        OperationResult::Success,
    );

    let stats = tracker.statistics(OperationType::DefinitionLookup);
    assert_eq!(stats.total_count, 1);
    assert_eq!(stats.success_count, 1);
    assert_eq!(stats.failure_count, 0);
}

#[test]
fn test_slo_tracker_statistics() {
    let tracker = SloTracker::default();

    // Record some operations
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
    assert!(stats.p50_ms > 0);
    assert!(stats.p95_ms > 0);
    assert!(stats.avg_ms > 0);
}

#[test]
fn test_slo_tracker_error_tracking() {
    let tracker = SloTracker::default();

    // Record some successful operations
    for _ in 0..9 {
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(
            OperationType::DefinitionLookup,
            start,
            OperationResult::Success,
        );
    }

    // Record one failure
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        start,
        OperationResult::Failure("error".to_string()),
    );

    let stats = tracker.statistics(OperationType::DefinitionLookup);
    assert_eq!(stats.total_count, 10);
    assert_eq!(stats.success_count, 9);
    assert_eq!(stats.failure_count, 1);
    assert_eq!(stats.error_rate, 0.1);
}

#[test]
fn test_slo_tracker_all_operations() {
    let tracker = SloTracker::default();

    // Record different operation types
    for op_type in [
        OperationType::IndexInitialization,
        OperationType::IncrementalUpdate,
        OperationType::DefinitionLookup,
        OperationType::Completion,
        OperationType::Hover,
    ] {
        let start = tracker.start_operation(op_type);
        thread::sleep(Duration::from_millis(1));
        tracker.record_operation_type(op_type, start, OperationResult::Success);
    }

    let all_stats = tracker.all_statistics();
    assert_eq!(all_stats.len(), 8); // All operation types

    for (op_type, stats) in all_stats {
        if matches!(
            op_type,
            OperationType::IndexInitialization
                | OperationType::IncrementalUpdate
                | OperationType::DefinitionLookup
                | OperationType::Completion
                | OperationType::Hover
        ) {
            assert_eq!(stats.total_count, 1);
        }
    }
}

#[test]
fn test_slo_tracker_reset() {
    let tracker = SloTracker::default();

    let start = tracker.start_operation(OperationType::DefinitionLookup);
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        start,
        OperationResult::Success,
    );

    tracker.reset();

    let stats = tracker.statistics(OperationType::DefinitionLookup);
    assert_eq!(stats.total_count, 0);
}

#[test]
fn test_slo_config() {
    let config = SloConfig::default();
    assert_eq!(config.index_init_p95_ms, 5000);
    assert_eq!(config.incremental_update_p95_ms, 100);
    assert_eq!(config.definition_lookup_p95_ms, 50);
    assert_eq!(config.completion_p95_ms, 100);
    assert_eq!(config.hover_p95_ms, 50);
    assert_eq!(config.max_error_rate, 0.01);
}

#[test]
fn test_slo_met() {
    let tracker = SloTracker::default();

    // Record fast operations (should meet SLO)
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
    assert!(stats.slo_met);
}

#[test]
fn test_slo_all_slos_met() {
    let tracker = SloTracker::default();

    // Record fast operations for all types
    for op_type in [
        OperationType::IndexInitialization,
        OperationType::IncrementalUpdate,
        OperationType::DefinitionLookup,
        OperationType::Completion,
        OperationType::Hover,
    ] {
        for _ in 0..5 {
            let start = tracker.start_operation(op_type);
            thread::sleep(Duration::from_millis(1));
            tracker.record_operation_type(op_type, start, OperationResult::Success);
        }
    }

    assert!(tracker.all_slos_met());
}

#[test]
fn test_operation_type_targets() {
    let config = SloConfig::default();

    assert_eq!(
        OperationType::IndexInitialization.slo_target_ms(&config),
        5000
    );
    assert_eq!(
        OperationType::IncrementalUpdate.slo_target_ms(&config),
        100
    );
    assert_eq!(
        OperationType::DefinitionLookup.slo_target_ms(&config),
        50
    );
    assert_eq!(OperationType::Completion.slo_target_ms(&config), 100);
    assert_eq!(OperationType::Hover.slo_target_ms(&config), 50);
}

#[test]
fn test_operation_type_names() {
    assert_eq!(
        OperationType::IndexInitialization.name(),
        "index_initialization"
    );
    assert_eq!(OperationType::IncrementalUpdate.name(), "incremental_update");
    assert_eq!(OperationType::DefinitionLookup.name(), "definition_lookup");
    assert_eq!(OperationType::Completion.name(), "completion");
    assert_eq!(OperationType::Hover.name(), "hover");
}

#[test]
fn test_operation_result() {
    assert!(OperationResult::Success.is_success());
    assert!(!OperationResult::Failure("error".to_string()).is_success());

    let result: OperationResult = Ok::<(), String>(()).into();
    assert!(result.is_success());

    let result: OperationResult = Err::<(), String>("error".to_string()).into();
    assert!(!result.is_success());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_state_machine_cache_integration() {
    let machine = IndexStateMachine::new();
    let cache = BoundedLruCache::<String, String>::default();

    // Simulate index building with caching
    machine.transition_to_initializing();
    cache.insert("init".to_string(), "initialized".to_string());

    machine.transition_to_building(100);
    for i in 0..10 {
        cache.insert(format!("file{}", i), format!("content{}", i));
    }

    machine.transition_to_ready(10, 100);

    assert!(machine.state().is_ready());
    assert_eq!(cache.len(), 11);
}

#[test]
fn test_state_machine_slo_integration() {
    let machine = IndexStateMachine::new();
    let tracker = SloTracker::default();

    // Track index initialization
    let start = tracker.start_operation(OperationType::IndexInitialization);
    machine.transition_to_initializing();
    machine.transition_to_building(100);
    machine.transition_to_ready(100, 5000);
    tracker.record_operation_type(
        OperationType::IndexInitialization,
        start,
        OperationResult::Success,
    );

    // Track definition lookup
    let start = tracker.start_operation(OperationType::DefinitionLookup);
    thread::sleep(Duration::from_millis(1));
    tracker.record_operation_type(
        OperationType::DefinitionLookup,
        start,
        OperationResult::Success,
    );

    assert!(machine.state().is_ready());
    assert!(tracker.all_slos_met());
}

#[test]
fn test_full_production_readiness_workflow() {
    let machine = IndexStateMachine::new();
    let cache = BoundedLruCache::<String, String>::default();
    let tracker = SloTracker::default();

    // 1. Initialize
    let start = tracker.start_operation(OperationType::IndexInitialization);
    machine.transition_to_initializing();
    machine.update_initialization_progress(50);
    tracker.record_operation_type(
        OperationType::IndexInitialization,
        start,
        OperationResult::Success,
    );

    // 2. Build index
    let start = tracker.start_operation(OperationType::FileIndexing);
    machine.transition_to_building(100);
    for i in 0..10 {
        cache.insert(format!("file{}", i), format!("content{}", i));
        machine.update_building_progress(i + 1, BuildPhase::Indexing);
    }
    tracker.record_operation_type(
        OperationType::FileIndexing,
        start,
        OperationResult::Success,
    );

    // 3. Ready state
    machine.transition_to_ready(10, 100);

    // 4. Perform queries
    for _ in 0..5 {
        let start = tracker.start_operation(OperationType::DefinitionLookup);
        cache.get(&"file0".to_string());
        tracker.record_operation_type(
            OperationType::DefinitionLookup,
            start,
            OperationResult::Success,
        );
    }

    // 5. Update
    let start = tracker.start_operation(OperationType::IncrementalUpdate);
    machine.transition_to_updating(1);
    cache.insert("file10".to_string(), "content10".to_string());
    tracker.record_operation_type(
        OperationType::IncrementalUpdate,
        start,
        OperationResult::Success,
    );
    machine.transition_to_ready(11, 110);

    // Verify everything is working
    assert!(machine.state().is_ready());
    assert_eq!(cache.len(), 11);
    assert!(tracker.all_slos_met());
}

// ============================================================================
// Production Coordinator Tests
// ============================================================================

#[test]
fn test_production_coordinator_creation() {
    let coordinator = ProductionIndexCoordinator::new();
    assert!(matches!(coordinator.state(), IndexState::Idle { .. }));
    
    let stats = coordinator.statistics();
    assert_eq!(stats.cache_stats.len(), 3);
    assert_eq!(stats.slo_stats.len(), 8);
}

#[test]
fn test_production_coordinator_initialization() {
    let coordinator = ProductionIndexCoordinator::new();
    assert!(coordinator.initialize().is_ok());
    assert!(coordinator.state().is_ready());
}

#[test]
fn test_production_coordinator_file_indexing() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let code = "sub hello { return 42; }";
    assert!(coordinator.index_file(uri, code.to_string()).is_ok());
}

#[test]
fn test_production_coordinator_definition_lookup() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let code = "sub hello { return 42; }";
    coordinator.index_file(uri, code.to_string()).unwrap();

    let def = coordinator.find_definition("hello");
    assert!(def.is_some());
}

#[test]
fn test_production_coordinator_references() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let code = "sub hello { return 42; } hello();";
    coordinator.index_file(uri, code.to_string()).unwrap();

    let refs = coordinator.find_references("hello");
    assert!(!refs.is_empty());
}

#[test]
fn test_production_coordinator_caching() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let code = "sub hello { return 42; }";
    
    // First indexing - should cache
    assert!(coordinator.index_file(uri.clone(), code.to_string()).is_ok());
    
    // Second indexing - should hit cache
    assert!(coordinator.index_file(uri, code.to_string()).is_ok());
    
    // Check cache stats
    let stats = coordinator.statistics();
    let ast_stats = stats.cache_stats.get("ast").unwrap();
    assert!(ast_stats.hits > 0);
}

#[test]
fn test_production_coordinator_slo_tracking() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let code = "sub hello { return 42; }";
    coordinator.index_file(uri, code.to_string()).unwrap();

    let def = coordinator.find_definition("hello");
    assert!(def.is_some());

    // Check SLO stats
    let stats = coordinator.statistics();
    assert!(stats.all_slos_met);
    
    let file_stats = stats.slo_stats.get(&OperationType::FileIndexing).unwrap();
    assert_eq!(file_stats.total_count, 1);
    assert_eq!(file_stats.success_count, 1);
    
    let def_stats = stats.slo_stats.get(&OperationType::DefinitionLookup).unwrap();
    assert_eq!(def_stats.total_count, 1);
    assert_eq!(def_stats.success_count, 1);
}

#[test]
fn test_production_coordinator_invalidation() {
    let coordinator = ProductionIndexCoordinator::new();
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    coordinator.index_file(uri, "sub hello {}".to_string()).unwrap();

    coordinator.invalidate(InvalidationReason::ManualRequest);
    assert!(matches!(coordinator.state(), IndexState::Idle { .. }));
    
    // Cache should be cleared
    let stats = coordinator.statistics();
    let ast_stats = stats.cache_stats.get("ast").unwrap();
    assert_eq!(ast_stats.current_items, 0);
}

#[test]
fn test_production_coordinator_memory_limits() {
    let mut config = ProductionCoordinatorConfig::default();
    config.cache_config.ast.max_bytes = 100; // Very small limit
    
    let coordinator = ProductionIndexCoordinator::with_config(config);
    coordinator.initialize().unwrap();

    let uri = Url::parse("file:///example.pl").unwrap();
    let large_code = "x".repeat(200); // Larger than cache limit
    
    // Should still work, but cache eviction will occur
    assert!(coordinator.index_file(uri, large_code).is_ok());
    
    let stats = coordinator.statistics();
    assert!(stats.total_memory_usage <= 100); // Should respect limit
}

#[test]
fn test_production_coordinator_concurrent_operations() {
    let coordinator = std::sync::Arc::new(ProductionIndexCoordinator::new());
    coordinator.initialize().unwrap();

    let mut handles = vec![];

    for i in 0..10 {
        let coord_clone = std::sync::Arc::clone(&coordinator);
        let handle = thread::spawn(move || {
            let uri = Url::parse(&format!("file:///example{}.pl", i)).unwrap();
            let code = format!("sub hello{} {{ return {}; }}", i, i);
            coord_clone.index_file(uri, code).unwrap();
            
            let def = coord_clone.find_definition(&format!("hello{}", i));
            assert!(def.is_some());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All operations should have completed successfully
    let stats = coordinator.statistics();
    assert!(stats.all_slos_met);
    assert!(stats.cache_stats.get("ast").unwrap().current_items > 0);
}

#[test]
fn test_production_coordinator_full_workflow() {
    let coordinator = ProductionIndexCoordinator::new();
    
    // 1. Initialize
    assert!(coordinator.initialize().is_ok());
    assert!(coordinator.state().is_ready());
    
    // 2. Index multiple files
    let files = vec![
        ("file:///main.pl", "sub main { hello(); }"),
        ("file:///utils.pl", "sub hello { return 42; }"),
        ("file:///config.pl", "our $CONFIG = 1;"),
    ];
    
    for (uri_str, code) in &files {
        let uri = Url::parse(uri_str).unwrap();
        assert!(coordinator.index_file(uri, code.to_string()).is_ok());
    }
    
    // 3. Perform lookups
    let main_def = coordinator.find_definition("main");
    assert!(main_def.is_some());
    
    let hello_def = coordinator.find_definition("hello");
    assert!(hello_def.is_some());
    
    let hello_refs = coordinator.find_references("hello");
    assert_eq!(hello_refs.len(), 2); // definition + call
    
    // 4. Check statistics
    let stats = coordinator.statistics();
    assert!(stats.all_slos_met);
    assert_eq!(stats.cache_stats.get("ast").unwrap().current_items, 3);
    assert_eq!(stats.cache_stats.get("symbol").unwrap().current_items, 3);
    
    // 5. Invalidate and reinitialize
    coordinator.invalidate(InvalidationReason::ConfigurationChanged);
    assert!(matches!(coordinator.state(), IndexState::Idle { .. }));
    
    assert!(coordinator.initialize().is_ok());
    assert!(coordinator.state().is_ready());
}
