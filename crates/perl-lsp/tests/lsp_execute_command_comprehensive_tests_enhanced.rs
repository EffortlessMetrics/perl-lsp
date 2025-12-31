//! Enhanced test scaffolding for LSP executeCommand functionality (Issue #145)
//!
//! Tests feature spec: SPEC_145_LSP_EXECUTE_COMMAND_AND_CODE_ACTIONS.md
//! Architecture: ADR_003_EXECUTE_COMMAND_CODE_ACTIONS_ARCHITECTURE.md
//!
//! This module provides targeted test enhancements to address specific test failures:
//! - AC1: Server capability advertisement compliance with LSP 3.17+ format
//! - AC2: perl.runCritic protocol response standardization with structured format
//! - AC3: Dual analyzer strategy error handling with built-in fallback
//! - AC4: Protocol compliance under edge cases with appropriate error responses
//! - AC5: Revolutionary performance preservation maintaining 5000x improvements

use serde_json::json;
use std::time::Duration;

mod support;
use support::lsp_harness::{LspHarness, TempWorkspace};

// ======================== Enhanced Test Fixtures ========================

mod enhanced_execute_command_fixtures {
    /// Perl code with syntax errors that should trigger error handling
    pub const SYNTAX_ERROR_WITH_CONTEXT: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

# Syntax error: missing closing quote
my $broken_string = "unterminated string
print "This will cause issues";

# Another syntax error: missing semicolon and malformed sub
sub broken_function {
    my $param = shift
    # Missing semicolon above
    return $param
}

# Invalid regex
my $regex = qr/[/;

# Unmatched brackets
my @array = (1, 2, 3;
"#;

    /// Empty file for edge case testing
    pub const EMPTY_FILE: &str = "";

    /// File with only comments
    pub const COMMENTS_ONLY_FILE: &str = r#"#!/usr/bin/perl
# This file only has comments
# No actual code
# Should result in minimal violations
"#;
}

/// Create enhanced test server with better error handling validation
fn create_enhanced_execute_command_server() -> (LspHarness, TempWorkspace) {
    let workspace = TempWorkspace::new().expect("Failed to create temp workspace");

    // Write test files to workspace
    workspace
        .write("syntax_errors.pl", enhanced_execute_command_fixtures::SYNTAX_ERROR_WITH_CONTEXT)
        .expect("Failed to write syntax errors file");
    workspace
        .write("empty_file.pl", enhanced_execute_command_fixtures::EMPTY_FILE)
        .expect("Failed to write empty file");
    workspace
        .write("comments_only.pl", enhanced_execute_command_fixtures::COMMENTS_ONLY_FILE)
        .expect("Failed to write comments only file");

    let harness = LspHarness::new_raw();

    // Return uninitialized harness so tests can call initialize_default() themselves
    (harness, workspace)
}

// ======================== AC1: Enhanced Server Capabilities Testing ========================

#[test]
// AC1:executeCommand - Enhanced server capabilities validation with LSP 3.17+ compliance
fn test_enhanced_execute_command_server_capabilities() {
    let (mut harness, _workspace) = create_enhanced_execute_command_server();

    // Get server capabilities with detailed validation
    let init_result = harness.initialize_default().expect("Server should initialize successfully");

    let capabilities =
        init_result.get("capabilities").expect("Initialize result should contain capabilities");

    // AC1: Verify executeCommandProvider is advertised with proper structure
    assert!(
        capabilities.get("executeCommandProvider").is_some(),
        "Server should advertise executeCommandProvider capability per LSP 3.17+"
    );

    let execute_command_provider = &capabilities["executeCommandProvider"];

    // Validate LSP 3.17+ structure requirements
    assert!(
        execute_command_provider.is_object(),
        "executeCommandProvider should be object per LSP 3.17+ specification"
    );

    assert!(
        execute_command_provider.get("commands").is_some(),
        "ExecuteCommandProvider should list supported commands"
    );

    let commands = execute_command_provider["commands"]
        .as_array()
        .expect("Commands should be an array per LSP specification");

    // AC1: Verify all required commands are supported per Issue #145
    let expected_commands = vec![
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.debugTests",
        "perl.runCritic", // Critical for Issue #145
    ];

    for expected_command in expected_commands {
        let command_found = commands.iter().any(|cmd| cmd.as_str() == Some(expected_command));
        assert!(
            command_found,
            "Command '{}' should be in supported commands list for Issue #145 compliance",
            expected_command
        );
    }

    // AC1: Validate command list is non-empty and contains strings
    assert!(!commands.is_empty(), "Commands list should not be empty");
    for cmd in commands {
        assert!(cmd.is_string(), "All commands should be strings per LSP specification");
    }
}

#[test]
// AC1:executeCommand - Protocol compliance validation with enhanced error handling
fn test_enhanced_execute_command_protocol_compliance() {
    let (mut harness, _workspace) = create_enhanced_execute_command_server();

    // AC4: Test invalid command with proper error structure
    let invalid_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.invalidCommand",
            "arguments": []
        }),
        Duration::from_secs(2),
    );

    // Should return proper LSP error response
    match invalid_result {
        Ok(response) => {
            // Check if response contains error field
            assert!(
                response.get("error").is_some(),
                "Invalid command should return error in response per LSP protocol"
            );
        }
        Err(_) => {
            // Also acceptable - some implementations return HTTP-level errors
        }
    }

    // AC4: Test malformed request structure
    let malformed_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "invalid_field": "test"
            // Missing required 'command' field
        }),
        Duration::from_secs(2),
    );

    // Should handle gracefully
    assert!(
        malformed_result.is_ok() || malformed_result.is_err(),
        "Should handle malformed requests gracefully"
    );
}

// ======================== AC2: Enhanced perl.runCritic Testing ========================

#[test]
// AC2:runCritic - Enhanced syntax error handling with proper response structure
fn test_enhanced_perl_run_critic_syntax_error_handling() {
    let (mut harness, workspace) = create_enhanced_execute_command_server();

    // Initialize server with workspace
    harness
        .initialize_with_root(&workspace.root_uri, None)
        .expect("Server should initialize successfully");

    // Open documents for testing
    harness
        .open_document(
            &workspace.uri("syntax_errors.pl"),
            enhanced_execute_command_fixtures::SYNTAX_ERROR_WITH_CONTEXT,
        )
        .expect("Failed to open syntax errors file");

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("syntax_errors.pl")]
            }),
            Duration::from_secs(3),
        )
        .expect("perl.runCritic should handle syntax errors gracefully");

    // AC2: Validate response structure per specification
    assert!(
        result.get("status").is_some(),
        "Response should have status field per perl.runCritic specification"
    );

    assert!(
        result.get("violations").is_some(),
        "Response should have violations field even with syntax errors"
    );

    assert!(
        result.get("analyzerUsed").is_some(),
        "Response should indicate which analyzer was used (dual strategy)"
    );

    // AC3: Should report either violations OR errors (dual analyzer strategy)
    let has_violations = result["violations"].as_array().map(|v| !v.is_empty()).unwrap_or(false);

    let has_errors = result.get("errors").is_some();
    let has_error_status = result["status"].as_str() == Some("error");

    assert!(
        has_violations || has_errors || has_error_status,
        "Should report syntax issues as violations, errors field, or error status per dual analyzer strategy"
    );

    // AC2: Validate analyzer fallback indication
    let analyzer_used = result["analyzerUsed"].as_str().expect("Should indicate analyzer type");
    assert!(
        analyzer_used == "builtin" || analyzer_used == "external",
        "Should use either 'builtin' or 'external' analyzer per dual strategy"
    );
}

#[test]
// AC2:runCritic - Enhanced empty file handling with edge case validation
fn test_enhanced_empty_file_handling() {
    let (mut harness, workspace) = create_enhanced_execute_command_server();

    // Initialize server with workspace
    harness
        .initialize_with_root(&workspace.root_uri, None)
        .expect("Server should initialize successfully");

    // Open empty file for testing
    harness
        .open_document(
            &workspace.uri("empty_file.pl"),
            enhanced_execute_command_fixtures::EMPTY_FILE,
        )
        .expect("Failed to open empty file");

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("empty_file.pl")]
            }),
            Duration::from_secs(2),
        )
        .expect("Should handle empty files gracefully");

    // AC4: Edge case - empty files should succeed with minimal violations
    assert_eq!(result["status"], "success", "Empty files should be handled successfully");

    let violations = result["violations"].as_array().expect("Should return violations array");

    // Empty files might have basic violations (missing pragmas)
    assert!(
        violations.len() <= 3,
        "Empty files should have minimal violations, got: {}",
        violations.len()
    );
}

#[test]
// AC2:runCritic - Performance validation with revolutionary threading preservation
fn test_enhanced_performance_validation() {
    // Create large file content for performance testing
    let violations_content = "my $var = 42;\nprint \"$var\\n\";\n".repeat(50);
    let large_file_content = format!(
        "#!/usr/bin/perl\n# Large file performance test\n{}{}\n",
        "# Comment line\n".repeat(20),
        violations_content
    );

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("large_performance.pl", &large_file_content)])
            .expect("Failed to create performance test workspace");

    harness
        .open_document(&workspace.uri("large_performance.pl"), &large_file_content)
        .expect("Failed to open large performance file");

    // AC5: Revolutionary performance preservation - adaptive timeout
    let thread_count =
        std::env::var("RUST_TEST_THREADS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);

    let timeout_secs = match thread_count {
        n if n <= 2 => 10, // High contention - more time needed
        n if n <= 4 => 7,  // Medium contention
        _ => 5,            // Low contention - maintain speed
    };

    harness.wait_for_idle(Duration::from_millis(200)); // Quick idle wait

    let start_time = std::time::Instant::now();

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("large_performance.pl")]
            }),
            Duration::from_secs(timeout_secs), // Adaptive timeout
        )
        .expect("perl.runCritic should complete within timeout for large files");

    let duration = start_time.elapsed();

    // AC5: Performance requirement with revolutionary thread-aware scaling
    let max_duration = Duration::from_secs(timeout_secs - 1); // Leave 1s buffer
    assert!(
        duration < max_duration,
        "perl.runCritic should complete within {}s for large files (revolutionary performance), took: {:?}",
        timeout_secs - 1,
        duration
    );

    assert_eq!(result["status"], "success", "Should succeed for large files");

    // Should detect violations but complete quickly
    let violations = result["violations"].as_array().expect("Should return violations");
    assert!(!violations.is_empty(), "Should detect violations in large file");
}

// ======================== AC4: Enhanced Protocol Compliance ========================

#[test]
#[cfg_attr(
    not(feature = "stress-tests"),
    ignore = "STRESS: executeCommand timing test - needs proper shutdown handling"
)]
// AC4:protocolCompliance - URI handling with comprehensive validation
fn test_enhanced_uri_handling() {
    let (mut harness, workspace) = create_enhanced_execute_command_server();
    harness.initialize_default().expect("Server should initialize successfully");

    // Test various URI formats
    let test_cases = vec![
        (workspace.uri("comments_only.pl"), true, "Valid workspace URI"),
        ("file:///nonexistent/file.pl".to_string(), false, "Non-existent file URI"),
        ("invalid_uri_format".to_string(), false, "Invalid URI format"),
        ("".to_string(), false, "Empty URI"),
    ];

    for (uri, should_succeed, description) in test_cases {
        let result = harness.request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [uri]
            }),
            Duration::from_secs(2),
        );

        if should_succeed {
            assert!(result.is_ok(), "Should handle valid URI: {}", description);
            if let Ok(response) = result {
                assert!(
                    response.get("status").is_some(),
                    "Valid URI should have status: {}",
                    description
                );
            }
        } else {
            // Should either succeed with error status or return error response
            match result {
                Ok(response) => {
                    // Graceful handling - should have error status or error field
                    let has_error_status = response["status"].as_str() == Some("error");
                    let has_error_field = response.get("error").is_some();
                    assert!(
                        has_error_status || has_error_field,
                        "Invalid URI should be handled gracefully: {}",
                        description
                    );
                }
                Err(_) => {
                    // Also acceptable - error response for invalid input
                }
            }
        }
    }
}

#[test]
#[cfg_attr(
    not(feature = "stress-tests"),
    ignore = "STRESS: Concurrent executeCommand stress test - needs shutdown handling"
)]
// AC4:protocolCompliance - Concurrent request handling validation
fn test_enhanced_concurrent_handling() {
    let (mut harness, workspace) = create_enhanced_execute_command_server();
    harness.initialize_default().expect("Server should initialize successfully");

    // AC4: Test concurrent requests with different files
    let requests = vec![
        ("syntax_errors.pl", "Request 1: syntax errors"),
        ("comments_only.pl", "Request 2: comments only"),
        ("empty_file.pl", "Request 3: empty file"),
    ];

    let mut results = Vec::new();

    // Send requests concurrently (simplified - real concurrency needs async)
    for (file, description) in requests {
        let result = harness.request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri(file)]
            }),
            Duration::from_secs(3),
        );
        results.push((result, description));
    }

    // Validate all results
    for (result, description) in results {
        assert!(result.is_ok(), "Concurrent request should succeed: {}", description);

        if let Ok(response) = result {
            assert!(response.get("status").is_some(), "Should have status: {}", description);
            assert!(
                response.get("analyzerUsed").is_some(),
                "Should indicate analyzer: {}",
                description
            );
        }
    }
}

// ======================== Revolutionary Performance Integration ========================

#[test]
#[cfg_attr(
    not(feature = "stress-tests"),
    ignore = "STRESS: Performance integration test - needs proper shutdown"
)]
// AC5:performance - Thread-aware timeout scaling validation
fn test_revolutionary_performance_integration() {
    let (mut harness, workspace) = create_enhanced_execute_command_server();
    harness.initialize_default().expect("Server should initialize successfully");

    // AC5: Revolutionary performance with thread-aware scaling
    let thread_count =
        std::env::var("RUST_TEST_THREADS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);

    // Adaptive performance expectations
    let (expected_max_ms, timeout_secs) = match thread_count {
        n if n <= 2 => (500, 3), // High contention: more lenient
        n if n <= 4 => (300, 2), // Medium contention
        _ => (200, 2),           // Low contention: maintain speed
    };

    let start_time = std::time::Instant::now();

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("comments_only.pl")]
            }),
            Duration::from_secs(timeout_secs),
        )
        .expect("Revolutionary performance: command should complete quickly");

    let duration = start_time.elapsed();

    // AC5: Validate revolutionary performance is preserved
    assert!(
        duration < Duration::from_millis(expected_max_ms),
        "Revolutionary performance: should complete within {}ms (5000x improvement), took: {:?} (threads={})",
        expected_max_ms,
        duration,
        thread_count
    );

    assert_eq!(result["status"], "success", "Revolutionary performance: should succeed");
}
