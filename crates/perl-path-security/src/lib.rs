//! Workspace-bound path validation and traversal prevention.
//!
//! This crate centralizes path-boundary checks used by tooling that accepts
//! user-provided file paths (for example LSP/DAP requests).

use std::path::{Component, Path, PathBuf};

/// Path validation errors for workspace-bound operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspacePathError {
    /// Parent traversal or invalid component escaping workspace constraints.
    #[error("Path traversal attempt detected: {0}")]
    PathTraversalAttempt(String),

    /// Path resolves outside the workspace root.
    #[error("Path outside workspace: {0}")]
    PathOutsideWorkspace(String),

    /// Path contains null bytes or disallowed control characters.
    #[error("Invalid path characters detected")]
    InvalidPathCharacters,
}

/// Validate and normalize a path so it remains within `workspace_root`.
///
/// The returned path is absolute and suitable for downstream filesystem access.
pub fn validate_workspace_path(
    path: &Path,
    workspace_root: &Path,
) -> Result<PathBuf, WorkspacePathError> {
    // Reject null bytes and control characters to avoid protocol/filesystem confusion.
    if let Some(path_str) = path.to_str()
        && (path_str.contains('\0') || path_str.chars().any(|c| c.is_control() && c != '\t'))
    {
        return Err(WorkspacePathError::InvalidPathCharacters);
    }

    let workspace_canonical = workspace_root.canonicalize().map_err(|error| {
        WorkspacePathError::PathOutsideWorkspace(format!(
            "Workspace root not accessible: {} ({error})",
            workspace_root.display()
        ))
    })?;

    // Join relative paths with workspace; keep absolute paths untouched.
    let resolved = if path.is_absolute() { path.to_path_buf() } else { workspace_root.join(path) };

    // Existing paths are canonicalized directly. Non-existing paths are normalized by
    // processing components while preventing escape beyond workspace depth.
    let final_path = if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&workspace_canonical) {
            return Err(WorkspacePathError::PathOutsideWorkspace(format!(
                "Path resolves outside workspace: {} (workspace: {})",
                canonical.display(),
                workspace_canonical.display()
            )));
        }

        canonical
    } else {
        let mut stack: Vec<Component<'_>> = workspace_canonical.components().collect();
        let workspace_depth = stack.len();

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    if stack.len() <= workspace_depth {
                        return Err(WorkspacePathError::PathTraversalAttempt(format!(
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
                    // ignore
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspacePathError::PathTraversalAttempt(format!(
                        "Invalid component in relative path: {}",
                        path.display()
                    )));
                }
            }
        }

        let mut normalized = PathBuf::new();
        for component in stack {
            normalized.push(component.as_os_str());
        }
        normalized
    };

    if !final_path.starts_with(&workspace_canonical) {
        return Err(WorkspacePathError::PathOutsideWorkspace(format!(
            "Path outside workspace: {} (workspace: {})",
            final_path.display(),
            workspace_canonical.display()
        )));
    }

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::{WorkspacePathError, validate_workspace_path};
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn validates_safe_relative_path() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let validated = validate_workspace_path(&PathBuf::from("src/main.pl"), workspace)?;
        assert!(validated.starts_with(workspace.canonicalize()?));
        assert!(validated.to_string_lossy().contains("src"));
        assert!(validated.to_string_lossy().contains("main.pl"));

        Ok(())
    }

    #[test]
    fn rejects_parent_directory_escape() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("../../../etc/passwd"), workspace);
        assert!(result.is_err());

        match result {
            Err(WorkspacePathError::PathTraversalAttempt(_))
            | Err(WorkspacePathError::PathOutsideWorkspace(_)) => Ok(()),
            Err(error) => Err(format!("unexpected error type: {error:?}").into()),
            Ok(_) => Err("expected path validation error".into()),
        }
    }

    #[test]
    fn rejects_null_byte_injection() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result =
            validate_workspace_path(&PathBuf::from("valid.pl\0../../etc/passwd"), workspace);
        assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));

        Ok(())
    }

    #[test]
    fn allows_dot_files_inside_workspace() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from(".gitignore"), workspace);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn supports_current_directory_component() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let validated = validate_workspace_path(&PathBuf::from("./lib/Module.pm"), workspace)?;
        assert!(validated.to_string_lossy().contains("lib"));
        assert!(validated.to_string_lossy().contains("Module.pm"));

        Ok(())
    }

    #[test]
    fn mixed_separator_behavior_matches_platform_rules() -> TestResult {
        let workspace = std::env::current_dir()?;
        let path = PathBuf::from("..\\../etc/passwd");

        let result = validate_workspace_path(&path, &workspace);
        if cfg!(windows) {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }

        Ok(())
    }
}
