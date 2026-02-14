//! Public API mutation hardening tests for executeCommand implementation (PR #170)
//!
//! This test suite targets the 29 surviving mutants from mutation testing through the public API
//! to achieve ≥80% mutation score. Tests focus on observable behavior differences when mutations
//! are applied, ensuring mutants are killed through proper validation.
//!
//! **Mutation Categories Targeted:**
//! - Return value bypasses: Functions returning `Ok(Default::default())`
//! - Command routing failures: Command dispatch logic bypasses
//! - Parameter validation gaps: Input sanitization bypasses
//! - Protocol response mutations: LSP response format corruptions
//! - Position/arithmetic mutations: Line/column calculation corruptions
//!
//! **Strategy:**
//! - Use only public ExecuteCommandProvider API
//! - Deep value validation beyond boolean success checks
//! - Comprehensive edge case testing
//! - Specific response structure validation
//! - Cross-command behavior verification

use perl_lsp::execute_command::{ExecuteCommandProvider, get_supported_commands};
use serde_json::Value;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ============= RETURN VALUE BYPASS MUTATION KILLERS =============
// Target: Functions returning Ok(Default::default()) instead of proper results

#[test]
fn test_execute_command_not_default_comprehensive() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Create test files for comprehensive testing
    let test_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'test execution';\n";
    let temp_file = "/tmp/test_execute_not_default.pl";
    fs::write(temp_file, test_content)?;

    let sub_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nsub test_function { print 'executed'; return 42; }\n";
    let sub_file = "/tmp/test_sub_not_default.pl";
    fs::write(sub_file, sub_content)?;

    // Test each command to ensure no Ok(Default::default()) returns
    let test_cases = vec![
        ("perl.runTests", vec![Value::String(temp_file.to_string())], "runTests"),
        ("perl.runFile", vec![Value::String(temp_file.to_string())], "runFile"),
        (
            "perl.runTestSub",
            vec![Value::String(sub_file.to_string()), Value::String("test_function".to_string())],
            "runTestSub",
        ),
        ("perl.debugTests", vec![Value::String(temp_file.to_string())], "debugTests"),
        ("perl.runCritic", vec![Value::String(temp_file.to_string())], "runCritic"),
    ];

    for (command, args, description) in test_cases {
        let result = provider.execute_command(command, args);
        assert!(result.is_ok(), "Command {} should succeed", description);

        let result_value = result.map_err(|e| format!("Command should return Ok: {}", e))?;

        // CRITICAL MUTATION KILLER: Verify not Ok(Default::default())
        assert_ne!(
            result_value,
            Value::Object(serde_json::Map::new()),
            "Command {} must not return empty default object - targets return value bypass mutations",
            description
        );

        // Verify meaningful structure
        assert!(result_value.is_object(), "Command {} should return object", description);
        let obj = result_value.as_object().ok_or("Result should be an object")?;
        assert!(!obj.is_empty(), "Command {} result should not be empty", description);

        // Each command should have command-specific structure
        match command {
            "perl.runTests" | "perl.runFile" => {
                assert!(obj.contains_key("success"), "{} should have success field", description);
                assert!(obj.contains_key("output"), "{} should have output field", description);
                assert!(
                    result_value["success"].is_boolean(),
                    "{} success should be boolean",
                    description
                );
                assert!(
                    result_value["output"].is_string(),
                    "{} output should be string",
                    description
                );
            }
            "perl.runTestSub" => {
                assert!(obj.contains_key("success"), "{} should have success field", description);
                assert!(
                    obj.contains_key("subroutine"),
                    "{} should have subroutine field",
                    description
                );
                assert!(
                    result_value["subroutine"].is_string(),
                    "{} subroutine should be string",
                    description
                );
                assert_eq!(
                    result_value["subroutine"], "test_function",
                    "{} should have correct subroutine name",
                    description
                );
            }
            "perl.debugTests" => {
                assert!(obj.contains_key("success"), "{} should have success field", description);
                assert!(obj.contains_key("output"), "{} should have output field", description);
                assert_eq!(
                    result_value["success"], false,
                    "{} should indicate not implemented",
                    description
                );
                let output = result_value["output"].as_str().ok_or("output should be a string")?;
                assert!(
                    output.contains("not yet implemented"),
                    "{} should have not implemented message",
                    description
                );
            }
            "perl.runCritic" => {
                assert!(obj.contains_key("status"), "{} should have status field", description);
                assert!(
                    obj.contains_key("violations"),
                    "{} should have violations field",
                    description
                );
                assert!(
                    obj.contains_key("analyzerUsed"),
                    "{} should have analyzerUsed field",
                    description
                );
                assert!(
                    result_value["violations"].is_array(),
                    "{} violations should be array",
                    description
                );
                assert!(
                    result_value["analyzerUsed"].is_string(),
                    "{} analyzerUsed should be string",
                    description
                );
            }
            _ => must(Err::<(), _>(format!("Unexpected command: {}", command))),
        }
    }

    // Clean up
    fs::remove_file(temp_file).ok();
    fs::remove_file(sub_file).ok();
    Ok(())
}

// ============= COMMAND ROUTING MUTATION KILLERS =============
// Target: Command dispatch logic that can be completely bypassed

#[test]
fn test_command_routing_specificity_comprehensive() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Create test file
    let test_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'routing test';\n";
    let temp_file = "/tmp/test_routing_comprehensive.pl";
    fs::write(temp_file, test_content)?;

    // Execute all commands and verify they produce DIFFERENT results
    let run_tests_result = provider
        .execute_command("perl.runTests", vec![Value::String(temp_file.to_string())])
        .map_err(|e| format!("runTests should succeed: {}", e))?;

    let run_file_result = provider
        .execute_command("perl.runFile", vec![Value::String(temp_file.to_string())])
        .map_err(|e| format!("runFile should succeed: {}", e))?;

    let debug_tests_result = provider
        .execute_command("perl.debugTests", vec![Value::String(temp_file.to_string())])
        .map_err(|e| format!("debugTests should succeed: {}", e))?;

    let run_critic_result = provider
        .execute_command("perl.runCritic", vec![Value::String(temp_file.to_string())])
        .map_err(|e| format!("runCritic should succeed: {}", e))?;

    // MUTATION KILLER: Verify command-specific behaviors (proves routing works)

    // debugTests has unique "not implemented" behavior
    assert_eq!(debug_tests_result["success"], false, "debugTests should be false");
    assert!(
        debug_tests_result["output"]
            .as_str()
            .ok_or("output should be a string")?
            .contains("not yet implemented"),
        "debugTests should have specific not implemented message"
    );

    // runCritic has unique structure with status/violations/analyzerUsed
    assert!(run_critic_result.get("status").is_some(), "runCritic should have status field");
    assert!(
        run_critic_result.get("violations").is_some(),
        "runCritic should have violations field"
    );
    assert!(
        run_critic_result.get("analyzerUsed").is_some(),
        "runCritic should have analyzerUsed field"
    );
    assert_eq!(run_critic_result["status"], "success", "runCritic should have success status");

    // runTests and runFile have command execution structure but different behavior
    assert!(run_tests_result["success"].is_boolean(), "runTests should have boolean success");
    assert!(run_file_result["success"].is_boolean(), "runFile should have boolean success");

    // CRITICAL: All results should be structurally different (proving no routing bypass)
    assert_ne!(
        run_tests_result, run_critic_result,
        "runTests and runCritic should produce different results - proves routing works"
    );
    assert_ne!(
        run_file_result, debug_tests_result,
        "runFile and debugTests should produce different results - proves routing works"
    );
    assert_ne!(
        debug_tests_result, run_critic_result,
        "debugTests and runCritic should produce different results - proves routing works"
    );

    // Clean up
    fs::remove_file(temp_file).ok();
    Ok(())
}

#[test]
fn test_unknown_command_handling() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Test unknown command handling (targets command routing mutations)
    let result = provider.execute_command("perl.nonExistentCommand", vec![]);

    // MUTATION KILLER: Should fail with specific error (not bypass to success)
    assert!(result.is_err(), "Unknown command should return error");
    let error_msg = result.err().ok_or("Expected error")?;
    assert!(error_msg.contains("Unknown command"), "Should indicate unknown command");
    assert!(
        error_msg.contains("perl.nonExistentCommand"),
        "Should include the actual command name"
    );

    // Test multiple unknown commands to ensure consistent behavior
    let unknown_commands = vec![
        "perl.invalidCommand",
        "unknown.command",
        "perl.fakeCommand",
        "",
        "not.a.perl.command",
    ];

    for unknown_cmd in unknown_commands {
        let result = provider.execute_command(unknown_cmd, vec![]);
        assert!(result.is_err(), "Unknown command '{}' should return error", unknown_cmd);
        let error_msg = result.err().ok_or(format!("Expected error for '{}'", unknown_cmd))?;
        assert!(
            error_msg.contains("Unknown command"),
            "Should indicate unknown command for '{}'",
            unknown_cmd
        );
    }
    Ok(())
}

// ============= PARAMETER VALIDATION MUTATION KILLERS =============
// Target: Input sanitization bypasses and argument extraction mutations

#[test]
fn test_parameter_validation_comprehensive() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Test missing file path arguments for all commands that require them
    let commands_requiring_file_path =
        vec!["perl.runTests", "perl.runFile", "perl.debugTests", "perl.runCritic"];

    for command in commands_requiring_file_path {
        // Test with no arguments
        let result = provider.execute_command(command, vec![]);
        assert!(result.is_err(), "Command {} should fail with no arguments", command);
        let error_msg = result.err().ok_or(format!("Expected error for command {}", command))?;
        assert!(
            error_msg.contains("Missing file path argument"),
            "Command {} should have missing file path error",
            command
        );

        // Test with invalid argument types
        let invalid_args = [
            vec![Value::Null],
            vec![Value::Bool(true)],
            vec![Value::Number(serde_json::Number::from(123))],
            vec![Value::Array(vec![])],
            vec![Value::Object(serde_json::Map::new())],
        ];

        for (i, args) in invalid_args.iter().enumerate() {
            let result = provider.execute_command(command, args.clone());
            assert!(
                result.is_err(),
                "Command {} should fail with invalid args case {}",
                command,
                i
            );
            let error_msg =
                result.err().ok_or(format!("Expected error for command {} case {}", command, i))?;
            assert!(
                error_msg.contains("Missing file path argument"),
                "Command {} should have missing file path error for case {}",
                command,
                i
            );
        }
    }

    // Test runTestSub specific validation (requires 2 arguments)
    // Create a dummy file because path validation runs before argument validation
    // Use NamedTempFile for automatic cleanup (RAII pattern)
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(b"")?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let result =
        provider.execute_command("perl.runTestSub", vec![Value::String(temp_path.clone())]);
    assert!(result.is_err(), "runTestSub should fail with missing subroutine name");
    let error_msg =
        result.err().ok_or("Expected error for runTestSub with missing subroutine name")?;
    assert!(
        error_msg.contains("Missing subroutine name argument"),
        "Should have missing subroutine name error, got: {}",
        error_msg
    );

    // Test with invalid subroutine name type
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(temp_path.clone()), Value::Number(serde_json::Number::from(456))],
    );
    assert!(result.is_err(), "runTestSub should fail with invalid subroutine name type");
    let error_msg =
        result.err().ok_or("Expected error for runTestSub with invalid subroutine name type")?;
    assert!(
        error_msg.contains("Missing subroutine name argument"),
        "Should have missing subroutine name error for invalid type, got: {}",
        error_msg
    );

    // temp_file is automatically cleaned up when it goes out of scope (RAII)
    Ok(())
}

#[test]
fn test_file_path_extraction_validation() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Test that extract_file_path_argument returns actual values, not hardcoded ones
    // We do this indirectly by testing runCritic with different file paths

    let test_paths = vec![
        "/tmp/path1.pl",
        "/tmp/different_path.pl",
        "/home/user/test.pl",
        "file:///tmp/uri_test.pl",
    ];

    for path in test_paths {
        let result =
            provider.execute_command("perl.runCritic", vec![Value::String(path.to_string())]);

        // Should succeed or fail gracefully, but if it succeeds, it should process the actual path
        match result {
            Ok(response) => {
                // If it succeeds, verify it processed the path (either found file or reported not found)
                if response["status"] == "error" {
                    let error_msg = response["error"].as_str().ok_or("error should be a string")?;
                    if error_msg.contains("File not found") {
                        // Good - it actually checked the specific path
                        assert!(
                            error_msg.contains(path.strip_prefix("file://").unwrap_or(path)),
                            "Error should mention the actual path: {}",
                            path
                        );
                    }
                }
            }
            Err(error) => {
                // Should have meaningful error related to the path
                assert!(!error.is_empty(), "Error should not be empty for path: {}", path);
            }
        }
    }
    Ok(())
}

// ============= PROTOCOL RESPONSE MUTATION KILLERS =============
// Target: LSP response format corruptions and structure mutations

#[test]
fn test_response_structure_validation() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Create test file with known content
    let test_content =
        "#!/usr/bin/perl\n# Missing pragmas for violations\nmy $var = 42;\nprint $var;\n";
    let temp_file = "/tmp/test_response_structure.pl";
    fs::write(temp_file, test_content)?;

    // Test runCritic response structure in detail
    let result =
        provider.execute_command("perl.runCritic", vec![Value::String(temp_file.to_string())]);
    assert!(result.is_ok(), "runCritic should succeed");
    let result_value = result.map_err(|e| format!("runCritic should return Ok: {}", e))?;

    // MUTATION KILLER: Verify exact response structure
    assert_eq!(result_value["status"], "success", "Should have success status");
    assert!(result_value["violations"].is_array(), "Should have violations array");
    assert!(result_value["violationCount"].is_number(), "Should have violation count");
    assert!(result_value["analyzerUsed"].is_string(), "Should have analyzer used");

    // Verify analyzer used is meaningful
    let analyzer_used =
        result_value["analyzerUsed"].as_str().ok_or("analyzerUsed should be a string")?;
    assert!(
        analyzer_used == "builtin" || analyzer_used == "external",
        "Analyzer should be 'builtin' or 'external', got: {}",
        analyzer_used
    );

    // Verify violation count matches array length
    let violations =
        result_value["violations"].as_array().ok_or("violations should be an array")?;
    let violation_count =
        result_value["violationCount"].as_u64().ok_or("violationCount should be a number")?;
    assert_eq!(
        violations.len() as u64,
        violation_count,
        "Violation count should match array length"
    );

    // If there are violations, verify their structure
    if !violations.is_empty() {
        let first_violation =
            violations.first().ok_or("violations array should have at least one element")?;
        assert!(first_violation["policy"].is_string(), "Violation should have policy string");
        assert!(
            first_violation["description"].is_string(),
            "Violation should have description string"
        );
        assert!(
            first_violation["explanation"].is_string(),
            "Violation should have explanation string"
        );
        assert!(first_violation["severity"].is_number(), "Violation should have severity number");
        assert!(first_violation["line"].is_number(), "Violation should have line number");
        assert!(first_violation["column"].is_number(), "Violation should have column number");
        assert!(first_violation["file"].is_string(), "Violation should have file string");

        // MUTATION KILLER: Verify line/column numbers are positive (tests + to - mutations)
        let line = first_violation["line"].as_u64().ok_or("line should be a number")?;
        let column = first_violation["column"].as_u64().ok_or("column should be a number")?;
        assert!(line > 0, "Line number should be positive (1-based), got: {}", line);
        assert!(column > 0, "Column number should be positive (1-based), got: {}", column);

        // Verify severity is reasonable
        let severity = first_violation["severity"].as_u64().ok_or("severity should be a number")?;
        assert!((1..=5).contains(&severity), "Severity should be 1-5, got: {}", severity);
    }

    // Clean up
    fs::remove_file(temp_file).ok();
    Ok(())
}

#[test]
fn test_file_not_found_error_structure() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Test with definitely non-existent file
    let result = provider.execute_command(
        "perl.runCritic",
        vec![Value::String("/tmp/definitely_nonexistent_file_12345.pl".to_string())],
    );

    assert!(result.is_ok(), "Should handle missing files gracefully");
    let result_value = result.map_err(|e| format!("Should return Ok for missing files: {}", e))?;

    // MUTATION KILLER: Verify error response structure
    assert_eq!(result_value["status"], "error", "Should have error status");
    assert!(result_value["error"].is_string(), "Should have error message");
    assert!(result_value["violations"].is_array(), "Should have empty violations array");
    assert_eq!(result_value["violationCount"], 0, "Should have zero violation count");
    assert_eq!(result_value["analyzerUsed"], "none", "Should indicate no analyzer used");

    // Verify error message content
    let error_msg = result_value["error"].as_str().ok_or("error should be a string")?;
    assert!(error_msg.contains("File not found"), "Should indicate file not found");
    assert!(
        error_msg.contains("definitely_nonexistent_file_12345.pl"),
        "Should mention the specific file name"
    );

    // Verify violations array is empty
    let violations =
        result_value["violations"].as_array().ok_or("violations should be an array")?;
    assert!(violations.is_empty(), "Violations array should be empty for error response");
    Ok(())
}

// ============= COMMAND EXECUTION FLOW MUTATION KILLERS =============
// Target: Complex execution flow mutations and edge cases

#[test]
fn test_command_execution_success_failure_logic() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Create files for testing different execution scenarios
    let valid_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint \"success\";\n";
    let valid_file = "/tmp/test_valid_execution.pl";
    fs::write(valid_file, valid_content)?;

    // Test successful execution
    let success_result =
        provider.execute_command("perl.runFile", vec![Value::String(valid_file.to_string())]);
    assert!(success_result.is_ok(), "Valid file should execute successfully");
    let success_value = success_result.map_err(|e| format!("runFile should return Ok: {}", e))?;

    // MUTATION KILLER: Verify success logic is not negated (! deletion mutations)
    assert!(success_value["success"].is_boolean(), "Should have boolean success field");
    // Note: We can't assert the exact value since it depends on execution, but we verify structure

    // Test with subroutine execution
    let sub_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nsub test_execution { print \"sub executed\"; return 1; }\n";
    let sub_file = "/tmp/test_sub_execution.pl";
    fs::write(sub_file, sub_content)?;

    let sub_result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(sub_file.to_string()), Value::String("test_execution".to_string())],
    );
    assert!(sub_result.is_ok(), "Valid subroutine should execute successfully");
    let sub_value = sub_result.map_err(|e| format!("runTestSub should return Ok: {}", e))?;

    // Verify subroutine-specific fields
    assert!(sub_value["success"].is_boolean(), "Sub execution should have success field");
    assert!(sub_value["subroutine"].is_string(), "Sub execution should have subroutine field");
    assert_eq!(sub_value["subroutine"], "test_execution", "Should have correct subroutine name");

    // Clean up
    fs::remove_file(valid_file).ok();
    fs::remove_file(sub_file).ok();
    Ok(())
}

#[test]
fn test_comprehensive_edge_cases() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Test empty file handling
    let empty_content = "";
    let empty_file = "/tmp/test_empty_file.pl";
    fs::write(empty_file, empty_content)?;

    let empty_result =
        provider.execute_command("perl.runCritic", vec![Value::String(empty_file.to_string())]);
    assert!(empty_result.is_ok(), "Should handle empty files");
    let empty_value =
        empty_result.map_err(|e| format!("runCritic should return Ok for empty file: {}", e))?;
    assert_eq!(empty_value["status"], "success", "Empty file should be success");

    // Test very large file path
    let long_path = format!("/tmp/{}.pl", "x".repeat(100));
    let long_result =
        provider.execute_command("perl.runCritic", vec![Value::String(long_path.clone())]);

    // Should handle gracefully (either process or report not found)
    match long_result {
        Ok(response) => {
            // If it succeeds, should have proper structure
            assert!(response.get("status").is_some(), "Should have status field");
        }
        Err(error) => {
            // Should have meaningful error
            assert!(!error.is_empty(), "Error should not be empty");
        }
    }

    // Test URI format handling
    let uri_path = format!("file://{}", empty_file);
    let uri_result = provider.execute_command("perl.runCritic", vec![Value::String(uri_path)]);
    assert!(uri_result.is_ok(), "Should handle file:// URIs");
    let uri_value = uri_result.map_err(|e| format!("runCritic should return Ok for URI: {}", e))?;
    assert_eq!(uri_value["status"], "success", "URI file should be success");

    // Clean up
    fs::remove_file(empty_file).ok();
    Ok(())
}

// ============= SUPPORTED COMMANDS VALIDATION =============
// Target: get_supported_commands mutations

#[test]
fn test_supported_commands_structure() -> TestResult {
    let commands = get_supported_commands();

    // MUTATION KILLER: Verify not empty/default list
    assert!(!commands.is_empty(), "Supported commands should not be empty");
    assert_eq!(commands.len(), 5, "Should have exactly 5 supported commands");

    // Verify specific commands are present
    let expected_commands = vec![
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.debugTests",
        "perl.runCritic",
    ];

    for expected in &expected_commands {
        assert!(commands.contains(&expected.to_string()), "Should contain command: {}", expected);
    }

    // Verify all commands are strings and properly formatted
    for command in &commands {
        assert!(!command.is_empty(), "Commands should not be empty");
        assert!(command.starts_with("perl."), "Commands should start with 'perl.'");
        assert!(!command.contains(' '), "Commands should not contain spaces");
        assert_ne!(command, "xyzzy", "Should not contain placeholder values");
    }

    // Verify each command is unique
    let mut unique_commands = commands.clone();
    unique_commands.sort();
    unique_commands.dedup();
    assert_eq!(commands.len(), unique_commands.len(), "All commands should be unique");
    Ok(())
}

// ============= COMPREHENSIVE INTEGRATION TEST =============
// Target: Full workflow validation to catch complex mutation interactions

#[test]
fn test_comprehensive_workflow_validation() -> TestResult {
    let provider = ExecuteCommandProvider::new();

    // Create comprehensive test file
    let comprehensive_content = r#"#!/usr/bin/perl
use strict;
use warnings;

# Test subroutine for comprehensive testing
sub comprehensive_workflow_test {
    my $input = shift;
    print "Processing: $input\n";
    return $input * 2;
}

# Main execution
my $value = 21;
my $result = comprehensive_workflow_test($value);
print "Result: $result\n";
"#;

    let temp_file = "/tmp/comprehensive_workflow_test.pl";
    fs::write(temp_file, comprehensive_content)?;

    // Execute all commands and verify end-to-end behavior
    let all_commands = vec![
        ("perl.runFile", vec![Value::String(temp_file.to_string())]),
        ("perl.runTests", vec![Value::String(temp_file.to_string())]),
        (
            "perl.runTestSub",
            vec![
                Value::String(temp_file.to_string()),
                Value::String("comprehensive_workflow_test".to_string()),
            ],
        ),
        ("perl.debugTests", vec![Value::String(temp_file.to_string())]),
        ("perl.runCritic", vec![Value::String(temp_file.to_string())]),
    ];

    let mut all_results = Vec::new();

    for (command, args) in all_commands {
        let result = provider.execute_command(command, args);
        assert!(result.is_ok(), "Command {} should succeed in workflow test", command);

        let result_value =
            result.map_err(|e| format!("Command {} should return Ok: {}", command, e))?;

        // Verify each result is meaningful and not Default::default()
        assert!(result_value.is_object(), "Command {} should return object", command);
        assert!(
            !result_value.as_object().ok_or("Result should be an object")?.is_empty(),
            "Command {} should not be empty",
            command
        );

        all_results.push((command, result_value));
    }

    // MUTATION KILLER: Verify all results are different (no command routing bypass)
    for i in 0..all_results.len() {
        for j in (i + 1)..all_results.len() {
            let (cmd1, ref result1) = all_results[i];
            let (cmd2, ref result2) = all_results[j];

            // Results should not be identical (proves routing and execution work)
            assert_ne!(
                result1, result2,
                "Commands {} and {} should produce different results - proves no routing bypass",
                cmd1, cmd2
            );
        }
    }

    // Verify specific command behaviors in workflow
    for (command, result) in &all_results {
        match *command {
            "perl.debugTests" => {
                assert_eq!(result["success"], false, "debugTests should indicate not implemented");
            }
            "perl.runCritic" => {
                assert_eq!(result["status"], "success", "runCritic should succeed");
                assert!(result["violations"].is_array(), "runCritic should have violations");
            }
            "perl.runTestSub" => {
                assert!(
                    result["subroutine"].is_string(),
                    "runTestSub should have subroutine field"
                );
                assert_eq!(
                    result["subroutine"], "comprehensive_workflow_test",
                    "Should have correct subroutine"
                );
            }
            _ => {
                assert!(result["success"].is_boolean(), "{} should have success field", command);
            }
        }
    }

    // Clean up
    fs::remove_file(temp_file).ok();
    Ok(())
}
