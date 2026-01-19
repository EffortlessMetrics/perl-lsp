//! Security regression tests for executeCommand.
//!
//! These tests verify that command injection vulnerabilities in run_test_sub,
//! run_tests, and run_file have been properly mitigated.

use perl_lsp::execute_command::ExecuteCommandProvider;
use serde_json::Value;

/// Test that run_test_sub is protected against code injection via file_path.
///
/// The vulnerable code previously constructed:
/// `do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}`
/// which allowed injection through the file_path parameter.
#[test]
fn test_run_test_sub_file_path_injection() {
    let provider = ExecuteCommandProvider::new();

    // Payload that would inject code if string interpolation is used
    let malicious_file_path = "nonexistent.pl'; print 'INJECTED_VIA_FILE'; '";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(malicious_file_path.to_string()), Value::String("somesub".to_string())],
    );

    // Command should execute (Ok) but output must NOT contain injected code
    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result.unwrap();
    let output = val["output"].as_str().unwrap_or("");
    let error = val["error"].as_str().unwrap_or("");

    assert!(
        !output.contains("INJECTED_VIA_FILE"),
        "Vulnerability: code injection via file_path succeeded! Output: {}",
        output
    );
    assert!(
        !error.contains("INJECTED_VIA_FILE"),
        "Vulnerability: code injection via file_path succeeded! Error: {}",
        error
    );
}

/// Test that run_test_sub is protected against code injection via sub_name.
///
/// Note: The sub_name is passed via @ARGV, so the malicious string is treated
/// as a literal subroutine name to look up, not as code to execute. The error
/// message will contain the literal name (safe behavior), but the injected
/// code will NOT be executed.
#[test]
fn test_run_test_sub_subname_injection() {
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

    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result.unwrap();
    let output = val["output"].as_str().unwrap_or("");

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
    let success = val["success"].as_bool().unwrap_or(true);
    assert!(!success, "Command should have failed (subroutine not found)");
}

/// Test that run_file is protected against argument injection via file_path.
///
/// A file path starting with `-` could be interpreted as a flag without `--`.
#[test]
fn test_run_file_argument_injection() {
    let provider = ExecuteCommandProvider::new();

    // Payload that would be interpreted as a flag without `--` separator
    // `-e print 'INJECTED'` would execute arbitrary code
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runFile", vec![Value::String(malicious_file_path.to_string())]);

    // The command should fail gracefully (file doesn't exist or is treated as literal)
    // but should NOT execute `-e` as a flag
    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result.unwrap();

    // If `-e` was interpreted as a flag, perl would complain about missing argument
    // or execute code. With `--`, it's treated as a filename.
    let error = val["error"].as_str().unwrap_or("");
    assert!(
        !error.contains("No code specified for -e"),
        "Vulnerability: -e was interpreted as a flag, not a filename"
    );
}

/// Test that run_tests is protected against argument injection via file_path.
#[test]
fn test_run_tests_argument_injection() {
    let provider = ExecuteCommandProvider::new();

    // Similar test for run_tests
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runTests", vec![Value::String(malicious_file_path.to_string())]);

    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result.unwrap();

    let error = val["error"].as_str().unwrap_or("");
    assert!(
        !error.contains("No code specified for -e"),
        "Vulnerability: -e was interpreted as a flag in run_tests"
    );
}

/// Test that file paths with shell metacharacters don't cause issues.
#[test]
fn test_shell_metacharacter_safety() {
    let provider = ExecuteCommandProvider::new();

    // File paths with shell metacharacters that could cause issues
    // if shell expansion occurred
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

        // Should execute without shell expansion
        assert!(result.is_ok(), "Command should not fail for path: {}", path);
    }
}
