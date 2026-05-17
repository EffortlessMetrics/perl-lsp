//! Security validation tests for perl-dap (AC16)
//!
//! These tests verify the implementation of enterprise security features:
//! - Path traversal prevention
//! - Input validation
//! - Resource limits
//! - Secure defaults

use perl_dap::security::{
    DEFAULT_TIMEOUT_MS, SecurityError, validate_condition, validate_expression, validate_path,
    validate_timeout,
};
use std::path::PathBuf;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn temp_workspace() -> Result<TempDir, Box<dyn std::error::Error>> {
    Ok(tempfile::Builder::new().prefix("perl-dap-security-").tempdir()?)
}

// ===== Path Validation Tests =====

#[test]
fn test_path_validation_safe_relative_paths() -> TestResult {
    let workspace = temp_workspace()?;

    // Safe relative paths
    let safe_paths = vec!["src/main.pl", "./lib/Module.pm", "test.pl"];

    for path_str in safe_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, workspace.path());
        assert!(
            result.is_ok(),
            "Path '{}' should be valid within workspace, got error: {:?}",
            path_str,
            result
        );
    }

    Ok(())
}

#[test]
fn test_path_validation_parent_traversal_attempts() -> TestResult {
    let workspace = temp_workspace()?;

    // Malicious paths with parent directory references
    let malicious_paths =
        vec!["../../../etc/passwd", "../../.ssh/id_rsa", "../../../../../../../etc/shadow"];

    for path_str in malicious_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, workspace.path());

        assert!(
            result.is_err(),
            "Parent traversal path '{}' should be rejected (workspace: {}), result: {:?}",
            path_str,
            workspace.path().display(),
            result
        );

        match result {
            Err(
                SecurityError::PathTraversalAttempt(_) | SecurityError::PathOutsideWorkspace(_),
            ) => {}
            Err(error) => {
                return Err(format!(
                    "Expected PathTraversalAttempt or PathOutsideWorkspace error for '{path_str}', got: {error:?}"
                )
                .into());
            }
            Ok(path) => {
                return Err(format!(
                    "Parent traversal path '{path_str}' unexpectedly resolved to {}",
                    path.display()
                )
                .into());
            }
        }
    }

    Ok(())
}

#[test]
fn test_path_validation_absolute_paths() -> TestResult {
    let workspace = temp_workspace()?;

    // Absolute paths outside workspace should be rejected
    let outside_paths = vec!["/etc/passwd", "/root/.ssh/id_rsa"];

    for path_str in outside_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, workspace.path());
        assert!(
            result.is_err(),
            "Absolute path '{}' outside workspace should be rejected",
            path_str
        );
    }

    Ok(())
}

#[test]
fn test_path_validation_null_byte_injection() -> TestResult {
    let workspace = PathBuf::from("/workspace");

    // Null byte injection attempts
    let path = PathBuf::from("valid.pl\0../../etc/passwd");
    let result = validate_path(&path, &workspace);

    assert!(result.is_err(), "Null byte injection should be rejected");

    match result {
        Err(SecurityError::InvalidPathCharacters) => Ok(()),
        Err(error) => Err(format!("Expected InvalidPathCharacters error, got: {error:?}").into()),
        Ok(path) => {
            Err(format!("Null byte injection unexpectedly resolved to {}", path.display()).into())
        }
    }
}

// ===== Expression Validation Tests =====

#[test]
fn test_expression_validation_valid_expressions() -> TestResult {
    let valid_exprs = vec!["$x + 1", "$hash{key}", "my_function()", "defined($var)", "$array[0]"];

    for expr in valid_exprs {
        validate_expression(expr)?;
    }

    Ok(())
}

#[test]
fn test_expression_validation_newline_injection() -> TestResult {
    let malicious_exprs = vec!["1\nprint 'hacked'", "$x\nsystem('rm -rf /')", "valid\rmalicious"];

    for expr in malicious_exprs {
        let result = validate_expression(expr);
        assert!(
            result.is_err(),
            "Expression with newlines '{}' should be rejected",
            expr.escape_default()
        );

        match result {
            Err(SecurityError::InvalidExpression) => {}
            Err(error) => {
                return Err(format!("Expected InvalidExpression error, got: {error:?}").into());
            }
            Ok(()) => {
                return Err(format!(
                    "Expression '{}' unexpectedly validated",
                    expr.escape_default()
                )
                .into());
            }
        }
    }

    Ok(())
}

// ===== Condition Validation Tests =====

#[test]
fn test_condition_validation_safe_conditions() -> TestResult {
    let valid_conditions = vec!["$x > 10", "defined($var)", "$count == 5", "$name eq 'test'"];

    for cond in valid_conditions {
        validate_condition(cond)?;
    }

    Ok(())
}

#[test]
fn test_condition_validation_protocol_injection() -> TestResult {
    // Protocol injection attempts in breakpoint conditions
    let malicious_conditions = vec!["1; print \"PWNED\"\n", "$x > 10\nsystem('ls')"];

    for cond in malicious_conditions {
        let result = validate_condition(cond);
        assert!(result.is_err(), "Malicious condition '{}' should be rejected", cond);
    }

    Ok(())
}

// ===== Timeout Validation Tests =====

#[test]
fn test_timeout_validation_within_bounds() -> TestResult {
    assert_eq!(validate_timeout(1000)?, 1000);
    assert_eq!(validate_timeout(5000)?, 5000);
    assert_eq!(validate_timeout(100_000)?, 100_000);
    assert_eq!(validate_timeout(DEFAULT_TIMEOUT_MS)?, DEFAULT_TIMEOUT_MS);

    Ok(())
}

#[test]
fn test_timeout_validation_zero_clamped() -> TestResult {
    assert_eq!(validate_timeout(0)?, 1, "Zero timeout should be clamped to 1ms");

    Ok(())
}

#[test]
fn test_timeout_validation_excessive_returns_error() {
    assert!(validate_timeout(500_000).is_err(), "Excessive timeout should be an error");
    assert!(validate_timeout(1_000_000).is_err(), "Million ms timeout should be an error");
}

// ===== Integration Tests =====

#[test]
fn test_security_comprehensive_path_traversal_matrix() -> TestResult {
    // Test matrix from fixtures/security/path_traversal_attempts.json
    let test_cases = vec![
        ("../../../etc/passwd", true),
        ("/etc/passwd", true),
        ("./lib/MyModule.pm", false),
        ("./tests/fixtures/hello.pl", false),
        ("script.pl", false),
        ("test.pl", false),
    ];

    let workspace = temp_workspace()?;

    for (path_str, should_reject) in test_cases {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, workspace.path());

        if should_reject {
            assert!(result.is_err(), "Path '{}' should be rejected but was allowed", path_str);
        } else {
            // For non-rejecting paths, they should pass (we're validating structure, not existence)
            assert!(
                result.is_ok(),
                "Path '{}' should be valid within workspace, got error: {:?}",
                path_str,
                result
            );
        }
    }

    Ok(())
}

#[test]
fn test_security_unicode_safety() {
    // AC16.4: Unicode boundary safety
    let expr_with_emoji = "my $var = '🚀';";

    // Should not reject valid Unicode
    assert!(validate_expression(expr_with_emoji).is_ok());

    // But should still reject newlines after Unicode
    let malicious = "my $var = '🚀';\nprint 'hacked'";
    assert!(validate_expression(malicious).is_err());
}
