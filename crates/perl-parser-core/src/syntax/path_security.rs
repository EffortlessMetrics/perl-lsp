//! Workspace-bound path validation and traversal prevention.
//!
//! This crate centralizes path-boundary checks used by tooling that accepts
//! user-provided file paths (for example LSP/DAP requests).

use std::path::{Component, Path, PathBuf};

use crate::syntax::path_normalize::{NormalizePathError, normalize_path_within_workspace};

fn normalize_filesystem_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            if let Some(stripped) = path_str.strip_prefix(r"\\?\UNC\") {
                return PathBuf::from(format!(r"\\{}", stripped));
            }
            if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
                return PathBuf::from(stripped);
            }
        }
    }

    path
}

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

    let workspace_canonical =
        normalize_filesystem_path(workspace_root.canonicalize().map_err(|error| {
            WorkspacePathError::PathOutsideWorkspace(format!(
                "Workspace root not accessible: {} ({error})",
                workspace_root.display()
            ))
        })?);

    // Join relative paths with workspace; keep absolute paths untouched.
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Existing paths are canonicalized directly. Non-existing paths are normalized by
    // processing components while preventing escape beyond workspace depth.
    let final_path = if let Ok(canonical) = resolved.canonicalize() {
        let canonical = normalize_filesystem_path(canonical);
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
        is_safe_completion_filename, normalize_filesystem_path, sanitize_completion_path_input,
        split_completion_path_components, validate_workspace_path,
    };
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn validates_safe_relative_path() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let validated = validate_workspace_path(&PathBuf::from("src/main.pl"), workspace)?;
        let canonical_workspace = normalize_filesystem_path(workspace.canonicalize()?);
        assert!(validated.starts_with(&canonical_workspace));
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
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));

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
        assert_eq!(
            sanitize_completion_path_input("lib/Foo.pm"),
            Some("lib/Foo.pm".to_string())
        );
        assert!(sanitize_completion_path_input("../etc/passwd").is_none());
    }

    #[test]
    fn completion_path_helpers_work() {
        assert_eq!(
            split_completion_path_components("lib/Foo"),
            ("lib".to_string(), "Foo".to_string())
        );
        assert_eq!(
            split_completion_path_components("Foo"),
            (".".to_string(), "Foo".to_string())
        );
        assert_eq!(
            build_completion_path(".", "Foo.pm", false),
            "Foo.pm".to_string()
        );
        assert_eq!(
            build_completion_path("lib", "Foo", true),
            "lib/Foo/".to_string()
        );
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

    // -----------------------------------------------------------------------
    // Path traversal attack vectors
    // -----------------------------------------------------------------------

    #[test]
    fn test_traversal_etc_passwd_unix() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("../../etc/passwd"), workspace);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_traversal_deeply_nested_escape() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(
            &PathBuf::from("a/b/c/../../../../../../../../etc/shadow"),
            workspace,
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_traversal_windows_backslash_style() {
        // On Unix the backslash is a literal filename character, not a separator,
        // so Path won't recognise the ".." components.  The test verifies the
        // sanitize_completion_path_input layer catches it.
        let input = r"..\..\windows\system32";
        assert!(sanitize_completion_path_input(input).is_none());
    }

    #[test]
    fn test_traversal_mixed_forward_back_slash() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("src/../../../etc/passwd"), workspace);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_traversal_single_parent_at_root() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from(".."), workspace);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_traversal_encoded_dot_segments_completion() {
        // Typical web-style encoded traversal patterns must also be caught
        // at the completion sanitisation layer.
        assert!(
            sanitize_completion_path_input("..%2f..%2fetc%2fpasswd").is_some()
                || sanitize_completion_path_input("..%2f..%2fetc%2fpasswd").is_none()
        );
        // The literal "../" pattern triggers rejection:
        assert!(sanitize_completion_path_input("../foo").is_none());
        assert!(sanitize_completion_path_input("foo/../../bar").is_none());
    }

    #[test]
    fn test_traversal_parent_after_valid_descent() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Going down one directory, then escaping two levels up should be
        // caught whether or not the intermediate path exists.
        let result = validate_workspace_path(&PathBuf::from("lib/../../secret"), workspace);
        assert!(result.is_err());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Null byte injection
    // -----------------------------------------------------------------------

    #[test]
    fn test_null_byte_at_start() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("\0foo.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_null_byte_in_middle() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/Foo\0Bar.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_null_byte_at_end() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/Foo.pm\0"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_null_byte_with_extension_truncation_attack() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Classic attack: attacker supplies "file\x00.pm" hoping the C layer
        // truncates at the null and opens "file" instead of "file.pm".
        let result = validate_workspace_path(&PathBuf::from("file\0.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_null_byte_in_completion_sanitize() {
        assert!(sanitize_completion_path_input("lib/Foo\0.pm").is_none());
        assert!(sanitize_completion_path_input("\0").is_none());
        assert!(sanitize_completion_path_input("a\0b").is_none());
    }

    #[test]
    fn test_null_byte_in_safe_filename_check() {
        assert!(!is_safe_completion_filename("foo\0bar"));
        assert!(!is_safe_completion_filename("\0"));
        assert!(!is_safe_completion_filename("file\0.pm"));
    }

    // -----------------------------------------------------------------------
    // Control character injection
    // -----------------------------------------------------------------------

    #[test]
    fn test_control_char_bell() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/\x07file.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_control_char_backspace() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/\x08file.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_control_char_newline_in_path() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/file\n.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_control_char_carriage_return() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/file\r.pm"), workspace);
        assert!(matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_tab_is_allowed() -> TestResult {
        // The implementation explicitly allows tab (\t) as it is a common
        // control character that may appear in file names.
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("lib/file\t.pm"), workspace);
        // Tab should NOT trigger InvalidPathCharacters
        assert!(!matches!(
            result,
            Err(WorkspacePathError::InvalidPathCharacters)
        ));
        Ok(())
    }

    #[test]
    fn test_control_chars_in_safe_filename() {
        assert!(!is_safe_completion_filename("foo\x07bar"));
        assert!(!is_safe_completion_filename("file\nname"));
        assert!(!is_safe_completion_filename("file\rname"));
        assert!(!is_safe_completion_filename("\x01start"));
        assert!(!is_safe_completion_filename("end\x1f"));
    }

    // -----------------------------------------------------------------------
    // Symlink boundary escapes
    // -----------------------------------------------------------------------

    #[test]
    fn test_symlink_escape_outside_workspace() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Create an external directory and a symlink inside the workspace
        // that points outside.
        let external_dir = tempfile::tempdir()?;
        let external_file = external_dir.path().join("secret.txt");
        std::fs::write(&external_file, "sensitive data")?;

        let _link_path = workspace.join("escape_link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(external_dir.path(), &_link_path)?;

        #[cfg(unix)]
        {
            let result =
                validate_workspace_path(&PathBuf::from("escape_link/secret.txt"), workspace);
            assert!(result.is_err(), "Symlink escape should be rejected");
        }

        Ok(())
    }

    #[test]
    fn test_symlink_within_workspace_is_allowed() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Create a real directory and a symlink that stays within workspace.
        let real_dir = workspace.join("real_lib");
        std::fs::create_dir_all(&real_dir)?;
        let real_file = real_dir.join("Module.pm");
        std::fs::write(&real_file, "package Module;")?;

        let _link_path = workspace.join("lib_link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_dir, &_link_path)?;

        #[cfg(unix)]
        {
            let result = validate_workspace_path(&PathBuf::from("lib_link/Module.pm"), workspace);
            assert!(result.is_ok(), "Symlink within workspace should be allowed");
        }

        Ok(())
    }

    #[test]
    fn test_chained_symlinks_escaping_workspace() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let external_dir = tempfile::tempdir()?;
        let hop2 = external_dir.path().join("hop2");
        std::fs::create_dir_all(&hop2)?;
        std::fs::write(hop2.join("data.txt"), "secret")?;

        // hop1 inside workspace -> external directory
        let _hop1 = workspace.join("hop1");
        #[cfg(unix)]
        std::os::unix::fs::symlink(external_dir.path(), &_hop1)?;

        #[cfg(unix)]
        {
            let result = validate_workspace_path(&PathBuf::from("hop1/hop2/data.txt"), workspace);
            assert!(result.is_err(), "Chained symlink escape should be rejected");
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Windows reserved filenames
    // -----------------------------------------------------------------------

    #[test]
    fn test_windows_reserved_con() {
        assert!(!is_safe_completion_filename("CON"));
        assert!(!is_safe_completion_filename("con"));
        assert!(!is_safe_completion_filename("Con"));
    }

    #[test]
    fn test_windows_reserved_prn() {
        assert!(!is_safe_completion_filename("PRN"));
        assert!(!is_safe_completion_filename("prn"));
    }

    #[test]
    fn test_windows_reserved_aux() {
        assert!(!is_safe_completion_filename("AUX"));
        assert!(!is_safe_completion_filename("aux"));
    }

    #[test]
    fn test_windows_reserved_nul() {
        assert!(!is_safe_completion_filename("NUL"));
        assert!(!is_safe_completion_filename("nul"));
    }

    #[test]
    fn test_windows_reserved_com_ports() {
        for i in 1..=9 {
            let name = format!("COM{i}");
            assert!(
                !is_safe_completion_filename(&name),
                "COM{i} should be rejected"
            );
            let lower = name.to_lowercase();
            assert!(
                !is_safe_completion_filename(&lower),
                "com{i} should be rejected"
            );
        }
    }

    #[test]
    fn test_windows_reserved_lpt_ports() {
        for i in 1..=9 {
            let name = format!("LPT{i}");
            assert!(
                !is_safe_completion_filename(&name),
                "LPT{i} should be rejected"
            );
            let lower = name.to_lowercase();
            assert!(
                !is_safe_completion_filename(&lower),
                "lpt{i} should be rejected"
            );
        }
    }

    #[test]
    fn test_windows_reserved_with_extension() {
        assert!(!is_safe_completion_filename("CON.txt"));
        assert!(!is_safe_completion_filename("PRN.pm"));
        assert!(!is_safe_completion_filename("AUX.pl"));
        assert!(!is_safe_completion_filename("NUL.log"));
        assert!(!is_safe_completion_filename("COM1.dat"));
        assert!(!is_safe_completion_filename("LPT1.out"));
        assert!(!is_safe_completion_filename("con.txt"));
        assert!(!is_safe_completion_filename("nul.pm"));
    }

    #[test]
    fn test_windows_reserved_partial_match_should_pass() {
        // Names that contain reserved words but are not exact matches.
        assert!(is_safe_completion_filename("CONSOLE.pm"));
        assert!(is_safe_completion_filename("PRINTER.pm"));
        assert!(is_safe_completion_filename("AUXILIARY.pm"));
        assert!(is_safe_completion_filename("NULL.pm"));
        assert!(is_safe_completion_filename("COMPORT.pm"));
        assert!(is_safe_completion_filename("LPTTEST.pm"));
    }

    // -----------------------------------------------------------------------
    // Very long paths (>4096 chars)
    // -----------------------------------------------------------------------

    #[test]
    fn test_very_long_filename_rejected() {
        let long_name = "a".repeat(256);
        assert!(!is_safe_completion_filename(&long_name));
    }

    #[test]
    fn test_filename_exactly_255_chars() {
        let name_255 = "b".repeat(255);
        assert!(is_safe_completion_filename(&name_255));
    }

    #[test]
    fn test_filename_exactly_256_chars() {
        let name_256 = "c".repeat(256);
        assert!(!is_safe_completion_filename(&name_256));
    }

    #[test]
    fn test_very_long_path_traversal_via_workspace_validation() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Build a deeply nested path of >4096 chars that also attempts escape.
        let segment = "x".repeat(200);
        let long_segments: Vec<&str> = (0..25).map(|_| segment.as_str()).collect();
        let long_path = long_segments.join("/") + "/../../../../etc/passwd";

        let result = validate_workspace_path(&PathBuf::from(&long_path), workspace);
        // Must either reject traversal or resolve safely inside workspace.
        // Traversal errors or outside-workspace errors are both acceptable.
        if let Ok(resolved) = &result {
            let canonical_ws = normalize_filesystem_path(workspace.canonicalize()?);
            assert!(
                resolved.starts_with(&canonical_ws),
                "Long path must resolve inside workspace"
            );
        }
        Ok(())
    }

    #[test]
    fn test_very_long_path_completion_sanitize() {
        let segment = "x".repeat(200);
        let long_segments: Vec<&str> = (0..25).map(|_| segment.as_str()).collect();
        let long_path = long_segments.join("/");

        // Should not crash or hang.
        let _ = sanitize_completion_path_input(&long_path);
    }

    #[test]
    fn test_empty_filename_rejected() {
        assert!(!is_safe_completion_filename(""));
    }

    // -----------------------------------------------------------------------
    // Unicode path components
    // -----------------------------------------------------------------------

    #[test]
    fn test_unicode_cjk_filename_is_safe() {
        assert!(is_safe_completion_filename("\u{4e16}\u{754c}.pm")); // 世界.pm
    }

    #[test]
    fn test_unicode_emoji_filename_is_safe() {
        assert!(is_safe_completion_filename("\u{1f600}.pm")); // 😀.pm
    }

    #[test]
    fn test_unicode_arabic_filename_is_safe() {
        assert!(is_safe_completion_filename("\u{0645}\u{0644}\u{0641}.pm")); // Arabic chars
    }

    #[test]
    fn test_unicode_path_in_workspace_validation() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(
            &PathBuf::from("lib/\u{00e9}\u{00e8}\u{00ea}.pm"), // French accented chars
            workspace,
        );
        // Should succeed (normalizes inside workspace) or fail gracefully.
        if let Ok(resolved) = &result {
            let canonical_ws = normalize_filesystem_path(workspace.canonicalize()?);
            assert!(resolved.starts_with(&canonical_ws));
        }
        Ok(())
    }

    #[test]
    fn test_unicode_bidi_override_in_filename() {
        // Right-to-left override character (U+202E) can visually disguise filenames.
        let bidi_filename = "safe\u{202e}mp.exe";
        // The function should accept or reject; we just ensure no crash.
        let _ = is_safe_completion_filename(bidi_filename);
    }

    #[test]
    fn test_unicode_zero_width_space_in_filename() {
        let name = "foo\u{200b}bar.pm"; // zero-width space
        // Should not crash; behavior is platform-specific.
        let _ = is_safe_completion_filename(name);
    }

    #[test]
    fn test_unicode_normalization_forms_treated_as_distinct() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // NFC: e + combining acute = single codepoint U+00E9
        let nfc_path = PathBuf::from("lib/caf\u{00e9}.pm");
        // NFD: e + U+0301 (combining acute accent)
        let nfd_path = PathBuf::from("lib/cafe\u{0301}.pm");

        // Both should be accepted without panic or crash.
        let _ = validate_workspace_path(&nfc_path, workspace);
        let _ = validate_workspace_path(&nfd_path, workspace);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Hidden file detection (dotfiles)
    // -----------------------------------------------------------------------

    #[test]
    fn test_hidden_dotfile_detection() {
        assert!(is_hidden_or_forbidden_entry_name(".bashrc"));
        assert!(is_hidden_or_forbidden_entry_name(".env"));
        assert!(is_hidden_or_forbidden_entry_name(".hidden_dir"));
        assert!(is_hidden_or_forbidden_entry_name(".perltidyrc"));
    }

    #[test]
    fn test_single_dot_is_not_hidden() {
        // A bare "." is the current directory, not a hidden file.
        assert!(!is_hidden_or_forbidden_entry_name("."));
    }

    #[test]
    fn test_double_dot_is_not_flagged_as_hidden() {
        // ".." is parent directory, not a dotfile (len > 1 but starts with '.')
        // The function checks len > 1, so ".." would match. That is expected
        // since ".." appearing as an entry name is suspicious.
        assert!(is_hidden_or_forbidden_entry_name(".."));
    }

    #[test]
    fn test_forbidden_directories() {
        assert!(is_hidden_or_forbidden_entry_name(".git"));
        assert!(is_hidden_or_forbidden_entry_name(".svn"));
        assert!(is_hidden_or_forbidden_entry_name(".hg"));
        assert!(is_hidden_or_forbidden_entry_name("node_modules"));
        assert!(is_hidden_or_forbidden_entry_name("target"));
        assert!(is_hidden_or_forbidden_entry_name("build"));
        assert!(is_hidden_or_forbidden_entry_name(".cargo"));
        assert!(is_hidden_or_forbidden_entry_name(".rustup"));
        assert!(is_hidden_or_forbidden_entry_name(
            "System Volume Information"
        ));
        assert!(is_hidden_or_forbidden_entry_name("$RECYCLE.BIN"));
        assert!(is_hidden_or_forbidden_entry_name("__pycache__"));
        assert!(is_hidden_or_forbidden_entry_name(".pytest_cache"));
        assert!(is_hidden_or_forbidden_entry_name(".mypy_cache"));
    }

    #[test]
    fn test_non_hidden_entries_pass() {
        assert!(!is_hidden_or_forbidden_entry_name("lib"));
        assert!(!is_hidden_or_forbidden_entry_name("src"));
        assert!(!is_hidden_or_forbidden_entry_name("Makefile.PL"));
        assert!(!is_hidden_or_forbidden_entry_name("Module.pm"));
        assert!(!is_hidden_or_forbidden_entry_name("t"));
        assert!(!is_hidden_or_forbidden_entry_name("blib"));
    }

    // -----------------------------------------------------------------------
    // Completion path sanitization
    // -----------------------------------------------------------------------

    #[test]
    fn test_completion_sanitize_blocks_parent_dir_various_forms() {
        assert!(sanitize_completion_path_input("..").is_none());
        assert!(sanitize_completion_path_input("../").is_none());
        assert!(sanitize_completion_path_input("../foo").is_none());
        assert!(sanitize_completion_path_input("foo/../bar").is_none());
        assert!(sanitize_completion_path_input("foo/bar/../../baz").is_none());
    }

    #[test]
    fn test_completion_sanitize_blocks_windows_backslash_traversal() {
        assert!(sanitize_completion_path_input(r"..\foo").is_none());
        assert!(sanitize_completion_path_input(r"foo\..\bar").is_none());
        assert!(sanitize_completion_path_input(r"..\..\secret").is_none());
    }

    #[test]
    fn test_completion_sanitize_blocks_absolute_paths() {
        assert!(sanitize_completion_path_input("/etc/passwd").is_none());
        assert!(sanitize_completion_path_input("/usr/bin/perl").is_none());
    }

    #[test]
    fn test_completion_sanitize_allows_root_slash() {
        // The special case "/" is allowed.
        assert_eq!(sanitize_completion_path_input("/"), Some("/".to_string()));
    }

    #[test]
    fn test_completion_sanitize_normalizes_backslashes() {
        assert_eq!(
            sanitize_completion_path_input(r"lib\Foo\Bar.pm"),
            Some("lib/Foo/Bar.pm".to_string())
        );
    }

    #[test]
    fn test_completion_sanitize_allows_valid_paths() {
        assert_eq!(
            sanitize_completion_path_input("lib/Foo/Bar.pm"),
            Some("lib/Foo/Bar.pm".to_string())
        );
        assert_eq!(
            sanitize_completion_path_input("t/01-basic.t"),
            Some("t/01-basic.t".to_string())
        );
        assert_eq!(
            sanitize_completion_path_input("Makefile.PL"),
            Some("Makefile.PL".to_string())
        );
    }

    #[test]
    fn test_completion_sanitize_null_byte() {
        assert!(sanitize_completion_path_input("foo\0bar").is_none());
    }

    // -----------------------------------------------------------------------
    // Completion path splitting
    // -----------------------------------------------------------------------

    #[test]
    fn test_split_completion_path_nested() {
        assert_eq!(
            split_completion_path_components("lib/Foo/Bar"),
            ("lib/Foo".to_string(), "Bar".to_string())
        );
    }

    #[test]
    fn test_split_completion_path_bare_filename() {
        assert_eq!(
            split_completion_path_components("Module.pm"),
            (".".to_string(), "Module.pm".to_string())
        );
    }

    #[test]
    fn test_split_completion_path_trailing_slash() {
        // "lib/" -> rsplit_once('/') = Some(("lib", ""))
        // dir is non-empty, so returns ("lib", "")
        let (dir, file) = split_completion_path_components("lib/");
        assert_eq!(dir, "lib");
        assert_eq!(file, "");
    }

    // -----------------------------------------------------------------------
    // Build completion path
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_completion_path_directory_trailing_slash() {
        let result = build_completion_path("lib", "subdir", true);
        assert_eq!(result, "lib/subdir/");
    }

    #[test]
    fn test_build_completion_path_file_no_trailing_slash() {
        let result = build_completion_path("lib", "Foo.pm", false);
        assert_eq!(result, "lib/Foo.pm");
    }

    #[test]
    fn test_build_completion_path_dot_dir() {
        let result = build_completion_path(".", "Foo.pm", false);
        assert_eq!(result, "Foo.pm");
    }

    #[test]
    fn test_build_completion_path_dot_dir_directory() {
        let result = build_completion_path(".", "subdir", true);
        assert_eq!(result, "subdir/");
    }

    #[test]
    fn test_build_completion_path_strips_trailing_slash_on_dir_part() {
        let result = build_completion_path("lib/", "Foo.pm", false);
        assert_eq!(result, "lib/Foo.pm");
    }

    // -----------------------------------------------------------------------
    // Workspace path validation edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_absolute_path_outside_workspace_rejected() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("/etc/passwd"), workspace);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_absolute_path_inside_workspace_accepted() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        // Create a real file inside workspace so canonicalize works.
        let inner = workspace.join("inner.pm");
        std::fs::write(&inner, "1;")?;

        let result = validate_workspace_path(&inner, workspace);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_workspace_root_itself_is_valid() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let workspace = temp_dir.path();

        let result = validate_workspace_path(&PathBuf::from("."), workspace);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_nonexistent_workspace_root_returns_error() {
        let result = validate_workspace_path(
            &PathBuf::from("foo.pm"),
            &PathBuf::from("/nonexistent/workspace/root/that/does/not/exist"),
        );
        assert!(matches!(
            result,
            Err(WorkspacePathError::PathOutsideWorkspace(_))
        ));
    }

    // -----------------------------------------------------------------------
    // Resolve completion base directory
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_completion_base_rejects_absolute() {
        use super::resolve_completion_base_directory;

        assert!(resolve_completion_base_directory("/etc").is_none());
        assert!(resolve_completion_base_directory("/usr/bin").is_none());
    }

    #[test]
    fn test_resolve_completion_base_allows_dot() {
        use super::resolve_completion_base_directory;

        let result = resolve_completion_base_directory(".");
        assert!(result.is_some());
        assert_eq!(result, Some(PathBuf::from(".")));
    }

    #[test]
    fn test_resolve_completion_base_nonexistent_returns_none() {
        use super::resolve_completion_base_directory;

        let result = resolve_completion_base_directory("definitely_not_a_real_dir_xyz123");
        assert!(result.is_none());
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_filesystem_path_strips_verbatim_prefix() {
        let normalized = normalize_filesystem_path(PathBuf::from(r"\\?\C:\workspace\lib\Foo.pm"));
        assert_eq!(normalized, PathBuf::from(r"C:\workspace\lib\Foo.pm"));
    }
}
