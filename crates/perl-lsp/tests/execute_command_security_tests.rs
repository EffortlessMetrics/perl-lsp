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
    let provider = ExecuteCommandProvider::new();

    // Create a minimal test file with a marker subroutine
    let test_file = "/tmp/security_test_sub.pl";
    std::fs::write(test_file, "sub safe_sub { print 'SAFE_SUB_EXECUTED'; }").ok();

    // This payload would execute code if string interpolation was used.
    // With the fix (using @ARGV), it's treated as a literal subroutine name.
    let malicious_sub_name = "safe_sub(); print 'INJECTED_CODE_RAN'";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(test_file.to_string()), Value::String(malicious_sub_name.to_string())],
    );

    // Clean up
    std::fs::remove_file(test_file).ok();

    // With enhanced validation, invalid identifier characters are rejected early
    assert!(result.is_err(), "Malicious subroutine name should be rejected by validation");
    let err = result.err().ok_or("Expected error but got Ok")?;

    assert!(
        err.contains("Invalid subroutine name"),
        "Error should be about invalid identifier: {}",
        err
    );
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

    let provider = ExecuteCommandProvider::new();

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

/// Test that run_test_sub blocks attempts to call CORE::exit directly.
#[test]
fn test_block_core_exit() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Create a dummy file
    let temp_dir = TempDir::new()?;
    let temp_file = temp_dir.path().join("repro_core_exit.pl");
    fs::write(&temp_file, "sub normal_sub { print 'normal'; }")?;

    // Try to call CORE::exit via sub_name
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(temp_file.to_string_lossy().to_string()),
            Value::String("CORE::exit".to_string())
        ]
    );

    // Should FAIL with security error
    assert!(result.is_err(), "Should block CORE::exit");
    let err = result.err().ok_or("Expected error")?;
    assert!(err.contains("Security: Calling CORE:: functions directly is not allowed"), "Unexpected error: {}", err);
    Ok(())
}

/// Test that run_test_sub blocks attempts to call CORE::dump directly.
#[test]
fn test_block_core_dump() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Create a dummy file
    let temp_dir = TempDir::new()?;
    let temp_file = temp_dir.path().join("repro_core_dump.pl");
    fs::write(&temp_file, "sub normal_sub { print 'normal'; }")?;

    // Try to call CORE::dump
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(temp_file.to_string_lossy().to_string()),
            Value::String("CORE::dump".to_string())
        ]
    );

    assert!(result.is_err(), "Should block CORE::dump");
    let err = result.err().ok_or("Expected error")?;
    assert!(err.contains("Security: Calling CORE:: functions directly is not allowed"), "Unexpected error: {}", err);
    Ok(())
}

/// Test that run_test_sub blocks invalid identifiers.
#[test]
fn test_block_invalid_identifier() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();
    let temp_dir = TempDir::new()?;
    let temp_file = temp_dir.path().join("repro_invalid.pl");
    fs::write(&temp_file, "sub normal { }")?;

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(temp_file.to_string_lossy().to_string()),
            Value::String("invalid-name!".to_string())
        ]
    );

    assert!(result.is_err(), "Should block invalid identifier");
    let err = result.err().ok_or("Expected error")?;
    assert!(err.contains("Invalid subroutine name"), "Unexpected error: {}", err);
    Ok(())
}

/// Test that run_test_sub allows valid package-qualified identifiers.
#[test]
fn test_allow_valid_identifiers() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();
    let temp_dir = TempDir::new()?;
    let temp_file = temp_dir.path().join("test_valid.pl");
    fs::write(&temp_file, "package Foo; sub bar { print 'ok'; }")?;

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(temp_file.to_string_lossy().to_string()),
            Value::String("Foo::bar".to_string())
        ]
    );

    assert!(result.is_ok(), "Valid package-qualified sub should work");
    Ok(())
}
