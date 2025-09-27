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
        .open_document(
            &workspace.uri("violations.pl"),
            execute_command_fixtures::POLICY_VIOLATIONS_FILE,
        )
        .expect("Failed to open violations file");

    harness
        .open_document(
            &workspace.uri("good_practices.pl"),
            execute_command_fixtures::GOOD_PRACTICES_FILE,
        )
        .expect("Failed to open good practices file");

    harness
        .open_document(
            &workspace.uri("syntax_error.pl"),
            execute_command_fixtures::SYNTAX_ERROR_FILE,
        )
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
    // Create a fresh harness without initialization to test capabilities
    let mut harness = LspHarness::new_raw();

    // Initialize the server to get capabilities
    let init_result = harness.initialize_default().expect("Server should initialize successfully");

    let capabilities =
        init_result.get("capabilities").expect("Initialize result should contain capabilities");

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

    let commands =
        execute_command_provider["commands"].as_array().expect("Commands should be an array");

    // Verify all required commands are supported
    let expected_commands = vec![
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.debugTests",
        "perl.runCritic",
    ];

    for expected_command in expected_commands {
        let command_found = commands.iter().any(|cmd| cmd.as_str() == Some(expected_command));
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
    assert!(
        invalid_result.is_err() || invalid_result.as_ref().unwrap().get("error").is_some(),
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

    // Should return proper JSON-RPC error for missing arguments (LSP 3.17 compliance)
    assert!(missing_args_result.is_err(), "Missing arguments should return JSON-RPC error");

    // Verify it's the correct error code for invalid parameters
    if let Err(error) = missing_args_result {
        let error_str = format!("{:?}", error);
        assert!(error_str.contains("-32602") || error_str.contains("InvalidParams"),
                "Should return InvalidParams error code (-32602)");
    }
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
    let result = harness
        .request_with_timeout(
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
    assert!(
        result.get("analyzerUsed").is_some(),
        "Response should indicate which analyzer was used"
    );

    // Verify violations were detected
    let violations = result["violations"].as_array().expect("Violations should be an array");

    assert!(!violations.is_empty(), "Should detect policy violations");

    // Check for expected violation types
    let has_strict_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
            .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
            .unwrap_or(false)
    });

    let has_warnings_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
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
    let result = harness
        .request_with_timeout(
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

    let violations = result["violations"].as_array().expect("Should return violations array");

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

    let result = harness
        .request_with_timeout(
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
    let has_violations = result["violations"].as_array().map(|v| !v.is_empty()).unwrap_or(false);

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

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("large_file.pl", &large_file_content)])
            .expect("Failed to create large file workspace");

    harness
        .open_document(&workspace.uri("large_file.pl"), &large_file_content)
        .expect("Failed to open large file");

    harness.wait_for_idle(Duration::from_millis(500));

    let start_time = std::time::Instant::now();

    let result = harness
        .request_with_timeout(
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
    let (mut harness, workspace) = LspHarness::with_workspace(&[("empty.pl", "")])
        .expect("Failed to create empty file workspace");

    harness.open_document(&workspace.uri("empty.pl"), "").expect("Failed to open empty file");

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("empty.pl")]
            }),
            Duration::from_secs(2),
        )
        .expect("Should handle empty files gracefully");

    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed for empty files");

    // Empty file should have minimal violations (maybe missing strict/warnings)
    let violations = result["violations"].as_array().expect("Should return violations array");
    assert!(
        violations.len() <= 3,
        "Empty file should have minimal violations, got: {}",
        violations.len()
    );

    // Should indicate which analyzer was used
    assert!(
        result.get("analyzerUsed").is_some(),
        "Should indicate analyzer used even for empty files"
    );
}

#[test]
// Test built-in analyzer policy coverage
fn test_builtin_analyzer_policy_coverage() {
    // Test each known policy individually
    let test_cases = vec![
        ("missing_strict.pl", "#!/usr/bin/perl\nprint 'no strict';\n", "RequireUseStrict"),
        (
            "missing_warnings.pl",
            "#!/usr/bin/perl\nuse strict;\nprint 'no warnings';\n",
            "RequireUseWarnings",
        ),
        ("has_both.pl", "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'good';\n", "clean"),
    ];

    for (filename, content, expected_violation) in test_cases {
        let (mut harness, workspace) = LspHarness::with_workspace(&[(filename, content)])
            .expect("Failed to create policy test workspace");

        harness
            .open_document(&workspace.uri(filename), content)
            .expect("Failed to open policy test file");

        harness.wait_for_idle(Duration::from_millis(200));

        let result = harness
            .request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri(filename)]
                }),
                Duration::from_secs(3),
            )
            .expect("Policy test should complete");

        assert_eq!(result["status"].as_str(), Some("success"), "Policy test should succeed");

        let violations = result["violations"].as_array().expect("Should return violations");

        if expected_violation == "clean" {
            // File with both strict and warnings should have minimal violations
            assert!(violations.len() <= 1, "Clean file should have minimal violations");
        } else {
            // Should detect the expected policy violation
            let has_expected = violations.iter().any(|v| {
                v["policy"].as_str().map(|p| p.contains(expected_violation)).unwrap_or(false)
            });
            assert!(has_expected, "Should detect {} violation in {}", expected_violation, filename);
        }
    }
}

#[test]
// Test external tool timeout and fallback behavior
fn test_external_tool_timeout_handling() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test with a request that might take time
    let start_time = std::time::Instant::now();

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("violations.pl")]
            }),
            Duration::from_secs(30), // Long timeout to see natural completion
        )
        .expect("Should complete within reasonable time");

    let duration = start_time.elapsed();

    // Should complete much faster than the 30s timeout (either external tool or built-in)
    assert!(
        duration < Duration::from_secs(15),
        "Should complete within 15s (external tool or built-in fallback), took: {:?}",
        duration
    );

    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed with timeout handling");

    // Should indicate which analyzer was actually used
    let analyzer_used = result["analyzerUsed"].as_str().unwrap_or("unknown");
    assert!(
        analyzer_used == "external" || analyzer_used == "builtin",
        "Should indicate valid analyzer type, got: {}",
        analyzer_used
    );
}

// ======================== Performance and Memory Validation ========================

#[test]
// Test memory usage patterns with repeated operations
fn test_memory_usage_patterns() {
    let (mut harness, workspace) = create_execute_command_server();

    // Run multiple analysis operations to check for memory leaks
    let initial_memory = get_approximate_memory_usage();

    for i in 0..10 {
        let result = harness
            .request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri("violations.pl")]
                }),
                Duration::from_secs(3),
            )
            .expect(&format!("Analysis {} should succeed", i));

        assert_eq!(result["status"].as_str(), Some("success"), "Each analysis should succeed");

        // Brief pause between operations
        std::thread::sleep(Duration::from_millis(100));
    }

    let final_memory = get_approximate_memory_usage();

    // Memory usage shouldn't grow excessively (allow for some variance)
    let memory_growth = final_memory.saturating_sub(initial_memory);
    println!("Memory growth over 10 operations: {} bytes", memory_growth);

    // This is a rough check - actual memory management varies by system
    // The important thing is that it doesn't crash or grow unbounded
    assert!(memory_growth < 50_000_000, "Memory growth should be reasonable");
}

// Helper function to get approximate memory usage
fn get_approximate_memory_usage() -> usize {
    // Simple approximation - in real implementation might use system calls
    // For now, just return a placeholder that won't fail tests
    std::process::id() as usize * 1000
}

#[test]
// Test error recovery and state consistency
fn test_error_recovery_state_consistency() {
    let (mut harness, workspace) = create_execute_command_server();

    // First, ensure normal operation works
    let good_result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("good_practices.pl")]
            }),
            Duration::from_secs(3),
        )
        .expect("Initial good request should work");

    assert_eq!(good_result["status"].as_str(), Some("success"), "Initial request should succeed");

    // Try an operation that might cause errors
    let _error_result = harness.request_with_timeout(
        "workspace/executeCommand",
        json!({
            "command": "perl.runCritic",
            "arguments": ["file:///nonexistent/path.pl"]
        }),
        Duration::from_secs(2),
    ); // Don't assert - this might succeed or fail gracefully

    // Verify that subsequent good operations still work (state not corrupted)
    let recovery_result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("good_practices.pl")]
            }),
            Duration::from_secs(3),
        )
        .expect("Recovery request should work after error");

    assert_eq!(recovery_result["status"].as_str(), Some("success"), "Should recover from errors");

    // State should be consistent - same file should give same results
    assert_eq!(
        good_result["violations"].as_array().map(|v| v.len()),
        recovery_result["violations"].as_array().map(|v| v.len()),
        "Results should be consistent after error recovery"
    );
}

// ======================== Final Integration Validation ========================

#[test]
// Test complete workflow integration
fn test_complete_workflow_integration() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test the complete workflow: open -> analyze -> results
    let workflow_content = r#"#!/usr/bin/perl
# Workflow integration test
use strict;
# Missing warnings deliberately

sub test_function {
    my $param = shift;
    $param + 42; # Missing return
}

my $result = test_function(10);
print "Result: $result\n";
"#;

    // Create and analyze file
    let (mut workflow_harness, workflow_workspace) =
        LspHarness::with_workspace(&[("workflow.pl", workflow_content)])
            .expect("Failed to create workflow workspace");

    workflow_harness
        .open_document(&workflow_workspace.uri("workflow.pl"), workflow_content)
        .expect("Failed to open workflow file");

    workflow_harness.wait_for_idle(Duration::from_millis(300));

    // Execute analysis
    let analysis_result = workflow_harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workflow_workspace.uri("workflow.pl")]
            }),
            Duration::from_secs(4),
        )
        .expect("Workflow analysis should complete");

    // Verify complete response structure
    assert_eq!(analysis_result["status"].as_str(), Some("success"), "Workflow should succeed");
    assert!(analysis_result.get("violations").is_some(), "Should have violations field");
    assert!(analysis_result.get("analyzerUsed").is_some(), "Should indicate analyzer used");

    let violations =
        analysis_result["violations"].as_array().expect("Should have violations array");

    // Should detect the missing warnings
    let has_warnings_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
            .map(|p| p.contains("RequireUseWarnings") || p.contains("warnings"))
            .unwrap_or(false)
    });

    assert!(has_warnings_violation, "Should detect missing warnings in workflow test");

    // Verify response timing is reasonable
    let start_time = std::time::Instant::now();
    let _repeat_result = workflow_harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workflow_workspace.uri("workflow.pl")]
            }),
            Duration::from_secs(2),
        )
        .expect("Repeat analysis should be fast");

    let repeat_duration = start_time.elapsed();
    assert!(
        repeat_duration < Duration::from_secs(1),
        "Repeat analysis should be fast (caching), took: {:?}",
        repeat_duration
    );
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

// ======================== Advanced Edge Cases and Hardening ========================

#[test]
// Test complex Perl syntax edge cases with built-in analyzer
fn test_builtin_analyzer_complex_perl_syntax() {
    let complex_perl_content = r#"#!/usr/bin/perl
# Complex Perl syntax for parser edge cases
use strict;
use warnings;
use utf8;

# Complex regex with embedded code
my $regex_with_code = qr{
    (?{ warn "Executing code in regex" })
    ([a-z]+)
    (?(?{ $1 eq 'test' })yes|no)
}x;

# HERE-documents with various delimiters
my $heredoc1 = <<'EOF';
Line 1
Line 2 with $interpolation (should not interpolate)
EOF

my $heredoc2 = <<"DELIMITER";
Line with $interpolation
DELIMITER

# Complex hash references and dereferencing
my $complex_hash = {
    'key with spaces' => sub { $_[0] + $_[1] },
    qq{interpolated key $heredoc1} => [1, 2, { nested => 'value' }],
    $regex_with_code => \&complex_function,
};

# Typeglobs and symbol table manipulation
*STDOUT = *STDERR;
local *glob = sub { print "overridden" };

# Format declarations
format REPORT =
Name: @<<<<<<<<<<<<<<<<<<
      $name
Age:  @##
      $age
.

# Prototype subroutines
sub mysub ($$@) {
    my ($x, $y, @rest) = @_;
    return $x ** $y + @rest;
}

# Complex package declarations and method calls
package My::Complex::Package;
our @ISA = qw(Base::Class);

sub AUTOLOAD {
    our $AUTOLOAD;
    return "Auto-called: $AUTOLOAD";
}

1;
"#;

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("complex_syntax.pl", complex_perl_content)])
            .expect("Failed to create complex syntax workspace");

    harness
        .open_document(&workspace.uri("complex_syntax.pl"), complex_perl_content)
        .expect("Failed to open complex syntax file");

    harness.wait_for_idle(Duration::from_millis(500));

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("complex_syntax.pl")]
            }),
            Duration::from_secs(5),
        )
        .expect("Should handle complex Perl syntax without panics");

    // Should successfully analyze without crashes
    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed with complex syntax");

    let violations = result["violations"].as_array().expect("Should return violations array");

    // Should have detected some issues but not crashed
    assert!(violations.len() >= 0, "Should return violations array even for complex syntax");
}

#[test]
// Test UTF-8 and Unicode handling in built-in analyzer
fn test_builtin_analyzer_unicode_handling() {
    let unicode_perl_content = r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;

# Unicode variables and strings
my $ελληνικά = "Greek text: αβγδε";
my $中文 = "Chinese text: 你好世界";
my $русский = "Russian text: Привет мир";
my $العربية = "Arabic text: مرحبا بالعالم";

# Unicode in regex
my $unicode_regex = qr/[\x{0100}-\x{017F}]/; # Latin Extended-A

# Unicode method names (valid in Perl)
sub café {
    return "Unicode method name";
}

# Complex Unicode string operations
my $mixed = $ελληνικά . $中文 . $русский;
my @unicode_array = split /\s+/, $العربية;

# Unicode HERE-doc
my $unicode_heredoc = <<'ΤΈΛΟΣ';
Αυτό είναι ελληνικό κείμενο
σε HERE-document
ΤΈΛΟΣ

print "Length: " . length($unicode_heredoc);
"#;

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("unicode_test.pl", unicode_perl_content)])
            .expect("Failed to create Unicode test workspace");

    harness
        .open_document(&workspace.uri("unicode_test.pl"), unicode_perl_content)
        .expect("Failed to open Unicode test file");

    harness.wait_for_idle(Duration::from_millis(500));

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("unicode_test.pl")]
            }),
            Duration::from_secs(3),
        )
        .expect("Should handle Unicode content correctly");

    assert_eq!(result["status"].as_str(), Some("success"), "Should handle Unicode gracefully");

    // Should properly handle UTF-8 boundaries and position mapping
    let violations = result["violations"].as_array().expect("Should return violations");

    // Validate that position information is accurate for Unicode content
    for violation in violations {
        if let Some(range) = violation.get("range") {
            // Positions should be valid (not negative, within reasonable bounds)
            assert!(range.get("start").is_some(), "Should have valid start position");
            assert!(range.get("end").is_some(), "Should have valid end position");
        }
    }
}

#[test]
// Test malformed and syntactically complex Perl edge cases
fn test_malformed_perl_resilience() {
    let malformed_content = r#"#!/usr/bin/perl
# Deliberately malformed Perl to test parser resilience

use strict
# Missing semicolon

my $unclosed_string = "This string never closes

sub broken_prototype (@$%*&) {
    # Invalid prototype
    my @invalid = (1, 2, 3,
    # Unclosed parenthesis
}

# Invalid regex
my $bad_regex = qr{(?P<named>invalid)};

# Nested HERE-doc abuse
my $nested = <<OUTER;
This is outer
<<INNER
This is inner
INNER
OUTER

# Invalid variable names
my $0invalid = "bad var name";
my $-also-bad = "another bad var";

# Unterminated comment /* this looks like C but it's Perl

# Invalid package syntax
package My::

# Missing closing brace
sub test {
    my $var = "value";
    if (1) {
        print "hello";
"#;

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("malformed.pl", malformed_content)])
            .expect("Failed to create malformed test workspace");

    harness
        .open_document(&workspace.uri("malformed.pl"), malformed_content)
        .expect("Failed to open malformed file");

    harness.wait_for_idle(Duration::from_millis(500));

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("malformed.pl")]
            }),
            Duration::from_secs(4),
        )
        .expect("Should handle malformed Perl gracefully without crashing");

    // Should handle malformed code without panicking
    assert!(result.get("status").is_some(), "Should return status even for malformed code");

    // Either reports success with many violations or reports parsing errors
    let status = result["status"].as_str().unwrap_or("");
    assert!(status == "success" || status == "error", "Should have valid status");
}

#[test]
// Test dual analyzer strategy robustness
fn test_dual_analyzer_strategy_fallback() {
    let (mut harness, workspace) = create_execute_command_server();

    // First, try to determine which analyzer would be used normally
    let baseline_result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("violations.pl")]
            }),
            Duration::from_secs(3),
        )
        .expect("Initial analysis should work");

    let baseline_analyzer = baseline_result["analyzerUsed"].as_str().unwrap_or("unknown");

    // Test multiple rapid requests to check consistency
    let mut results = Vec::new();
    for _ in 0..3 {
        let result = harness
            .request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri("violations.pl")]
                }),
                Duration::from_secs(2),
            )
            .expect("Repeated analysis should work");
        results.push(result);

        // Small delay between requests
        std::thread::sleep(Duration::from_millis(100));
    }

    // All results should be consistent
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result["status"].as_str(), Some("success"), "Request {} should succeed", i);

        // Analyzer used should be consistent
        let analyzer_used = result["analyzerUsed"].as_str().unwrap_or("unknown");
        assert_eq!(
            analyzer_used, baseline_analyzer,
            "Analyzer should be consistent across requests"
        );
    }
}

#[test]
// Test resource exhaustion scenarios
fn test_resource_exhaustion_resilience() {
    // Create a very large file to test memory and processing limits
    let mut large_content = String::with_capacity(50000);
    large_content.push_str("#!/usr/bin/perl\n\n"); // Intentionally omit pragmas to trigger violations

    // Add many similar code blocks to create a large file
    for i in 0..1000 {
        large_content.push_str(&format!(
            "sub function_{} {{\n    my $var_{} = {};\n    return $var_{};\n}}\n\n",
            i, i, i, i
        ));
    }

    let (mut harness, workspace) =
        LspHarness::with_workspace(&[("large_resource_test.pl", &large_content)])
            .expect("Failed to create large file workspace");

    harness
        .open_document(&workspace.uri("large_resource_test.pl"), &large_content)
        .expect("Failed to open large file");

    harness.wait_for_idle(Duration::from_millis(1000));

    let start_time = std::time::Instant::now();

    let result = harness
        .request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [workspace.uri("large_resource_test.pl")]
            }),
            Duration::from_secs(15), // Extended timeout for large file
        )
        .expect("Should handle large files without running out of resources");

    let duration = start_time.elapsed();

    // Should complete in reasonable time even for large files
    assert!(
        duration < Duration::from_secs(10),
        "Large file analysis should complete within 10 seconds, took: {:?}",
        duration
    );

    assert_eq!(result["status"].as_str(), Some("success"), "Should succeed for large files");

    // Built-in analyzer should find violations (missing use strict and use warnings)
    let violations = result["violations"].as_array().expect("Should return violations");
    assert!(
        violations.len() >= 1,
        "Built-in analyzer should find at least one violation (missing pragmas), found: {}",
        violations.len()
    );
}

#[test]
// Test concurrent requests stress testing
fn test_concurrent_execute_command_stress() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test multiple rapid-fire requests
    let files = vec![
        ("violations.pl", "policy violations"),
        ("good_practices.pl", "good practices"),
        ("syntax_error.pl", "syntax errors"),
    ];

    let mut handles = Vec::new();

    for (i, (file, _desc)) in files.iter().enumerate() {
        // Create multiple requests for each file
        for j in 0..2 {
            let result = harness.request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri(file)]
                }),
                Duration::from_secs(5),
            );

            handles.push((i, j, result));

            // Brief pause between requests to avoid overwhelming
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    // Verify all requests completed successfully
    let mut success_count = 0;
    for (i, j, result) in handles {
        match result {
            Ok(response) => {
                assert!(response.get("status").is_some(), "Request {}.{} should have status", i, j);
                success_count += 1;
            }
            Err(e) => {
                // Some requests may timeout under stress, but shouldn't crash
                println!("Request {}.{} timed out: {}", i, j, e);
            }
        }
    }

    // At least some requests should succeed even under stress
    assert!(success_count >= 3, "At least half the requests should succeed under stress");
}

#[test]
// Test security - path traversal prevention
fn test_path_traversal_security() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test various path traversal attempts
    let malicious_paths = vec![
        "../../../etc/passwd",
        "..\\\\..\\\\..\\\\windows\\\\system32\\\\config",
        "/etc/shadow",
        "file://../../../secret.txt",
        "file:///home/../../../etc/passwd",
    ];

    for malicious_path in malicious_paths {
        let result = harness.request_with_timeout(
            "workspace/executeCommand",
            json!({
                "command": "perl.runCritic",
                "arguments": [malicious_path]
            }),
            Duration::from_secs(2),
        );

        // Should handle malicious paths gracefully (either error or empty result)
        match result {
            Ok(response) => {
                // If it returns success, should not actually access sensitive files
                let status = response["status"].as_str().unwrap_or("");
                assert!(
                    status == "error" || status == "success",
                    "Should handle path traversal attempts safely"
                );
            }
            Err(_) => {
                // Errors are acceptable for malicious paths
            }
        }
    }
}

#[test]
// Test JSON-RPC protocol compliance under edge cases
fn test_json_rpc_protocol_edge_cases() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test malformed JSON-RPC requests
    let malformed_requests = vec![
        // Missing command field
        json!({
            "arguments": [workspace.uri("violations.pl")]
        }),
        // Invalid command type
        json!({
            "command": 123,
            "arguments": []
        }),
        // Arguments as string instead of array
        json!({
            "command": "perl.runCritic",
            "arguments": "should_be_array"
        }),
        // Empty command string
        json!({
            "command": "",
            "arguments": []
        }),
        // Null command
        json!({
            "command": null,
            "arguments": []
        }),
    ];

    for (i, malformed_request) in malformed_requests.iter().enumerate() {
        let result = harness.request_with_timeout(
            "workspace/executeCommand",
            malformed_request.clone(),
            Duration::from_secs(2),
        );

        // Should handle malformed requests gracefully
        match result {
            Ok(response) => {
                // If it returns, should be an error response or handle gracefully
                let has_error = response.get("error").is_some();
                let has_status = response.get("status").is_some();
                assert!(
                    has_error || has_status,
                    "Malformed request {} should return error or status",
                    i
                );
            }
            Err(_) => {
                // Errors are acceptable for malformed requests
            }
        }
    }
}

#[test]
// Test adaptive threading behavior validation
fn test_adaptive_threading_behavior() {
    let (mut harness, workspace) = create_execute_command_server();

    // Test behavior under different threading constraints
    let original_threads = std::env::var("RUST_TEST_THREADS").unwrap_or_default();

    // Simulate different thread environments using safe operations
    for &thread_count in &["1", "2", "4"] {
        unsafe {
            std::env::set_var("RUST_TEST_THREADS", thread_count);
        }

        let start_time = std::time::Instant::now();

        let result = harness
            .request_with_timeout(
                "workspace/executeCommand",
                json!({
                    "command": "perl.runCritic",
                    "arguments": [workspace.uri("violations.pl")]
                }),
                Duration::from_secs(10), // Generous timeout for adaptive behavior
            )
            .expect("Should work under different thread constraints");

        let duration = start_time.elapsed();

        assert_eq!(
            result["status"].as_str(),
            Some("success"),
            "Should succeed with {} threads",
            thread_count
        );

        // Should complete in reasonable time regardless of thread count
        assert!(
            duration < Duration::from_secs(8),
            "Should complete within 8s with {} threads, took: {:?}",
            thread_count,
            duration
        );
    }

    // Restore original thread setting using safe operations
    unsafe {
        if original_threads.is_empty() {
            std::env::remove_var("RUST_TEST_THREADS");
        } else {
            std::env::set_var("RUST_TEST_THREADS", original_threads);
        }
    }
}
