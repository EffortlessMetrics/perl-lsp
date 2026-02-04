//! Security validation tests for perl-dap (AC16)
//!
//! These tests verify the implementation of enterprise security features:
//! - Path traversal prevention
//! - Input validation
//! - Resource limits
//! - Secure defaults

use perl_dap::security::{
    DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression,
    validate_path, validate_timeout,
};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ===== Path Validation Tests =====

#[test]
fn test_path_validation_safe_relative_paths() -> TestResult {
    let workspace = std::env::current_dir()?.join("test_workspace");
    std::fs::create_dir_all(&workspace)?;

    // Safe relative paths
    let safe_paths = vec!["src/main.pl", "./lib/Module.pm", "test.pl", ".gitignore"];

    for path_str in safe_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, &workspace);
        assert!(result.is_ok(), "Path '{}' should be valid within workspace", path_str);
    }

    std::fs::remove_dir_all(&workspace).ok();
    Ok(())
}

#[test]
fn test_path_validation_parent_traversal_attempts() {
    let workspace = std::env::current_dir().expect("Failed to get cwd").join("test_workspace");
    std::fs::create_dir_all(&workspace).ok();

    // Malicious paths with parent directory references
    let malicious_paths =
        vec!["../../../etc/passwd", "../../.ssh/id_rsa", "../../../../../../../etc/shadow"];

    for path_str in malicious_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, &workspace);

        if result.is_ok() {
            eprintln!(
                "DEBUG: Path '{}' was ALLOWED (workspace: {})",
                path_str,
                workspace.display()
            );
            eprintln!("DEBUG: Result: {:?}", result);
        }

        assert!(
            result.is_err(),
            "Parent traversal path '{}' should be rejected (workspace: {}), result: {:?}",
            path_str,
            workspace.display(),
            result
        );

        // Verify it's the right error type
        if let Err(e) = result {
            match e {
                SecurityError::PathTraversalAttempt(_) | SecurityError::PathOutsideWorkspace(_) => {}
                _ => panic!("Expected PathTraversalAttempt or PathOutsideWorkspace error for '{}', got: {:?}", path_str, e),
            }
        }
    }

    std::fs::remove_dir_all(&workspace).ok();
}

#[test]
fn test_path_validation_absolute_paths() {
    let workspace = std::env::current_dir().expect("Failed to get cwd").join("test_workspace");
    std::fs::create_dir_all(&workspace).ok();

    // Absolute paths outside workspace should be rejected
    let outside_paths = vec!["/etc/passwd", "/root/.ssh/id_rsa"];

    for path_str in outside_paths {
        let path = PathBuf::from(path_str);
        let result = validate_path(&path, &workspace);
        assert!(
            result.is_err(),
            "Absolute path '{}' outside workspace should be rejected",
            path_str
        );
    }

    std::fs::remove_dir_all(&workspace).ok();
}

#[test]
fn test_path_validation_null_byte_injection() {
    let workspace = PathBuf::from("/workspace");

    // Null byte injection attempts
    let path = PathBuf::from("valid.pl\0../../etc/passwd");
    let result = validate_path(&path, &workspace);

    assert!(result.is_err(), "Null byte injection should be rejected");

    match result {
        Err(SecurityError::InvalidPathCharacters) => {}
        _ => panic!("Expected InvalidPathCharacters error"),
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
fn test_expression_validation_newline_injection() {
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
            _ => panic!("Expected InvalidExpression error"),
        }
    }
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
fn test_condition_validation_protocol_injection() {
    // Protocol injection attempts in breakpoint conditions
    let malicious_conditions = vec!["1; print \"PWNED\"\n", "$x > 10\nsystem('ls')"];

    for cond in malicious_conditions {
        let result = validate_condition(cond);
        assert!(result.is_err(), "Malicious condition '{}' should be rejected", cond);
    }
}

// ===== Timeout Validation Tests =====

#[test]
fn test_timeout_validation_within_bounds() {
    assert_eq!(validate_timeout(1000), 1000);
    assert_eq!(validate_timeout(5000), 5000);
    assert_eq!(validate_timeout(100_000), 100_000);
    assert_eq!(validate_timeout(DEFAULT_TIMEOUT_MS), DEFAULT_TIMEOUT_MS);
}

#[test]
fn test_timeout_validation_zero_clamped() {
    assert_eq!(validate_timeout(0), 1, "Zero timeout should be clamped to 1ms");
}

#[test]
fn test_timeout_validation_excessive_capped() {
    assert_eq!(validate_timeout(500_000), MAX_TIMEOUT_MS, "Excessive timeout should be capped");
    assert_eq!(validate_timeout(1_000_000), MAX_TIMEOUT_MS, "Million ms timeout should be capped");
}

// ===== Integration Tests =====

#[test]
fn test_security_comprehensive_path_traversal_matrix() {
    // Test matrix from fixtures/security/path_traversal_attempts.json
    let test_cases = vec![
        ("../../../etc/passwd", true),
        ("/etc/passwd", true),
        ("./lib/MyModule.pm", false),
        ("./tests/fixtures/hello.pl", false),
        ("script.pl", false),
        ("./.gitignore", false),
    ];

    let workspace =
        std::env::current_dir().expect("Failed to get cwd").join("test_workspace_comprehensive");

    for (path_str, should_reject) in test_cases {
        // Ensure workspace exists for each test case
        std::fs::create_dir_all(&workspace).expect("Failed to create workspace");

        let path = PathBuf::from(path_str);
        let result = validate_path(&path, &workspace);

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

    std::fs::remove_dir_all(&workspace).ok();
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
