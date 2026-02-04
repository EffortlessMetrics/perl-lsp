//! Security regression tests for executeCommand.
//!
//! These tests verify that command injection vulnerabilities in run_test_sub,
//! run_tests, and run_file have been properly mitigated.
//!
//! Note: With secure path resolution, malicious/non-existent paths are rejected
//! early at the path validation stage (returning Err), preventing execution entirely.

use perl_lsp::execute_command::ExecuteCommandProvider;
use serde_json::Value;
use std::error::Error;
use std::fs;
use tempfile::TempDir;

/// Test that run_test_sub is protected against code injection via file_path.
///
/// The vulnerable code previously constructed:
/// `do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}`
/// which allowed injection through the file_path parameter.
///
/// With secure path resolution, non-existent files are rejected before execution.
/// The key security property is that the malicious code never reaches Perl.
#[test]
fn test_run_test_sub_file_path_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Payload that would inject code if string interpolation is used
    let malicious_file_path = "nonexistent.pl'; print 'INJECTED_VIA_FILE'; '";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(malicious_file_path.to_string()), Value::String("somesub".to_string())],
    );

    // With secure path resolution, non-existent files are rejected early
    // BEFORE any shell command or Perl code is executed
    assert!(result.is_err(), "Malicious path should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path resolution (file not found/canonicalize failure)
    // NOT about Perl code execution or subroutine lookup
    assert!(
        err.contains("canonicalize") || err.contains("Failed to"),
        "Error should be about path validation, not code execution: {}",
        err
    );

    // The path may be echoed in the error (this is fine - it's just a filename),
    // but the key is that no Perl code was executed with this malicious string.
    // The secure path resolution catches it at the Rust layer.
    Ok(())
}

/// Test that run_test_sub is protected against code injection via sub_name.
///
/// Note: The sub_name is passed via @ARGV, so the malicious string is treated
/// as a literal subroutine name to look up, not as code to execute. The error
/// message will contain the literal name (safe behavior), but the injected
/// code will NOT be executed.
#[test]
fn test_run_test_sub_subname_injection() -> Result<(), Box<dyn Error>> {
    // Create a minimal test file with a marker subroutine
    let test_file = "/tmp/security_test_sub.pl";
    std::fs::write(test_file, "sub safe_sub { print 'SAFE_SUB_EXECUTED'; }").ok();

    let provider = ExecuteCommandProvider::with_security_context(
        vec![],
        vec![std::path::PathBuf::from(test_file)],
    );

    // This payload would execute code if string interpolation was used.
    // With the fix (using @ARGV), it's treated as a literal subroutine name.
    let malicious_sub_name = "safe_sub(); print 'INJECTED_CODE_RAN'";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(test_file.to_string()), Value::String(malicious_sub_name.to_string())],
    );

    // Clean up
    std::fs::remove_file(test_file).ok();

    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result?;
    let output = val["output"].as_str().ok_or("Missing 'output' field")?;

    // Key assertions:
    // 1. The injected print statement should NOT have executed
    assert!(
        !output.contains("INJECTED_CODE_RAN"),
        "Vulnerability: code injection via sub_name succeeded! Output: {}",
        output
    );

    // 2. The safe_sub should NOT have been called either (the malicious name
    //    includes "safe_sub()" but that should be treated literally, not executed)
    assert!(
        !output.contains("SAFE_SUB_EXECUTED"),
        "Unexpected: safe_sub was called despite malicious sub_name. Output: {}",
        output
    );

    // 3. The command should have failed because no subroutine with that literal name exists
    let success = val["success"].as_bool().ok_or("Missing 'success' field")?;
    assert!(!success, "Command should have failed (subroutine not found)");
    Ok(())
}

/// Test that run_file is protected against argument injection via file_path.
///
/// A file path starting with `-` could be interpreted as a flag without `--`.
/// With secure path resolution, non-existent files are rejected before execution.
#[test]
fn test_run_file_argument_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Payload that would be interpreted as a flag without `--` separator
    // `-e print 'INJECTED'` would execute arbitrary code
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runFile", vec![Value::String(malicious_file_path.to_string())]);

    // With secure path resolution, non-existent files are rejected early
    assert!(result.is_err(), "Malicious path '-e' should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path validation
    assert!(
        err.contains("canonicalize") || err.contains("not found"),
        "Error should be about path validation: {}",
        err
    );
    Ok(())
}

/// Test that run_tests is protected against argument injection via file_path.
/// With secure path resolution, non-existent files are rejected before execution.
#[test]
fn test_run_tests_argument_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Similar test for run_tests
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runTests", vec![Value::String(malicious_file_path.to_string())]);

    // With secure path resolution, non-existent files are rejected early
    assert!(result.is_err(), "Malicious path '-e' should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path validation
    assert!(
        err.contains("canonicalize") || err.contains("not found"),
        "Error should be about path validation: {}",
        err
    );
    Ok(())
}

/// Test that file paths with shell metacharacters are safely rejected.
///
/// With secure path resolution, files that don't exist are rejected before
/// any shell command is executed, preventing shell metacharacter expansion.
#[test]
fn test_shell_metacharacter_safety() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // File paths with shell metacharacters that could cause issues
    // if shell expansion occurred. These non-existent paths should be
    // rejected during path validation before any shell execution.
    let dangerous_paths = vec![
        "/tmp/test$(whoami).pl",
        "/tmp/test`id`.pl",
        "/tmp/test;rm -rf /.pl",
        "/tmp/test|cat /etc/passwd.pl",
        "/tmp/test&& echo pwned.pl",
    ];

    for path in dangerous_paths {
        let result =
            provider.execute_command("perl.runFile", vec![Value::String(path.to_string())]);

        // Non-existent paths should be rejected during path resolution
        assert!(result.is_err(), "Non-existent path should be rejected: {}", path);
        let err = result.err().ok_or("Expected error but got Ok")?;

        // Error should be about path validation, not shell execution
        assert!(
            err.contains("canonicalize") || err.contains("not found"),
            "Error should be about path validation for {}: {}",
            path,
            err
        );
    }
    Ok(())
}

/// Test that valid files with safe paths execute correctly.
#[test]
fn test_valid_file_execution() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_valid.pl");
    fs::write(&file_path, "print 'VALID_OUTPUT';")?;

    let provider =
        ExecuteCommandProvider::with_security_context(vec![], vec![file_path.clone()]);

    let result = provider.execute_command(
        "perl.runFile",
        vec![Value::String(file_path.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "Valid file should execute successfully");
    let val = result?;
    let output = val["output"].as_str().ok_or("Missing 'output' field")?;

    assert!(output.contains("VALID_OUTPUT"), "Output should contain expected result: {}", output);
    Ok(())
}
