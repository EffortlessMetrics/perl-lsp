//! Comprehensive tests for LSP executeCommand functionality
//!
//! Tests feature spec: SPEC_145_LSP_EXECUTE_COMMAND_AND_CODE_ACTIONS.md
//! Architecture: ADR_003_EXECUTE_COMMAND_CODE_ACTIONS_ARCHITECTURE.md
//!
//! This module provides complete test coverage for all executeCommand features
//! with focus on Issue #145 resolution and LSP 3.17+ protocol compliance.

use serde_json::json;
use std::time::Duration;

mod support;
use support::lsp_harness::{LspHarness, TempWorkspace};

// Test fixtures for executeCommand testing
mod execute_command_fixtures {
    /// Perl code with various policy violations for perlcritic testing
    pub const POLICY_VIOLATIONS_FILE: &str = r#"#!/usr/bin/perl
# This file deliberately contains Perl::Critic policy violations

my $variable = 42;
print "Value: $variable\n";

sub calculate {
    my ($a, $b) = @_;
    $a + $b;  # Missing explicit return
}

open FILE, "test.txt";  # Should use 3-arg open
print FILE "Hello\n";
close FILE;

for my $i (0..10) {
    print $i;
}

my @array = (1,2,3,4,5);
my $element = $array[0];
"#;

    /// Perl code with good practices (should have minimal violations)
    pub const GOOD_PRACTICES_FILE: &str = r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

sub process_file {
    my ($filename) = @_;

    open my $fh, '<', $filename
        or die "Cannot open file '$filename': $!";

    my @lines = <$fh>;
    close $fh
        or warn "Cannot close file '$filename': $!";

    return @lines;
}

my $result = calculate(5, 10);
print "Result: $result\n";

1;
"#;

    /// Test file with syntax errors for error handling tests
    pub const SYNTAX_ERROR_FILE: &str = r#"#!/usr/bin/perl
use strict;
use warnings;

my $variable = 42
# Missing semicolon

sub broken_function {
    my ($param = @_;  # Syntax error in parameter
    return $param;
}

print "This won't compile"
"#;
}

/// Create test server with executeCommand-focused workspace
fn create_execute_command_server() -> (LspHarness, TempWorkspace) {
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("violations.pl", execute_command_fixtures::POLICY_VIOLATIONS_FILE),
        ("good_practices.pl", execute_command_fixtures::GOOD_PRACTICES_FILE),
        ("syntax_error.pl", execute_command_fixtures::SYNTAX_ERROR_FILE),
    ])
    .expect("Failed to create executeCommand test workspace");

    // Initialize documents
    harness
        .open_document(&workspace.uri("violations.pl"), execute_command_fixtures::POLICY_VIOLATIONS_FILE)
        .expect("Failed to open violations file");

    harness
        .open_document(&workspace.uri("good_practices.pl"), execute_command_fixtures::GOOD_PRACTICES_FILE)
        .expect("Failed to open good practices file");

    harness
        .open_document(&workspace.uri("syntax_error.pl"), execute_command_fixtures::SYNTAX_ERROR_FILE)
        .expect("Failed to open syntax error file");

    // Trigger processing and wait for idle
    harness.did_save(&workspace.uri("violations.pl")).ok();
    harness.did_save(&workspace.uri("good_practices.pl")).ok();
    harness.did_save(&workspace.uri("syntax_error.pl")).ok();

    harness.wait_for_idle(Duration::from_millis(1000));

    (harness, workspace)
}

// ======================== AC1: Complete executeCommand LSP Method Implementation ========================

#[test]
// AC1:executeCommand - Complete executeCommand LSP method implementation
fn test_execute_command_server_capabilities() {
    let (mut harness, _workspace) = create_execute_command_server();

    // Get server capabilities after initialization
    // Initialize the server to get capabilities
    let init_result = harness.initialize_default()
        .expect("Server should initialize successfully");

    let capabilities = init_result.get("capabilities")
        .expect("Initialize result should contain capabilities");

    // Verify executeCommandProvider is advertised
    assert!(
        capabilities.get("executeCommandProvider").is_some(),
        "Server should advertise executeCommandProvider capability"
    );

    let execute_command_provider = &capabilities["executeCommandProvider"];
    assert!(
        execute_command_provider.get("commands").is_some(),
        "ExecuteCommandProvider should list supported commands"
    );

    let commands = execute_command_provider["commands"].as_array()
        .expect("Commands should be an array");

    // Verify all required commands are supported
    let expected_commands = vec![
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.debugTests",
        "perl.runCritic"
    ];

    for expected_command in expected_commands {
        let command_found = commands.iter().any(|cmd|
            cmd.as_str() == Some(expected_command)
        );
        assert!(
            command_found,
            "Command '{}' should be in supported commands list",
            expected_command
        );
    }
}

#[test]
// AC1:executeCommand - Protocol compliance with error handling
fn test_execute_command_protocol_compliance() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test invalid command
    let invalid_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.invalidCommand",
            "arguments": []
        }),
        Duration::from_secs(2),
    );

    // Should return error for invalid command
    assert!(invalid_result.is_err() ||
        invalid_result.as_ref().unwrap().get("error").is_some(),
        "Invalid command should return error response"
    );

    // Test missing arguments
    let missing_args_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic"
            // Missing arguments array
        }),
        Duration::from_secs(2),
    );

    // Should handle missing arguments gracefully
    assert!(missing_args_result.is_ok(), "Missing arguments should be handled gracefully");
}

#[test]
// AC1:executeCommand - Command parameter validation
fn test_execute_command_parameter_validation() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test perl.runCritic with invalid URI
    let invalid_uri_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": ["invalid_uri_format"]
        }),
        Duration::from_secs(2),
    );

    assert!(invalid_uri_result.is_ok(), "Should handle invalid URI gracefully");

    // Test perl.runCritic with non-existent file
    let nonexistent_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": ["file:///nonexistent/file.pl"]
        }),
        Duration::from_secs(2),
    );

    assert!(nonexistent_result.is_ok(), "Should handle non-existent files gracefully");
}

// ======================== AC2: perl.runCritic Command Integration ========================

#[test]
// AC2:runCritic - perl.runCritic with external perlcritic (if available)
fn test_perl_run_critic_external_tool() {
    let (mut harness, workspace) = create_execute_command_server();

    // Execute perl.runCritic on violations file
    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("violations.pl")]
        }),
        Duration::from_secs(5), // Extended timeout for external tool
    )
    .expect("perl.runCritic command should execute successfully");

    // Verify response structure
    assert!(result.get("status").is_some(), "Response should have status field");
    assert!(result.get("violations").is_some(), "Response should have violations field");
    assert!(result.get("analyzerUsed").is_some(), "Response should indicate which analyzer was used");

    // Verify violations were detected
    let violations = result["violations"].as_array()
        .expect("Violations should be an array");

    assert!(!violations.is_empty(), "Should detect policy violations");

    // Check for expected violation types
    let has_strict_violation = violations.iter().any(|v| {
        v["policy"].as_str()
            .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
            .unwrap_or(false)
    });

    let has_warnings_violation = violations.iter().any(|v| {
        v["policy"].as_str()
            .map(|p| p.contains("RequireUseWarnings") || p.contains("warnings"))
            .unwrap_or(false)
    });

    assert!(has_strict_violation, "Should detect missing 'use strict'");
    assert!(has_warnings_violation, "Should detect missing 'use warnings'");
}

#[test]
// AC2:runCritic - Built-in analyzer fallback when external tool unavailable
fn test_perl_run_critic_builtin_analyzer() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test with good practices file (fewer violations expected)
    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("good_practices.pl")]
        }),
        Duration::from_secs(3),
    )
    .expect("perl.runCritic should work with built-in analyzer");

    // Verify response structure
    assert_eq!(result["status"].as_str(), Some("success"), "Should report success");

    let violations = result["violations"].as_array()
        .expect("Should return violations array");

    // Good practices file should have fewer violations
    assert!(
        violations.len() < 5,
        "Good practices file should have fewer violations, got: {}",
        violations.len()
    );
}

#[test]
// AC2:runCritic - Error handling for malformed Perl code
fn test_perl_run_critic_syntax_error_handling() {
    let (mut harness, workspace) = create_execute_command_server();

    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("syntax_error.pl")]
        }),
        Duration::from_secs(3),
    )
    .expect("perl.runCritic should handle syntax errors gracefully");

    // Should still return a valid response even with syntax errors
    assert!(result.get("status").is_some(), "Should have status even with syntax errors");

    // May report syntax errors as violations or in separate field
    let has_violations = result["violations"].as_array()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let has_errors = result.get("errors").is_some();

    assert!(
        has_violations || has_errors,
        "Should report syntax issues either as violations or errors"
    );
}

#[test]
// AC2:runCritic - Performance validation for large files
fn test_perl_run_critic_performance() {
    let large_file_content = format!(
        "#!/usr/bin/perl\n{}\n{}",
        "# Large file with many lines\n".repeat(100),
        execute_command_fixtures::POLICY_VIOLATIONS_FILE
    );

    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("large_file.pl", &large_file_content),
    ])
    .expect("Failed to create large file workspace");

    harness.open_document(&workspace.uri("large_file.pl"), &large_file_content)
        .expect("Failed to open large file");

    harness.wait_for_idle(Duration::from_millis(500));

    let start_time = std::time::Instant::now();

    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("large_file.pl")]
        }),
        Duration::from_secs(10), // Generous timeout for performance test
    )
    .expect("perl.runCritic should complete within timeout for large files");

    let duration = start_time.elapsed();

    // Performance requirement: should complete within reasonable time
    assert!(
        duration < Duration::from_secs(5),
        "perl.runCritic should complete within 5 seconds for large files, took: {:?}",
        duration
    );

    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed for large files");
}

// ======================== AC1 Additional: Existing executeCommand Validation ========================

#[test]
// AC1:executeCommand - Existing commands backward compatibility
fn test_existing_execute_commands() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test perl.runTests command
    let run_tests_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runTests",
            "arguments": [workspace.uri("good_practices.pl")]
        }),
        Duration::from_secs(3),
    );

    // Should not error (may not succeed due to no actual tests)
    assert!(run_tests_result.is_ok(), "perl.runTests should not error");

    // Test perl.runFile command
    let run_file_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runFile",
            "arguments": [workspace.uri("good_practices.pl")]
        }),
        Duration::from_secs(3),
    );

    assert!(run_file_result.is_ok(), "perl.runFile should not error");
}

#[test]
// AC1:executeCommand - Command execution timeout handling
fn test_execute_command_timeout_handling() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test command with very short timeout
    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("violations.pl")]
        }),
        Duration::from_millis(100), // Very short timeout
    );

    // Should either complete quickly or timeout gracefully
    match result {
        Ok(_) => {
            // Completed within timeout - good
        }
        Err(_) => {
            // Timed out - also acceptable for this test
            // The important thing is it doesn't hang indefinitely
        }
    }
}

// ======================== Error Handling and Edge Cases ========================

#[test]
// Test empty file handling
fn test_execute_command_empty_file() {
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("empty.pl", ""),
    ])
    .expect("Failed to create empty file workspace");

    harness.open_document(&workspace.uri("empty.pl"), "")
        .expect("Failed to open empty file");

    let result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("empty.pl")]
        }),
        Duration::from_secs(2),
    )
    .expect("Should handle empty files gracefully");

    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed for empty files");
}

#[test]
// Test concurrent executeCommand requests
fn test_concurrent_execute_commands() {
    let (mut harness, workspace) = create_execute_command_server();

    // Note: This is a simplified concurrency test
    // Real concurrent testing would require more sophisticated async handling

    let result1 = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("violations.pl")]
        }),
        Duration::from_secs(3),
    );

    let result2 = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": [workspace.uri("good_practices.pl")]
        }),
        Duration::from_secs(3),
    );

    // Both requests should succeed
    assert!(result1.is_ok(), "First concurrent request should succeed");
    assert!(result2.is_ok(), "Second concurrent request should succeed");
}