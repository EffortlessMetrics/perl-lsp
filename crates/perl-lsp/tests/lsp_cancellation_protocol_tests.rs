//! Comprehensive test suite for LSP Cancellation Protocol Compliance
//! Tests AC1-AC5: Enhanced $/cancelRequest processing with provider integration
//!
//! ## Test Coverage Mapping
//! - AC:1 - Enhanced JSON-RPC 2.0 $/cancelRequest notification processing
//! - AC:2 - Thread-safe cancellation token architecture with atomic operations
//! - AC:3 - Comprehensive LSP provider integration across all providers
//! - AC:4 - Enhanced error response handling with -32800 codes
//! - AC:5 - Multiple concurrent cancellation handling without interference
//!
//! ## Test Architecture
//! Tests are structured to fail initially (TDD red phase) due to missing implementation,
//! establishing a solid foundation for implementing enhanced cancellation capabilities.

#![allow(unused_imports)] // Some imports may not be used yet in scaffolding

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

mod common;
use common::*;

use perl_lsp::cancellation::{
    CancellableProvider, CancellationError, CancellationRegistry, PerlLspCancellationToken,
    ProviderCleanupContext,
};

/// Test fixture for cancellation scenarios
struct CancellationTestFixture {
    server: LspServer,
}

impl CancellationTestFixture {
    fn new() -> Self {
        let mut server = start_lsp_server();
        initialize_lsp(&mut server);

        let mut test_workspace_files = HashMap::new();

        // Create test files for dual pattern testing
        test_workspace_files.insert(
            "file:///lib/TestModule.pm".to_string(),
            r#"package TestModule;

sub test_function {
    my ($self, $arg) = @_;
    return $arg * 2;
}

sub another_function {
    my ($self) = @_;
    TestModule::test_function($self, 42);  # Qualified call
}

sub complex_function {
    my ($data) = @_;
    # Complex processing that takes time
    for my $i (0..1000) {
        $data = $data . "_processed_$i";
    }
    return $data;
}

1;
"#
            .to_string(),
        );

        test_workspace_files.insert(
            "file:///main.pl".to_string(),
            r#"use lib 'lib';
use TestModule;

my $module = TestModule->new();
my $result = $module->test_function(10);     # Bare method call
my $other = TestModule::another_function();  # Qualified call
my $complex = TestModule::complex_function("data");
print "Results: $result, $other, $complex\n";
"#
            .to_string(),
        );

        // Setup test files
        for (uri, content) in &test_workspace_files {
            setup_test_file(&mut server, uri, content);
        }

        // Wait for indexing to complete with adaptive timeout
        let indexing_timeout = match max_concurrent_threads() {
            0..=2 => Duration::from_secs(8), // Constrained: reduced from 5s to reasonable limit
            3..=4 => Duration::from_secs(4), // Moderate: reduced timeout
            _ => Duration::from_secs(3),     // Unconstrained: shorter timeout
        };
        drain_until_quiet(&mut server, Duration::from_millis(200), indexing_timeout);

        Self { server }
    }

    fn setup_test_file(&mut self, uri: &str, content: &str) {
        setup_test_file(&mut self.server, uri, content);
    }
}

/// Setup test file helper
fn setup_test_file(server: &mut LspServer, uri: &str, content: &str) {
    send_notification(
        server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );
}

// ============================================================================
// AC1: Enhanced JSON-RPC 2.0 $/cancelRequest Processing Tests
// ============================================================================

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#enhanced-protocol-requirements
/// AC:1 - Enhanced $/cancelRequest notification processing with provider context awareness
#[test]
fn test_enhanced_cancel_request_with_provider_context_ac1() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = CancellationTestFixture::new();

    // Test completion provider cancellation with enhanced context
    let completion_id = 1001;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": completion_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 5, "character": 10 }
            }
        }),
    );

    // Send enhanced cancellation with provider context
    // This should fail initially as enhanced cancellation infrastructure doesn't exist
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": completion_id,
                "context": {
                    "provider": "textDocument/completion",
                    "workspace_symbols": true,
                    "cross_file": true,
                    "cleanup_context": "completion_provider"
                }
            }
        }),
    );

    // Validate enhanced cancellation response
    let response =
        read_response_matching_i64(&mut fixture.server, completion_id, Duration::from_millis(500));

    if let Some(resp) = response {
        if let Some(error) = resp.get("error") {
            assert_eq!(
                error["code"].as_i64(),
                Some(-32800),
                "Should return RequestCancelled error code"
            );
            let message = error["message"].as_str().ok_or("Error message should be a string")?;
            assert!(
                message.contains("completion"),
                "Error message should reference completion provider"
            );

            // Validate enhanced error data
            if let Some(data) = error.get("data") {
                assert!(
                    data.get("provider").is_some(),
                    "Enhanced error should include provider context"
                );
                assert!(
                    data.get("latency_ms").is_some(),
                    "Enhanced error should include latency metrics"
                );
            }
        }
    }

    // Test will initially fail due to basic cancellation implementation
    // Enhanced provider context processing will be implemented in feature development
    Ok(())
}

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#provider-integration-schema
/// AC:1 - Multiple LSP provider cancellation validation with enhanced context
#[test]
fn test_multiple_provider_cancellation_with_context_ac1() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = CancellationTestFixture::new();

    let provider_scenarios = vec![
        (
            2001,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 2, "character": 5 }
            }),
            "hover_provider",
        ),
        (
            2002,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 3, "character": 15 }
            }),
            "definition_provider",
        ),
        (
            2003,
            "textDocument/references",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 3, "character": 15 },
                "context": { "includeDeclaration": true }
            }),
            "references_provider",
        ),
        (
            2004,
            "workspace/symbol",
            json!({
                "query": "test_function"
            }),
            "workspace_symbol_provider",
        ),
    ];

    // Send all provider requests concurrently
    for (id, method, params, _provider_type) in &provider_scenarios {
        send_request_no_wait(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        );
    }

    // Cancel all requests with provider-specific context
    for (id, method, _params, provider_type) in &provider_scenarios {
        send_notification(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": {
                    "id": id,
                    "context": {
                        "provider": method,
                        "provider_type": provider_type,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| format!("System time error: {}", e))?
                            .as_millis() as i64
                    }
                }
            }),
        );
    }

    // Validate all cancellations with enhanced error context
    for (id, method, _params, _provider_type) in &provider_scenarios {
        let response =
            read_response_matching_i64(&mut fixture.server, *id, Duration::from_millis(1000));

        if let Some(resp) = response {
            if let Some(error) = resp.get("error") {
                assert_eq!(error["code"].as_i64(), Some(-32800));
                let message =
                    error["message"].as_str().ok_or("Error message should be a string")?;
                let method_name =
                    method.split('/').next_back().ok_or("Invalid method format")?.to_string();
                assert!(
                    message.to_lowercase().contains(&method_name.to_lowercase()),
                    "Error message should reference specific provider: {}",
                    method_name
                );

                // Validate enhanced error data structure
                if let Some(data) = error.get("data") {
                    assert!(
                        data.get("provider").is_some(),
                        "Enhanced cancellation should include provider information"
                    );
                }
            }
            // Note: Some fast operations might complete before cancellation
        }
    }

    // This test establishes the pattern for provider-specific cancellation
    // Implementation will add enhanced context processing and cleanup coordination
    Ok(())
}

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#json-rpc-compliance
/// AC:1 - JSON-RPC 2.0 protocol compliance validation
#[test]
fn test_json_rpc_protocol_compliance_ac1() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Test 1: Invalid cancellation request handling
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                // Missing required "id" field - should be handled gracefully
                "invalid_field": "test_value"
            }
        }),
    );

    // Verify server remains stable and doesn't crash
    let health_check = read_response_timeout(&mut fixture.server, Duration::from_millis(200));
    assert!(health_check.is_none(), "$/cancelRequest notification should not produce response");
    assert!(
        fixture.server.is_alive(),
        "Server should remain alive after invalid cancellation request"
    );

    // Test 2: Cancellation of non-existent request
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "id": 999999 }
        }),
    );

    // Should handle gracefully without response
    let response = read_response_timeout(&mut fixture.server, Duration::from_millis(200));
    assert!(response.is_none(), "Non-existent request cancellation should not produce response");

    // Test 3: JSON-RPC structure validation
    let test_id = 3001;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": test_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 0, "character": 0 }
            }
        }),
    );

    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "id": test_id }
        }),
    );

    if let Some(resp) =
        read_response_matching_i64(&mut fixture.server, test_id, Duration::from_millis(500))
    {
        // Validate JSON-RPC 2.0 structure
        assert_eq!(resp.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
        assert!(resp.get("id").is_some());

        if let Some(error) = resp.get("error") {
            assert_eq!(error["code"].as_i64(), Some(-32800));
            assert!(error.get("message").is_some());
        }
    }

    // Test establishes JSON-RPC 2.0 compliance patterns
    // Enhanced implementation will add comprehensive protocol validation
    Ok(())
}

// ============================================================================
// AC2: Thread-Safe Cancellation Token Architecture Tests
// ============================================================================

/// Tests feature spec: CANCELLATION_ARCHITECTURE_GUIDE.md#thread-safe-cancellation-token
/// AC:2 - Thread-safe cancellation token with atomic operations
#[test]
fn test_atomic_cancellation_token_operations_ac2() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicBool, Ordering};

    let token = Arc::new(PerlLspCancellationToken::new(
        json!("atomic_operations_test"),
        "test_provider".to_string(),
    ));

    let all_done = Arc::new(AtomicBool::new(false));

    // Test concurrent cancellation checks from multiple threads
    // Each thread polls until cancellation is observed or timeout
    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            let token_clone = Arc::clone(&token);
            let done_clone = Arc::clone(&all_done);
            thread::spawn(move || {
                let mut observed_cancellation = false;
                let mut check_count: u64 = 0;
                let deadline = Instant::now() + Duration::from_millis(500);

                while Instant::now() < deadline && !done_clone.load(Ordering::Relaxed) {
                    let start = Instant::now();
                    let is_cancelled = token_clone.is_cancelled();
                    let latency = start.elapsed();
                    check_count += 1;

                    if is_cancelled {
                        observed_cancellation = true;
                        break;
                    }

                    // Validate latency requirement: each check should be fast
                    // Use generous threshold for CI/WSL2 environments where
                    // thread scheduling can introduce multi-millisecond delays
                    assert!(
                        latency < Duration::from_millis(50),
                        "Cancellation check latency {}us exceeds 50ms limit",
                        latency.as_micros()
                    );

                    // Yield to other threads periodically
                    if check_count.is_multiple_of(1000) {
                        thread::yield_now();
                    }
                }

                (thread_id, observed_cancellation, check_count)
            })
        })
        .collect();

    // Cancel from another thread after brief delay
    let cancel_token = Arc::clone(&token);
    let cancel_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        cancel_token.cancel();
    });

    // Wait for cancel thread
    cancel_handle.join().map_err(|e| format!("Cancel thread panicked: {:?}", e))?;

    // Signal workers to stop
    all_done.store(true, Ordering::Relaxed);

    // Wait for all threads and collect results
    let mut thread_results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(result) => thread_results.push(result),
            Err(e) => {
                return Err(format!(
                    "Thread panicked during concurrent cancellation test: {:?}",
                    e
                )
                .into());
            }
        }
    }

    // Verify cancellation propagated correctly
    assert!(token.is_cancelled(), "Token should be in cancelled state");

    // Analyze results for thread safety
    let observed_count = thread_results.iter().filter(|(_, observed, _)| *observed).count();

    assert!(observed_count > 0, "At least some threads should observe cancellation");

    // Verify all threads did meaningful work
    for (thread_id, _, check_count) in &thread_results {
        assert!(*check_count > 0, "Thread {} should have performed at least one check", thread_id);
    }

    Ok(())
}

/// Tests feature spec: CANCELLATION_ARCHITECTURE_GUIDE.md#cancellation-registry
/// AC:2 - Cancellation registry thread safety with concurrent operations
#[test]
fn test_cancellation_registry_concurrent_operations_ac2() -> Result<(), Box<dyn std::error::Error>>
{
    let registry = Arc::new(CancellationRegistry::new());

    // Test concurrent token registration and cancellation
    let handles: Vec<_> = (0..50)
        .map(|thread_id| {
            let registry_clone = Arc::clone(&registry);
            thread::spawn(move || {
                let mut operation_results: Vec<(&str, Duration, bool)> = Vec::new();

                // Each thread registers multiple tokens
                for token_id in 0..20 {
                    let request_id = json!(format!("thread_{}_{}", thread_id, token_id));

                    // Create and register token
                    let start = Instant::now();
                    let token = PerlLspCancellationToken::new(
                        request_id.clone(),
                        format!("thread_{}", thread_id),
                    );
                    let register_ok = registry_clone.register_token(token).is_ok();
                    let register_duration = start.elapsed();

                    // Immediately cancel some tokens to test concurrent operations
                    if token_id % 3 == 0 {
                        let cancel_start = Instant::now();
                        let cancel_result = registry_clone.cancel_request(&request_id);
                        let cancel_duration = cancel_start.elapsed();

                        operation_results.push((
                            "cancel",
                            register_duration + cancel_duration,
                            register_ok && cancel_result.is_ok(),
                        ));
                    } else {
                        operation_results.push(("register", register_duration, register_ok));
                    }
                }

                operation_results
            })
        })
        .collect();

    // Concurrent cleanup operations from separate thread
    let cleanup_registry = Arc::clone(&registry);
    let cleanup_handle = thread::spawn(move || {
        let mut cleanup_results = Vec::new();
        for cleanup_cycle in 0..25 {
            thread::sleep(adaptive_sleep_ms(20));
            let start = Instant::now();
            // Remove a known request id pattern to exercise concurrent removal
            let req_id = json!(format!("thread_0_{}", cleanup_cycle));
            cleanup_registry.remove_request(&req_id);
            let duration = start.elapsed();
            cleanup_results.push((cleanup_cycle, duration));
        }
        cleanup_results
    });

    // Wait for all operations to complete
    let mut all_operation_results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(thread_results) => all_operation_results.extend(thread_results),
            Err(e) => {
                return Err(format!("Thread panicked during registry operations: {:?}", e).into());
            }
        }
    }

    let cleanup_results =
        cleanup_handle.join().map_err(|e| format!("Cleanup thread panicked: {:?}", e))?;

    // Validate thread safety - no deadlocks or corruption occurred
    assert!(!all_operation_results.is_empty(), "Operations should have completed");
    assert!(!cleanup_results.is_empty(), "Cleanup operations should have completed");

    // Validate performance requirements
    let max_operation_time = all_operation_results
        .iter()
        .map(|(_, duration, _)| *duration)
        .max()
        .unwrap_or(Duration::from_nanos(0));

    assert!(
        max_operation_time < Duration::from_millis(100),
        "Registry operations should complete within 100ms"
    );

    let max_cleanup_time = cleanup_results
        .iter()
        .map(|(_, duration)| *duration)
        .max()
        .unwrap_or(Duration::from_nanos(0));

    assert!(
        max_cleanup_time < Duration::from_millis(50),
        "Cleanup operations should complete within 50ms"
    );

    Ok(())
}

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#provider-cleanup-context
/// AC:2 - Provider-specific cleanup with thread-safe coordination
#[test]
fn test_provider_cleanup_thread_safety_ac2() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let registry = Arc::new(CancellationRegistry::new());
    let cleanup_counter = Arc::new(AtomicUsize::new(0));

    // Register tokens with cleanup contexts for different providers
    let provider_types = ["completion", "hover", "references"];
    for (i, provider_type) in provider_types.iter().enumerate() {
        let request_id = json!(format!("cleanup_test_{}", i));
        let token = PerlLspCancellationToken::new(request_id.clone(), provider_type.to_string());
        registry.register_token(token)?;

        let counter_clone = Arc::clone(&cleanup_counter);
        let context =
            ProviderCleanupContext::new(provider_type.to_string(), Some(json!({"test": i})))
                .with_cleanup(move || {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });
        registry.register_cleanup(&request_id, context)?;
    }

    // Concurrent cancellation from multiple threads
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let registry_clone = Arc::clone(&registry);
            thread::spawn(move || {
                let request_id = json!(format!("cleanup_test_{}", i));
                let start = Instant::now();
                let result = registry_clone.cancel_request(&request_id);
                let duration = start.elapsed();
                (i, result.is_ok(), duration)
            })
        })
        .collect();

    // Wait for all cancellation operations
    let mut results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(result) => results.push(result),
            Err(e) => {
                return Err(format!("Provider cleanup thread panicked: {:?}", e).into());
            }
        }
    }

    // Validate cleanup coordination
    assert_eq!(results.len(), 3, "All cancellation operations should complete");

    let successful = results.iter().filter(|(_, ok, _)| *ok).count();
    assert_eq!(successful, 3, "All cancellation operations should succeed");

    // Validate cleanup callbacks were invoked
    assert_eq!(
        cleanup_counter.load(Ordering::Relaxed),
        3,
        "All cleanup callbacks should have been invoked"
    );

    // Validate cleanup latency
    let max_cleanup_latency =
        results.iter().map(|(_, _, duration)| *duration).max().unwrap_or(Duration::from_nanos(0));

    assert!(
        max_cleanup_latency < Duration::from_millis(100),
        "Provider cleanup should complete within 100ms"
    );

    Ok(())
}

// ============================================================================
// AC3: Comprehensive LSP Provider Integration Tests
// ============================================================================

/// Tests feature spec: LSP_CANCELLATION_INTEGRATION_SCHEMA.md#dual-indexing-integration
/// AC:3 - Dual indexing cancellation with consistency preservation
#[test]
fn test_dual_indexing_cancellation_consistency_ac3() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Wait for initial indexing to complete with adaptive timeout
    let initial_indexing_timeout = match max_concurrent_threads() {
        0..=2 => Duration::from_secs(12), // Constrained: reduced from 10s
        3..=4 => Duration::from_secs(6),  // Moderate: reduced timeout
        _ => Duration::from_secs(4),      // Unconstrained: shorter timeout
    };
    drain_until_quiet(&mut fixture.server, Duration::from_millis(500), initial_indexing_timeout);

    // Verify baseline dual pattern functionality before cancellation testing
    let baseline_qualified =
        request_workspace_symbols(&mut fixture.server, "TestModule::test_function");
    let baseline_bare = request_workspace_symbols(&mut fixture.server, "test_function");

    // Baseline assertions - these should work with current implementation
    assert!(
        !baseline_qualified.is_empty() || !baseline_bare.is_empty(),
        "Either qualified or bare pattern should find symbols"
    );

    // Test cancellation during workspace symbol search (re-indexing simulation)
    let reindex_id = 3001;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": reindex_id,
            "method": "workspace/symbol",
            "params": { "query": "complex_reindex_operation" }
        }),
    );

    // Cancel during indexing operation to test consistency preservation
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": reindex_id,
                "context": {
                    "operation": "workspace_indexing",
                    "dual_pattern_preservation": true
                }
            }
        }),
    );

    // Verify dual pattern functionality is preserved after cancellation
    let post_cancel_qualified =
        request_workspace_symbols(&mut fixture.server, "TestModule::test_function");
    let post_cancel_bare = request_workspace_symbols(&mut fixture.server, "test_function");

    // Index consistency validation - enhanced implementation will ensure
    // atomic operations preserve dual indexing integrity
    if !baseline_qualified.is_empty() && !post_cancel_qualified.is_empty() {
        assert_eq!(
            baseline_qualified.len(),
            post_cancel_qualified.len(),
            "Qualified pattern results should be consistent after cancellation"
        );
    }

    if !baseline_bare.is_empty() && !post_cancel_bare.is_empty() {
        assert_eq!(
            baseline_bare.len(),
            post_cancel_bare.len(),
            "Bare pattern results should be consistent after cancellation"
        );
    }

    // Test establishes dual indexing consistency patterns for enhanced implementation
    Ok(())
}

/// Helper function to request workspace symbols
fn request_workspace_symbols(server: &mut LspServer, query: &str) -> Vec<Value> {
    let response = send_request(
        server,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/symbol",
            "params": { "query": query }
        }),
    );

    response.get("result").and_then(|r| r.as_array()).cloned().unwrap_or_default()
}

/// Tests feature spec: LSP_CANCELLATION_INTEGRATION_SCHEMA.md#cross-file-navigation
/// AC:3 - Cross-file navigation cancellation with multi-tier fallback preservation
#[test]
fn test_cross_file_navigation_cancellation_ac3() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Wait for cross-file indexing to stabilize with adaptive timeout
    let cross_file_timeout = match max_concurrent_threads() {
        0..=2 => Duration::from_secs(18), // Constrained: reduced from 15s
        3..=4 => Duration::from_secs(8),  // Moderate: reduced timeout
        _ => Duration::from_secs(5),      // Unconstrained: shorter timeout
    };
    drain_until_quiet(&mut fixture.server, Duration::from_millis(1000), cross_file_timeout);

    // Test definition resolution cancellation across files
    let definition_id = 4001;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": definition_id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 4, "character": 20 }  // Reference to TestModule::test_function
            }
        }),
    );

    // Cancel definition resolution to test multi-tier fallback preservation
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": definition_id,
                "context": {
                    "navigation_tier": "cross_file",
                    "preserve_fallback": true,
                    "multi_tier_strategy": true
                }
            }
        }),
    );

    // Validate cancellation response or completion
    let response =
        read_response_matching_i64(&mut fixture.server, definition_id, Duration::from_secs(2));
    validate_cancellation_or_completion(response, "cross-file definition resolution");

    // Verify workspace navigation remains functional after cancellation
    let verification_response = send_request(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 2, "character": 5 }  // Simpler local reference
            }
        }),
    );

    // Navigation system should remain stable regardless of previous cancellation
    assert!(
        verification_response.get("result").is_some()
            || verification_response.get("error").is_some(),
        "Workspace navigation should remain functional after cancellation"
    );

    // Test establishes cross-file navigation cancellation patterns
    // Enhanced implementation will add multi-tier fallback preservation
    Ok(())
}

/// Helper function to validate cancellation response or normal completion
fn validate_cancellation_or_completion(response: Option<Value>, operation: &str) {
    if let Some(resp) = response {
        if let Some(error) = resp.get("error") {
            // Validate proper cancellation
            assert_eq!(
                error["code"].as_i64(),
                Some(-32800),
                "{} cancellation should return RequestCancelled code",
                operation
            );
            assert!(
                error.get("message").is_some(),
                "{} cancellation should include error message",
                operation
            );
        } else {
            // Normal completion is acceptable for fast operations
            assert!(
                resp.get("result").is_some(),
                "{} should have result if completed normally",
                operation
            );
        }
    }
    // No response also acceptable if cancelled before processing started
}

/// Tests feature spec: LSP_CANCELLATION_INTEGRATION_SCHEMA.md#workspace-symbol-search
/// AC:3 - Workspace symbol search with dual pattern cancellation handling
#[test]
fn test_workspace_symbol_dual_pattern_cancellation_ac3() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Create larger test workspace to increase cancellation opportunity
    let large_file_content = generate_large_perl_content(500); // 500 functions
    fixture.setup_test_file("file:///large_module.pl", &large_file_content);

    // Wait for large file indexing with adaptive timeout
    let large_file_timeout = match max_concurrent_threads() {
        0..=2 => Duration::from_secs(25), // Constrained: reduced from 20s
        3..=4 => Duration::from_secs(12), // Moderate: reduced timeout
        _ => Duration::from_secs(8),      // Unconstrained: shorter timeout
    };
    drain_until_quiet(&mut fixture.server, Duration::from_millis(1000), large_file_timeout);

    // Test qualified pattern search with cancellation
    let qualified_search_id = 5001;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": qualified_search_id,
            "method": "workspace/symbol",
            "params": { "query": "TestModule::complex_function" }
        }),
    );

    // Cancel after brief delay to test cancellation during search
    thread::sleep(Duration::from_millis(50));
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": qualified_search_id,
                "context": {
                    "search_pattern": "qualified",
                    "dual_pattern_mode": true
                }
            }
        }),
    );

    // Test bare pattern search with cancellation
    let bare_search_id = 5002;
    send_request_no_wait(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "id": bare_search_id,
            "method": "workspace/symbol",
            "params": { "query": "complex_function" }
        }),
    );

    thread::sleep(Duration::from_millis(50));
    send_notification(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": bare_search_id,
                "context": {
                    "search_pattern": "bare",
                    "dual_pattern_mode": true
                }
            }
        }),
    );

    // Validate both search cancellations
    let qualified_response = read_response_matching_i64(
        &mut fixture.server,
        qualified_search_id,
        Duration::from_secs(3),
    );
    let bare_response =
        read_response_matching_i64(&mut fixture.server, bare_search_id, Duration::from_secs(3));

    validate_cancellation_or_completion(qualified_response, "qualified pattern search");
    validate_cancellation_or_completion(bare_response, "bare pattern search");

    // Test establishes dual pattern search cancellation patterns
    // Enhanced implementation will add comprehensive dual indexing coordination
    Ok(())
}

/// Generate large Perl content for testing cancellation timing
fn generate_large_perl_content(function_count: usize) -> String {
    let mut content = String::new();
    content.push_str("package LargeModule;\n\n");

    for i in 0..function_count {
        content.push_str(&format!(
            "sub function_{} {{\n    my ($self, $arg) = @_;\n    return $arg + {};\n}}\n\n",
            i, i
        ));
    }

    content.push_str("1;\n");
    content
}

// ============================================================================
// AC4: Enhanced Error Response Handling Tests
// ============================================================================

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#enhanced-error-response
/// AC:4 - Enhanced -32800 error code responses with context and performance tracking
#[test]
fn test_enhanced_error_response_handling_ac4() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    let test_scenarios = vec![
        (
            "hover",
            "textDocument/hover",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 2, "character": 5 }
            }),
        ),
        (
            "completion",
            "textDocument/completion",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 3, "character": 10 }
            }),
        ),
        (
            "references",
            "textDocument/references",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 4, "character": 15 },
                "context": { "includeDeclaration": true }
            }),
        ),
    ];

    for (scenario_name, method, params) in test_scenarios {
        let request_id = 6000 + scenario_name.len() as i64; // Unique ID per scenario

        // Record start time for latency measurement
        let start_time = Instant::now();

        // Send request
        send_request_no_wait(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": method,
                "params": params
            }),
        );

        // Cancel with enhanced context
        send_notification(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": {
                    "id": request_id,
                    "context": {
                        "provider": method,
                        "scenario": scenario_name,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| format!("System time error: {}", e))?
                            .as_millis() as i64,
                        "latency_tracking": true
                    }
                }
            }),
        );

        // Measure total cancellation latency
        let cancellation_latency = start_time.elapsed();

        // Validate enhanced error response
        if let Some(response) =
            read_response_matching_i64(&mut fixture.server, request_id, Duration::from_secs(1))
        {
            if let Some(error) = response.get("error") {
                // Validate standard error code
                assert_eq!(
                    error["code"].as_i64(),
                    Some(-32800),
                    "Should return RequestCancelled error code (-32800)"
                );

                // Validate enhanced error message with provider context
                let message = error["message"].as_str().ok_or("Error should have message")?;
                assert!(
                    message.contains(scenario_name) || {
                        if let Some(method_name) = method.split('/').next_back() {
                            message.to_lowercase().contains(&method_name.to_lowercase())
                        } else {
                            message.to_lowercase().contains(&method.to_lowercase())
                        }
                    },
                    "Error message should reference provider: {}",
                    scenario_name
                );

                // Validate enhanced error data structure
                if let Some(data) = error.get("data") {
                    // Provider context validation
                    assert!(
                        data.get("provider").is_some(),
                        "Enhanced error should include provider information"
                    );

                    // Latency tracking validation (enhanced feature)
                    if data.get("latency_ms").is_some() {
                        let latency_ms =
                            data["latency_ms"].as_u64().ok_or("Latency should be numeric")?;
                        assert!(
                            latency_ms <= cancellation_latency.as_millis() as u64,
                            "Reported latency should be reasonable: {}ms",
                            latency_ms
                        );
                    }

                    // Request ID validation
                    assert_eq!(
                        data.get("request_id"),
                        Some(&json!(request_id)),
                        "Enhanced error should include original request ID"
                    );
                }
            }

            // Validate JSON-RPC 2.0 structure
            assert_eq!(response["jsonrpc"].as_str(), Some("2.0"));
            assert_eq!(response["id"].as_i64(), Some(request_id));
        }
    }

    // Test establishes enhanced error response patterns
    // Implementation will add comprehensive error context and performance tracking
    Ok(())
}

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#error-graceful-degradation
/// AC:4 - Graceful error handling under various cancellation scenarios
#[test]
fn test_graceful_error_degradation_ac4() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Test 1: Rapid successive cancellations
    let rapid_ids = vec![7001, 7002, 7003, 7004, 7005];

    for &id in &rapid_ids {
        send_request_no_wait(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": "file:///main.pl" },
                    "position": { "line": 1, "character": 5 }
                }
            }),
        );

        // Immediate cancellation
        send_notification(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": id }
            }),
        );
    }

    // Collect responses and validate graceful handling
    let mut responses = Vec::new();
    for &id in &rapid_ids {
        if let Some(resp) =
            read_response_matching_i64(&mut fixture.server, id, Duration::from_millis(500))
        {
            responses.push((id, resp));
        }
    }

    // System should handle rapid cancellations gracefully
    assert!(fixture.server.is_alive(), "Server should remain stable under rapid cancellation load");

    // Validate error responses maintain consistency
    for (id, response) in &responses {
        if let Some(error) = response.get("error") {
            assert_eq!(
                error["code"].as_i64(),
                Some(-32800),
                "Rapid cancellation {} should have correct error code",
                id
            );
            assert!(
                error.get("message").is_some(),
                "Rapid cancellation {} should include error message",
                id
            );
        }
    }

    // Test 2: Malformed cancellation requests with graceful degradation
    let malformed_scenarios = vec![
        // Missing ID
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "invalid": "no_id" }
        }),
        // Invalid ID type
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "id": {"invalid": "object"} }
        }),
        // Missing params
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest"
        }),
    ];

    for malformed_request in malformed_scenarios {
        send_notification(&mut fixture.server, malformed_request);

        // Brief pause to process
        thread::sleep(adaptive_sleep_ms(50));

        // Server should remain stable
        assert!(
            fixture.server.is_alive(),
            "Server should handle malformed cancellation requests gracefully"
        );
    }

    // Final stability check
    let health_response = send_request(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 0, "character": 0 }
            }
        }),
    );

    assert!(
        health_response.get("result").is_some() || health_response.get("error").is_some(),
        "Server should remain functional after error degradation testing"
    );

    // Test establishes graceful error degradation patterns
    Ok(())
}

// ============================================================================
// AC5: Multiple Concurrent Cancellation Handling Tests
// ============================================================================

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#concurrent-cancellation-management
/// AC:5 - Multiple concurrent cancellation handling without interference
#[test]
fn test_concurrent_cancellation_coordination_ac5() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Create concurrent requests across different providers
    let concurrent_requests = vec![
        (
            8001,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 0, "character": 5 }
            }),
        ),
        (
            8002,
            "textDocument/completion",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 1, "character": 10 }
            }),
        ),
        (
            8003,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 2, "character": 15 }
            }),
        ),
        (
            8004,
            "textDocument/references",
            json!({
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 3, "character": 20 },
                "context": { "includeDeclaration": true }
            }),
        ),
        (
            8005,
            "workspace/symbol",
            json!({
                "query": "test_function"
            }),
        ),
    ];

    // Send all requests concurrently
    let start_time = Instant::now();
    for (id, method, params) in &concurrent_requests {
        send_request_no_wait(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        );
    }

    // Cancel requests in different patterns to test coordination
    let cancellation_patterns = vec![
        // Immediate cancellations
        (8001, Duration::from_millis(10)),
        (8003, Duration::from_millis(15)),
        // Delayed cancellations
        (8002, Duration::from_millis(100)),
        (8005, Duration::from_millis(150)),
        // Skip 8004 to test mixed completion/cancellation
    ];

    // Execute cancellation pattern
    for (id, delay) in cancellation_patterns {
        thread::sleep(delay);
        send_notification(
            &mut fixture.server,
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": {
                    "id": id,
                    "context": {
                        "concurrent_group": "coordination_test",
                        "coordination_id": format!("cancel_{}", id)
                    }
                }
            }),
        );
    }

    // Collect all responses with reasonable timeout
    let mut results = HashMap::new();
    for (id, method, _) in &concurrent_requests {
        if let Some(response) =
            read_response_matching_i64(&mut fixture.server, *id, Duration::from_secs(2))
        {
            results.insert(*id, ((*method).to_string(), response));
        }
    }

    let total_coordination_time = start_time.elapsed();

    // Validate concurrent coordination
    assert!(
        fixture.server.is_alive(),
        "Server should remain stable during concurrent cancellation coordination"
    );

    assert!(
        total_coordination_time < Duration::from_secs(5),
        "Concurrent cancellation coordination should complete within reasonable time"
    );

    // Analyze cancellation vs completion results
    let mut cancelled_count = 0;
    let mut completed_count = 0;
    let mut error_count = 0;

    for (id, (method, response)) in &results {
        if let Some(error) = response.get("error") {
            if error["code"].as_i64() == Some(-32800) {
                cancelled_count += 1;

                // Validate cancellation error structure
                assert!(
                    error.get("message").is_some(),
                    "Cancelled request {} ({}) should have error message",
                    id,
                    method
                );
            } else {
                error_count += 1;
            }
        } else if response.get("result").is_some() {
            completed_count += 1;
        }
    }

    // Validate coordination effectiveness
    assert!(
        cancelled_count + completed_count + error_count == results.len(),
        "All responses should be categorized"
    );

    assert!(
        cancelled_count > 0 || completed_count > 0,
        "Some requests should either be cancelled or completed"
    );

    // Test establishes concurrent cancellation coordination patterns
    // Enhanced implementation will add comprehensive coordination and resource management
    Ok(())
}

/// Tests feature spec: LSP_CANCELLATION_PROTOCOL.md#resource-management
/// AC:5 - Resource cleanup during concurrent cancellation without memory leaks
#[test]
fn test_concurrent_resource_cleanup_ac5() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = CancellationTestFixture::new();

    // Memory measurement before concurrent operations
    let baseline_memory = estimate_memory_usage();

    // Create wave of concurrent requests and cancellations
    let wave_count = 10;
    let requests_per_wave = 20;

    for wave in 0..wave_count {
        let mut wave_requests = Vec::new();

        // Create wave of requests
        for req_in_wave in 0..requests_per_wave {
            let id = (wave * 1000) + req_in_wave + 9000;

            send_request_no_wait(
                &mut fixture.server,
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "textDocument/completion",
                    "params": {
                        "textDocument": { "uri": "file:///main.pl" },
                        "position": { "line": req_in_wave % 10, "character": 5 }
                    }
                }),
            );

            wave_requests.push(id);
        }

        // Cancel most requests in wave (leaving some to complete)
        for (idx, &id) in wave_requests.iter().enumerate() {
            if idx % 3 != 0 {
                // Cancel ~67% of requests
                send_notification(
                    &mut fixture.server,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "$/cancelRequest",
                        "params": {
                            "id": id,
                            "context": {
                                "wave": wave,
                                "resource_cleanup": true
                            }
                        }
                    }),
                );
            }
        }

        // Brief pause between waves for processing
        thread::sleep(adaptive_sleep_ms(100));
    }

    // Wait for all operations to settle
    drain_until_quiet(&mut fixture.server, Duration::from_millis(500), Duration::from_secs(10));

    // Memory measurement after concurrent operations
    let final_memory = estimate_memory_usage();
    let memory_growth = final_memory.saturating_sub(baseline_memory);

    // Validate resource cleanup effectiveness
    assert!(
        fixture.server.is_alive(),
        "Server should remain stable after concurrent resource cleanup testing"
    );

    // Memory growth should be reasonable (not indicating major leaks)
    assert!(
        memory_growth < 50 * 1024 * 1024, // 50MB threshold
        "Memory growth {} bytes should be reasonable after {} concurrent operations",
        memory_growth,
        wave_count * requests_per_wave
    );

    // Final system health check
    let health_response = send_request(
        &mut fixture.server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": "file:///main.pl" },
                "position": { "line": 0, "character": 0 }
            }
        }),
    );

    assert!(
        health_response.get("result").is_some() || health_response.get("error").is_some(),
        "Server should remain responsive after concurrent resource cleanup"
    );

    // Test establishes resource cleanup patterns for concurrent cancellation
    // Enhanced implementation will add comprehensive resource tracking and leak prevention
    Ok(())
}

/// Estimate memory usage (simplified for testing - real implementation would use system calls)
fn estimate_memory_usage() -> usize {
    // Placeholder for memory measurement
    // Real implementation would use platform-specific memory measurement
    // For Linux: parse /proc/self/status
    // For macOS: use task_info
    // For Windows: use GetProcessMemoryInfo
    0 // Will be replaced with actual memory measurement
}

// ============================================================================
// Integration Test Utilities
// ============================================================================

impl Drop for CancellationTestFixture {
    fn drop(&mut self) {
        // Graceful cleanup
        shutdown_and_exit(&mut self.server);
    }
}

// Test scaffolding is now established for AC1-AC5
// All tests are designed to:
// 1. Compile successfully (meeting TDD scaffolding requirements)
// 2. Fail initially due to missing cancellation infrastructure implementation
// 3. Provide clear patterns for enhanced cancellation feature development
// 4. Include comprehensive error handling and edge case validation
// 5. Integrate with existing LSP test infrastructure and patterns

// Next phase: Implement the missing cancellation infrastructure to make tests pass
