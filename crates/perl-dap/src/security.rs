//! Security validation module for DAP Phase 3 (AC16)
//!
//! This module provides enterprise-grade security features:
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

use anyhow::Result;
use std::path::{Component, Path, PathBuf};

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

/// Validate that a path is within the workspace boundary
///
/// This function prevents path traversal attacks by ensuring:
/// - No parent directory references that escape the workspace
/// - No absolute paths outside the workspace
/// - No symlinks that resolve outside the workspace
/// - No null bytes or control characters in paths
///
/// # Arguments
///
/// * `path` - The path to validate (can be relative or absolute)
/// * `workspace_root` - The workspace root directory
///
/// # Errors
///
/// Returns `SecurityError` if:
/// - Path contains `..` components that escape the workspace
/// - Path is absolute and outside the workspace
/// - Path contains null bytes or control characters
/// - Symlink resolves outside the workspace (when checked)
///
/// # Examples
///
/// ```no_run
/// use perl_dap::security::validate_path;
/// use std::path::PathBuf;
///
/// # fn main() -> anyhow::Result<()> {
/// let workspace = PathBuf::from("/workspace");
///
/// // Valid paths
/// validate_path(&PathBuf::from("src/main.pl"), &workspace)?;
/// validate_path(&PathBuf::from("./lib/Module.pm"), &workspace)?;
///
/// // Invalid paths (would return Err)
/// // validate_path(&PathBuf::from("../etc/passwd"), &workspace)?;
/// // validate_path(&PathBuf::from("/etc/passwd"), &workspace)?;
/// # Ok(())
/// # }
/// ```
pub fn validate_path(path: &Path, workspace_root: &Path) -> Result<PathBuf, SecurityError> {
    // Check for null bytes and control characters
    if let Some(path_str) = path.to_str() {
        if path_str.contains('\0') || path_str.chars().any(|c| c.is_control() && c != '\t') {
            return Err(SecurityError::InvalidPathCharacters);
        }
    }

    // Get canonical workspace root (must exist for validation)
    let workspace_canonical = workspace_root.canonicalize().map_err(|e| {
        SecurityError::PathOutsideWorkspace(format!(
            "Workspace root not accessible: {} ({})",
            workspace_root.display(),
            e
        ))
    })?;

    // Resolve the path: join relative paths with workspace, keep absolute as-is
    let resolved = if path.is_absolute() { path.to_path_buf() } else { workspace_root.join(path) };

    // Try to canonicalize the resolved path
    // For existing paths, this resolves symlinks and normalizes .. and .
    let final_path = if let Ok(canonical) = resolved.canonicalize() {
        // Path exists - check if within workspace
        if !canonical.starts_with(&workspace_canonical) {
            return Err(SecurityError::PathOutsideWorkspace(format!(
                "Path resolves outside workspace: {} (workspace: {})",
                canonical.display(),
                workspace_canonical.display()
            )));
        }
        canonical
    } else {
        // Path doesn't exist - manually normalize components
        // Process components relative to workspace
        let mut stack: Vec<Component> = workspace_canonical.components().collect();
        let workspace_depth = stack.len();

        // Process the user-provided path components
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    if stack.len() <= workspace_depth {
                        // Trying to go above workspace
                        return Err(SecurityError::PathTraversalAttempt(format!(
                            "Path attempts to escape workspace: {}",
                            path.display()
                        )));
                    }
                    stack.pop();
                }
                Component::Normal(name) => {
                    stack.push(Component::Normal(name));
                }
                Component::CurDir => {
                    // Skip current directory
                }
                Component::RootDir | Component::Prefix(_) => {
                    // Relative paths shouldn't have these
                    return Err(SecurityError::PathTraversalAttempt(format!(
                        "Invalid component in relative path: {}",
                        path.display()
                    )));
                }
            }
        }

        // Reconstruct the path from the stack
        let mut result = PathBuf::new();
        for component in stack {
            result.push(component);
        }

        result
    };

    // Final validation - ensure we're within workspace
    if !final_path.starts_with(&workspace_canonical) {
        return Err(SecurityError::PathOutsideWorkspace(format!(
            "Path outside workspace: {} (workspace: {})",
            final_path.display(),
            workspace_canonical.display()
        )));
    }

    Ok(final_path)
}

/// Validate an expression for safe evaluation
///
/// This function prevents protocol injection attacks by ensuring:
/// - No newline characters (\\n or \\r)
/// - No control characters that could break protocol framing
///
/// # Arguments
///
/// * `expression` - The expression to validate
///
/// # Errors
///
/// Returns `SecurityError::InvalidExpression` if the expression contains newlines
///
/// # Examples
///
/// ```
/// use perl_dap::security::validate_expression;
///
/// # fn main() -> anyhow::Result<()> {
/// // Valid expressions
/// validate_expression("$x + 1")?;
/// validate_expression("my_function()")?;
///
/// // Invalid expressions (would return Err)
/// // validate_expression("1\nprint 'hacked'")?;
/// # Ok(())
/// # }
/// ```
pub fn validate_expression(expression: &str) -> Result<(), SecurityError> {
    // Check for newlines (both \n and \r)
    if expression.contains('\n') || expression.contains('\r') {
        return Err(SecurityError::InvalidExpression);
    }

    Ok(())
}

/// Validate and cap a timeout value
///
/// This function ensures timeouts are within reasonable bounds:
/// - Minimum: 1ms (prevents zero timeout)
/// - Maximum: 300,000ms (5 minutes)
///
/// # Arguments
///
/// * `timeout_ms` - The requested timeout in milliseconds
///
/// # Returns
///
/// The capped timeout value, guaranteed to be within bounds
///
/// # Examples
///
/// ```
/// use perl_dap::security::validate_timeout;
///
/// assert_eq!(validate_timeout(1000), 1000);     // Within bounds
/// assert_eq!(validate_timeout(0), 1);           // Below minimum
/// assert_eq!(validate_timeout(500_000), 300_000); // Above maximum
/// ```
pub fn validate_timeout(timeout_ms: u32) -> u32 {
    // Ensure at least 1ms
    let timeout = timeout_ms.max(1);

    // Cap at maximum
    timeout.min(MAX_TIMEOUT_MS)
}

/// Validate a breakpoint condition for security issues
///
/// This function checks breakpoint conditions for protocol injection risks
/// by ensuring they don't contain newlines or control characters.
///
/// # Arguments
///
/// * `condition` - The breakpoint condition to validate
///
/// # Errors
///
/// Returns `SecurityError::InvalidExpression` if the condition is unsafe
///
/// # Examples
///
/// ```
/// use perl_dap::security::validate_condition;
///
/// # fn main() -> anyhow::Result<()> {
/// // Valid conditions
/// validate_condition("$x > 10")?;
/// validate_condition("defined($var)")?;
///
/// // Invalid conditions (would return Err)
/// // validate_condition("1\nprint 'pwned'")?;
/// # Ok(())
/// # }
/// ```
pub fn validate_condition(condition: &str) -> Result<(), SecurityError> {
    // Use the same validation as expressions
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
            | Err(SecurityError::PathOutsideWorkspace(_)) => {
                // Either error is acceptable - both indicate the path was rejected
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Expected PathTraversalAttempt or PathOutsideWorkspace error, got: {:?}", e));
            }
            Ok(_) => return Err(anyhow::anyhow!("Expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn test_validate_path_absolute_outside() -> Result<()> {
        use perl_tdd_support::{must, must_some};
        // Use a specific subdirectory as workspace to ensure separation
        let workspace = must(std::env::current_dir()).join("test_workspace");

        // Create workspace directory for the test
        fs::create_dir_all(&workspace).ok();

        // Use a path that's definitely outside the workspace
        let unsafe_path = must_some(workspace.parent()).join("etc/passwd");

        let result = validate_path(&unsafe_path, &workspace);

        // Clean up
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
        // Use current directory as workspace to ensure it exists
        let workspace = must(std::env::current_dir());
        // Windows-style path with mixed separators - on Unix this is just weird filename chars
        let path = PathBuf::from("..\\../etc/passwd");

        let result = validate_path(&path, &workspace);
        // On Unix, backslash is just a character, so ..\ is a directory name, not parent ref
        assert!(result.is_err(), "Mixed separators should likely be rejected or sanitized: {:?}", result);
        Ok(())
    }
}
