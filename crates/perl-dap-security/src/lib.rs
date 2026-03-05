//! Security validation module for DAP Phase 3 (AC16)
//!
//! This crate provides enterprise-grade security features:
//! - Path traversal prevention
//! - Input validation for expressions and conditions
//! - Resource limits enforcement
//! - Secure defaults

use anyhow::Result;
use perl_path_security::{WorkspacePathError, validate_workspace_path};
use std::path::{Path, PathBuf};

/// Security validation errors
#[derive(Debug, thiserror::Error)]
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

/// Validate that a path is within the workspace boundary.
pub fn validate_path(path: &Path, workspace_root: &Path) -> Result<PathBuf, SecurityError> {
    validate_workspace_path(path, workspace_root).map_err(SecurityError::from)
}

/// Validate an expression for safe evaluation.
pub fn validate_expression(expression: &str) -> Result<(), SecurityError> {
    if expression.contains('\n') || expression.contains('\r') {
        return Err(SecurityError::InvalidExpression);
    }

    Ok(())
}

/// Validate and cap a timeout value.
pub fn validate_timeout(timeout_ms: u32) -> u32 {
    let timeout = timeout_ms.max(1);
    timeout.min(MAX_TIMEOUT_MS)
}

/// Validate a breakpoint condition for security issues.
pub fn validate_condition(condition: &str) -> Result<(), SecurityError> {
    validate_expression(condition)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let workspace = must(std::env::current_dir()).join("test_workspace");
        fs::create_dir_all(&workspace).ok();

        let unsafe_path = must_some(workspace.parent()).join("etc/passwd");
        let result = validate_path(&unsafe_path, &workspace);

        fs::remove_dir(&workspace).ok();

        assert!(
            result.is_err(),
            "Absolute path outside workspace should be rejected: {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn test_validate_path_null_byte() -> Result<()> {
        let workspace = PathBuf::from("/workspace");
        let unsafe_path = PathBuf::from("valid.pl\0../../etc/passwd");

        let result = validate_path(&unsafe_path, &workspace);
        assert!(result.is_err(), "Null byte injection should be rejected");

        match result {
            Err(SecurityError::InvalidPathCharacters) => {}
            _ => return Err(anyhow::anyhow!("Expected InvalidPathCharacters error")),
        }
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
        assert!(result.is_err(), "Newline should be rejected");

        match result {
            Err(SecurityError::InvalidExpression) => {}
            _ => return Err(anyhow::anyhow!("Expected InvalidExpression error")),
        }
        Ok(())
    }

    #[test]
    fn test_validate_expression_carriage_return() {
        let result = validate_expression("1\rprint 'hacked'");
        assert!(result.is_err(), "Carriage return should be rejected");
    }

    #[test]
    fn test_validate_timeout_within_bounds() {
        assert_eq!(validate_timeout(1000), 1000);
        assert_eq!(validate_timeout(5000), 5000);
        assert_eq!(validate_timeout(100_000), 100_000);
    }

    #[test]
    fn test_validate_timeout_zero() {
        assert_eq!(validate_timeout(0), 1, "Zero timeout should be capped to 1ms");
    }

    #[test]
    fn test_validate_timeout_excessive() {
        assert_eq!(validate_timeout(500_000), MAX_TIMEOUT_MS, "Excessive timeout should be capped");
        assert_eq!(validate_timeout(1_000_000), MAX_TIMEOUT_MS);
    }

    #[test]
    fn test_validate_condition_valid() -> Result<()> {
        validate_condition("$x > 10")?;
        validate_condition("defined($var)")?;
        validate_condition("$count == 5")?;
        Ok(())
    }

    #[test]
    fn test_validate_condition_protocol_injection() {
        let result = validate_condition("1; print \"PWNED\"\n");
        assert!(result.is_err(), "Protocol injection attempt should be rejected");
    }

    #[test]
    fn test_validate_path_current_directory() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let path = PathBuf::from("./src/main.pl");
        let result = validate_path(&path, workspace)?;

        assert!(result.to_string_lossy().contains("src"));
        assert!(result.to_string_lossy().contains("main.pl"));
        Ok(())
    }

    #[test]
    fn test_validate_path_dot_files() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let workspace = tempdir.path();

        let path = PathBuf::from(".gitignore");
        let result = validate_path(&path, workspace);

        assert!(result.is_ok(), "Dot files within workspace should be allowed");
        Ok(())
    }

    #[test]
    fn test_validate_path_mixed_separators() -> Result<()> {
        use perl_tdd_support::must;
        let workspace = must(std::env::current_dir());
        let path = PathBuf::from("..\\../etc/passwd");

        let result = validate_path(&path, &workspace);
        if cfg!(windows) {
            assert!(
                result.is_err(),
                "Mixed separators should be rejected on Windows: {:?}",
                result
            );
        } else {
            assert!(
                result.is_ok(),
                "On Unix, backslash is a literal char, not a separator: {:?}",
                result
            );
        }
        Ok(())
    }
}
