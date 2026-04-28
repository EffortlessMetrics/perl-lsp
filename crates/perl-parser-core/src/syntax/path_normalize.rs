//! Secure workspace-relative path normalization.
//!
//! This crate performs component-based normalization for paths that may not yet
//! exist on disk. It prevents parent-directory traversal beyond a canonical
//! workspace root.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::{Component, Path, PathBuf};

/// Errors produced while normalizing a relative path against a workspace root.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NormalizePathError {
    /// Path traversal or invalid component escaping workspace constraints.
    #[error("Path traversal attempt detected: {0}")]
    PathTraversalAttempt(String),
}

/// Normalize `path` beneath `workspace_root` while preventing parent traversal escapes.
///
/// This function is intended for paths that may not exist yet and therefore cannot
/// be canonicalized directly.
pub fn normalize_path_within_workspace(
    path: &Path,
    workspace_root: &Path,
) -> Result<PathBuf, NormalizePathError> {
    let mut stack: Vec<Component<'_>> = workspace_root.components().collect();
    let workspace_depth = stack.len();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                if stack.len() <= workspace_depth {
                    return Err(NormalizePathError::PathTraversalAttempt(format!(
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
                return Err(NormalizePathError::PathTraversalAttempt(format!(
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

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{NormalizePathError, normalize_path_within_workspace};
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn normalizes_safe_relative_path() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path().canonicalize()?;

        let normalized =
            normalize_path_within_workspace(&PathBuf::from("src/main.pl"), &workspace)?;
        assert!(normalized.starts_with(&workspace));
        assert!(normalized.to_string_lossy().contains("src"));
        assert!(normalized.to_string_lossy().contains("main.pl"));

        Ok(())
    }

    #[test]
    fn rejects_parent_directory_escape() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path().canonicalize()?;

        let result =
            normalize_path_within_workspace(&PathBuf::from("../../../etc/passwd"), &workspace);
        assert!(matches!(result, Err(NormalizePathError::PathTraversalAttempt(_))));

        Ok(())
    }
}
