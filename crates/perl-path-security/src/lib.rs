//! Workspace-bound path validation and traversal prevention.
//!
//! This crate centralizes path-boundary checks used by tooling that accepts
//! user-provided file paths (for example LSP/DAP requests).

use std::path::{Component, Path, PathBuf};

use perl_path_normalize::{NormalizePathError, normalize_path_within_workspace};

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
        normalize_path_within_workspace(path, &workspace_canonical).map_err(
            |error| match error {
                NormalizePathError::PathTraversalAttempt(message) => {
                    WorkspacePathError::PathTraversalAttempt(message)
                }
            },
        )?
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

/// Sanitize and normalize user-provided completion path input.
///
/// Returns `None` when path contains traversal, absolute path (except `/`),
/// drive-prefix, null bytes, or suspicious traversal patterns.
pub fn sanitize_completion_path_input(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some(String::new());
    }

    if path.contains('\0') {
        return None;
    }

    let path_obj = Path::new(path);
    for component in path_obj.components() {
        match component {
            Component::ParentDir => return None,
            Component::RootDir if path != "/" => return None,
            Component::Prefix(_) => return None,
            _ => {}
        }
    }

    if path.contains("../") || path.contains("..\\") || path.starts_with('/') && path != "/" {
        return None;
    }

    Some(path.replace('\\', "/"))
}

/// Split sanitized completion path into `(directory_part, file_prefix)`.
pub fn split_completion_path_components(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, file)) if !dir.is_empty() => (dir.to_string(), file.to_string()),
        _ => (".".to_string(), path.to_string()),
    }
}

/// Resolve a directory used for file completion traversal.
pub fn resolve_completion_base_directory(dir_part: &str) -> Option<PathBuf> {
    let path = Path::new(dir_part);

    if path.is_absolute() && dir_part != "/" {
        return None;
    }

    if dir_part == "." {
        return Some(Path::new(".").to_path_buf());
    }

    match path.canonicalize() {
        Ok(canonical) => Some(canonical),
        Err(_) => {
            if path.exists() && path.is_dir() {
                Some(path.to_path_buf())
            } else {
                None
            }
        }
    }
}

/// Check whether a filename should be skipped during file completion traversal.
pub fn is_hidden_or_forbidden_entry_name(file_name: &str) -> bool {
    if file_name.starts_with('.') && file_name.len() > 1 {
        return true;
    }

    matches!(
        file_name,
        "node_modules"
            | ".git"
            | ".svn"
            | ".hg"
            | "target"
            | "build"
            | ".cargo"
            | ".rustup"
            | "System Volume Information"
            | "$RECYCLE.BIN"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
    )
}

/// Validate filename safety for completion entries.
pub fn is_safe_completion_filename(filename: &str) -> bool {
    if filename.is_empty() || filename.len() > 255 {
        return false;
    }

    if filename.contains('\0') || filename.chars().any(|c| c.is_control()) {
        return false;
    }

    let name_upper = filename.to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for reserved_name in &reserved {
        if name_upper == *reserved_name || name_upper.starts_with(&format!("{}.", reserved_name)) {
            return false;
        }
    }

    true
}

/// Build completion path string and append trailing slash for directories.
pub fn build_completion_path(dir_part: &str, filename: &str, is_dir: bool) -> String {
    let mut path = if dir_part == "." {
        filename.to_string()
    } else {
        format!("{}/{}", dir_part.trim_end_matches('/'), filename)
    };

    if is_dir {
        path.push('/');
    }

    path
}

#[cfg(test)]
mod tests {
    use super::{
        WorkspacePathError, build_completion_path, is_hidden_or_forbidden_entry_name,
        is_safe_completion_filename, sanitize_completion_path_input,
        split_completion_path_components, validate_workspace_path,
    };
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

    #[test]
    fn completion_path_sanitization_blocks_traversal() {
        assert_eq!(sanitize_completion_path_input(""), Some(String::new()));
        assert_eq!(sanitize_completion_path_input("lib/Foo.pm"), Some("lib/Foo.pm".to_string()));
        assert!(sanitize_completion_path_input("../etc/passwd").is_none());
    }

    #[test]
    fn completion_path_helpers_work() {
        assert_eq!(
            split_completion_path_components("lib/Foo"),
            ("lib".to_string(), "Foo".to_string())
        );
        assert_eq!(split_completion_path_components("Foo"), (".".to_string(), "Foo".to_string()));
        assert_eq!(build_completion_path(".", "Foo.pm", false), "Foo.pm".to_string());
        assert_eq!(build_completion_path("lib", "Foo", true), "lib/Foo/".to_string());
    }

    #[test]
    fn completion_filename_and_visibility_checks_work() {
        assert!(is_hidden_or_forbidden_entry_name(".git"));
        assert!(is_hidden_or_forbidden_entry_name("node_modules"));
        assert!(!is_hidden_or_forbidden_entry_name("lib"));

        assert!(is_safe_completion_filename("Foo.pm"));
        assert!(!is_safe_completion_filename("CON"));
        assert!(!is_safe_completion_filename("bad\0name"));
    }
}
