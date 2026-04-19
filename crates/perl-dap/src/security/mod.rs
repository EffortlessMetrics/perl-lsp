//! Security validation module for DAP Phase 3 (AC16)
//!
//! This crate provides enterprise-grade security features:
//! - Path traversal prevention
//! - Input validation for expressions and conditions
//! - Resource limits enforcement
//! - Secure defaults
//!
//! # Safety Guarantees
//!
//! - All file paths are validated against workspace boundaries
//! - Expressions cannot contain newlines (protocol injection prevention)
//! - Timeouts are capped at reasonable limits
//! - Dangerous operations are blocked in safe evaluation mode

use perl_parser_core::path_security::{WorkspacePathError, validate_workspace_path};
use std::path::{Path, PathBuf};

/// Security validation errors
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SecurityError {
    /// Path traversal attempt detected
    #[error("Path traversal attempt detected: {0}")]
    PathTraversalAttempt(String),

    /// Path outside workspace boundary
    #[error("Path outside workspace: {0}")]
    PathOutsideWorkspace(String),

    /// Symlink resolves outside workspace
    #[error("Symlink resolves outside workspace: {0}")]
    SymlinkOutsideWorkspace(String),

    /// Invalid path characters (null bytes, control characters)
    #[error("Invalid path characters detected")]
    InvalidPathCharacters,

    /// Expression contains newlines (protocol injection risk)
    #[error("Expression cannot contain newlines")]
    InvalidExpression,

    /// Timeout exceeds maximum allowed value
    #[error("Timeout exceeds maximum allowed value: {0}ms")]
    ExcessiveTimeout(u32),
}

/// Maximum allowed timeout in milliseconds (5 minutes)
pub const MAX_TIMEOUT_MS: u32 = 300_000;

/// Default timeout in milliseconds (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u32 = 5_000;

impl From<WorkspacePathError> for SecurityError {
    fn from(error: WorkspacePathError) -> Self {
        match error {
            WorkspacePathError::PathTraversalAttempt(message) => {
                Self::PathTraversalAttempt(message)
            }
            WorkspacePathError::PathOutsideWorkspace(message) => {
                Self::PathOutsideWorkspace(message)
            }
            WorkspacePathError::InvalidPathCharacters => Self::InvalidPathCharacters,
        }
    }
}

/// Validate that a path is within the workspace boundary
pub fn validate_path(path: &Path, workspace_root: &Path) -> Result<PathBuf, SecurityError> {
    validate_workspace_path(path, workspace_root).map_err(SecurityError::from)
}

/// Validate an expression for safe evaluation
pub fn validate_expression(expression: &str) -> Result<(), SecurityError> {
    if expression.contains('\n') || expression.contains('\r') {
        return Err(SecurityError::InvalidExpression);
    }

    Ok(())
}

/// Validate a timeout value, returning an error if it exceeds the maximum allowed.
pub fn validate_timeout(timeout_ms: u32) -> Result<u32, SecurityError> {
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(SecurityError::ExcessiveTimeout(timeout_ms));
    }
    Ok(timeout_ms.max(1))
}

/// Validate a breakpoint condition for security issues
pub fn validate_condition(condition: &str) -> Result<(), SecurityError> {
    validate_expression(condition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs;

    #[test]
    fn test_validate_path_within_workspace() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let safe_path = PathBuf::from("src/main.pl");
        let result = validate_path(&safe_path, workspace);

        assert!(result.is_ok(), "Path within workspace should be valid");
        Ok(())
    }

    #[test]
    fn test_validate_path_parent_traversal() -> Result<()> {
        use perl_tdd_support::must;
        let tempdir = must(tempfile::tempdir());
        let workspace = tempdir.path();

        let unsafe_path = PathBuf::from("../../../etc/passwd");
        let result = validate_path(&unsafe_path, workspace);

        assert!(result.is_err(), "Parent traversal should be rejected");

        match result {
            Err(SecurityError::PathTraversalAttempt(_))
            | Err(SecurityError::PathOutsideWorkspace(_)) => {}
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected PathTraversalAttempt or PathOutsideWorkspace error, got: {:?}",
                    e
                ));
            }
            Ok(_) => return Err(anyhow::anyhow!("Expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn test_validate_path_absolute_outside() -> Result<()> {
        use perl_tdd_support::{must, must_some};
        let tempdir = must(tempfile::tempdir());
        let workspace = tempdir.path();

        let tempdir2 = must(tempfile::tempdir());
        let outside_file = tempdir2.path().join("outside.pl");
        must(fs::write(&outside_file, "print 'outside';"));

        let result = validate_path(&outside_file, workspace);

        assert!(
            result.is_err(),
            "Absolute path outside workspace should be rejected"
        );

        match result {
            Err(SecurityError::PathOutsideWorkspace(_))
            | Err(SecurityError::PathTraversalAttempt(_)) => {}
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected PathOutsideWorkspace or PathTraversalAttempt error, got: {:?}",
                    e
                ));
            }
            Ok(_) => return Err(anyhow::anyhow!("Expected error, got Ok")),
        }

        let _ = must_some(outside_file.to_str());
        Ok(())
    }

    #[test]
    fn test_validate_path_with_null_byte() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let invalid_path = PathBuf::from("file\0name.pl");
        let result = validate_path(&invalid_path, workspace);

        assert!(result.is_err(), "Path with null byte should be rejected");
        assert!(
            matches!(result, Err(SecurityError::InvalidPathCharacters)),
            "Expected InvalidPathCharacters error"
        );
        Ok(())
    }

    #[test]
    fn test_validate_path_with_control_character() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let invalid_path = PathBuf::from("file\x1fname.pl");
        let result = validate_path(&invalid_path, workspace);

        assert!(
            result.is_err(),
            "Path with control character should be rejected"
        );
        assert!(
            matches!(result, Err(SecurityError::InvalidPathCharacters)),
            "Expected InvalidPathCharacters error"
        );
        Ok(())
    }

    #[test]
    fn test_validate_path_relative_dotdot_within_workspace() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        fs::create_dir_all(workspace.join("subdir"))?;
        let safe_path = PathBuf::from("subdir/../main.pl");
        let result = validate_path(&safe_path, workspace);

        assert!(
            result.is_ok(),
            "Normalized path within workspace should be valid"
        );
        Ok(())
    }

    #[test]
    fn test_validate_expression_valid() -> Result<()> {
        validate_expression("$x + 1")?;
        validate_expression("my_function()")?;
        validate_expression("$hash{key}")?;
        Ok(())
    }

    #[test]
    fn test_validate_expression_newline() -> Result<()> {
        let result = validate_expression("1\nprint 'hacked'");

        assert!(
            result.is_err(),
            "Expression with newline should be rejected"
        );
        assert!(
            matches!(result, Err(SecurityError::InvalidExpression)),
            "Expected InvalidExpression error"
        );
        Ok(())
    }

    #[test]
    fn test_validate_expression_carriage_return() {
        let result = validate_expression("1\rprint 'hacked'");
        assert!(
            result.is_err(),
            "Expression with carriage return should be rejected"
        );
        assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    }

    #[test]
    fn test_validate_timeout_within_bounds() -> Result<()> {
        assert_eq!(validate_timeout(1000)?, 1000);
        assert_eq!(validate_timeout(5000)?, 5000);
        assert_eq!(validate_timeout(100_000)?, 100_000);
        Ok(())
    }

    #[test]
    fn test_validate_timeout_zero() -> Result<()> {
        assert_eq!(
            validate_timeout(0)?,
            1,
            "Zero timeout should be clamped to 1ms"
        );
        Ok(())
    }

    #[test]
    fn test_validate_timeout_excessive() {
        use perl_tdd_support::must_err;
        let result = validate_timeout(500_000);
        assert!(result.is_err(), "Excessive timeout should be an error");
        assert_eq!(must_err(result), SecurityError::ExcessiveTimeout(500_000));
        assert!(validate_timeout(1_000_000).is_err());
    }

    #[test]
    fn test_validate_timeout_boundary_at_max_is_ok() -> Result<()> {
        assert!(validate_timeout(MAX_TIMEOUT_MS).is_ok());
        assert_eq!(validate_timeout(MAX_TIMEOUT_MS)?, MAX_TIMEOUT_MS);
        Ok(())
    }

    #[test]
    fn test_validate_timeout_one_over_max_is_error() {
        use perl_tdd_support::must_err;
        assert!(validate_timeout(MAX_TIMEOUT_MS + 1).is_err());
        assert_eq!(
            must_err(validate_timeout(MAX_TIMEOUT_MS + 1)),
            SecurityError::ExcessiveTimeout(MAX_TIMEOUT_MS + 1)
        );
    }

    #[test]
    fn test_validate_condition_valid() -> Result<()> {
        validate_condition("$x > 10")?;
        validate_condition("defined($var)")?;
        Ok(())
    }

    #[test]
    fn test_validate_condition_newline() -> Result<()> {
        let result = validate_condition("1\nprint 'pwned'");

        assert!(result.is_err(), "Condition with newline should be rejected");
        assert!(
            matches!(result, Err(SecurityError::InvalidExpression)),
            "Expected InvalidExpression error"
        );
        Ok(())
    }

    #[test]
    fn test_validate_path_empty_string() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let empty_path = PathBuf::from("");
        let result = validate_path(&empty_path, workspace);

        assert!(
            result.is_ok(),
            "Empty path should resolve to workspace root"
        );
        Ok(())
    }

    #[test]
    fn test_validate_expression_empty_string() -> Result<()> {
        validate_expression("")?;
        Ok(())
    }

    #[test]
    fn test_validate_timeout_boundary_values() -> Result<()> {
        assert_eq!(validate_timeout(1)?, 1);
        assert_eq!(validate_timeout(MAX_TIMEOUT_MS)?, MAX_TIMEOUT_MS);
        assert!(validate_timeout(MAX_TIMEOUT_MS + 1).is_err());
        Ok(())
    }

    #[test]
    fn test_security_error_display_messages() {
        let path_error = SecurityError::PathTraversalAttempt("../../../etc/passwd".to_string());
        assert!(format!("{}", path_error).contains("Path traversal attempt detected"));

        let expr_error = SecurityError::InvalidExpression;
        assert_eq!(
            format!("{}", expr_error),
            "Expression cannot contain newlines"
        );

        let timeout_error = SecurityError::ExcessiveTimeout(500_000);
        assert!(format!("{}", timeout_error).contains("500000ms"));
    }
}
