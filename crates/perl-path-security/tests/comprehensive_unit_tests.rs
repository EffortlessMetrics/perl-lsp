//! Comprehensive security-focused unit tests for `perl-path-security`.
//!
//! These tests exercise adversarial inputs that a malicious LSP/DAP client
//! might send to escape the workspace sandbox.

use perl_path_security::{WorkspacePathError, validate_workspace_path};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn workspace() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let canonical = tmp.path().canonicalize()?;
    Ok((tmp, canonical))
}

// ---------------------------------------------------------------------------
// Path traversal – classic patterns
// ---------------------------------------------------------------------------

#[test]
fn traversal_single_parent() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from(".."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_double_parent() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../.."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_deep_escape_etc_passwd() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../../../etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_deep_escape_etc_shadow() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../../../../etc/shadow"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_descend_then_escape() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Go into a subdir then back out past the root
    let result = validate_workspace_path(&PathBuf::from("sub/../../.."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_many_parents() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(30) + "etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(&evil), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_interleaved_down_up() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // a/../b/../../secret
    let result = validate_workspace_path(&PathBuf::from("a/../b/../../secret"), &ws);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Path traversal – absolute paths pointing outside workspace
// ---------------------------------------------------------------------------

#[test]
fn absolute_path_outside_workspace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_path_root() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_path_tmp() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/tmp"), &ws);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Null byte injection
// ---------------------------------------------------------------------------

#[test]
fn null_byte_at_start() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("\0evil.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_in_middle() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("safe.pl\0../../etc/passwd"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_at_end() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file.pl\0"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_in_directory_component() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib/\0/Module.pm"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

// ---------------------------------------------------------------------------
// Control character injection
// ---------------------------------------------------------------------------

#[test]
fn control_char_newline() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\n.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_carriage_return() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\r.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_bell() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x07.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_escape() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x1b.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn tab_is_allowed() -> TestResult {
    // The implementation explicitly allows tab (\t)
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\t.pl"), &ws);
    // Tab should NOT trigger InvalidPathCharacters
    assert!(!matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

// ---------------------------------------------------------------------------
// Unicode normalization / confusable attacks
// ---------------------------------------------------------------------------

#[test]
fn unicode_dot_dot_fullwidth() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Fullwidth period U+FF0E — should not be treated as ".." traversal
    // but also should not bypass checks if OS normalizes it
    let evil = "\u{FF0E}\u{FF0E}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    // On Linux this is a literal filename, so it stays in workspace (OK).
    // The important thing is it does NOT resolve to /etc/passwd.
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_dot_dot_halfwidth() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Halfwidth forms period U+FF61
    let evil = "\u{FF61}\u{FF61}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_slash_lookalike() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Fraction slash U+2044 — should not be treated as path separator
    let evil = "..\u{2044}..\u{2044}etc\u{2044}passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    // Should stay in workspace (treated as a literal filename component)
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_right_to_left_override() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // RLO U+202E can be used to disguise filenames
    let evil = "\u{202E}fdssap/cte/../../../";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    // Must either reject or stay in workspace
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_combining_characters_in_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Combining dot above U+0307 on top of "." — not a real ".."
    let evil = ".\u{0307}.\u{0307}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Symlink-based escape
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn symlink_escape_detected() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    // Create a symlink inside workspace pointing outside
    let link_path = ws.join("escape_link");
    std::os::unix::fs::symlink("/etc", &link_path)?;

    let result = validate_workspace_path(&PathBuf::from("escape_link/passwd"), ws);
    // The canonicalized path should resolve to /etc/passwd which is outside
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_parent_detected() -> TestResult {
    // Use two sibling tempdirs so the symlink target actually exists and
    // canonicalize() can resolve it, exposing the escape.
    let outer = tempfile::tempdir()?;
    let ws_dir = outer.path().join("workspace");
    let secret_dir = outer.path().join("secret");
    std::fs::create_dir(&ws_dir)?;
    std::fs::create_dir(&secret_dir)?;
    std::fs::write(secret_dir.join("data.txt"), "secret")?;

    // Symlink inside workspace pointing to the sibling secret dir
    let link_path = ws_dir.join("escape_link");
    std::os::unix::fs::symlink(&secret_dir, &link_path)?;

    let result = validate_workspace_path(&PathBuf::from("escape_link/data.txt"), &ws_dir);
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_within_workspace_is_allowed() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    // Create a real target directory inside workspace
    let target = ws.join("real_dir");
    std::fs::create_dir(&target)?;
    std::fs::write(target.join("file.pl"), "# ok")?;

    // Symlink within workspace pointing to another workspace location
    let link_path = ws.join("internal_link");
    std::os::unix::fs::symlink(&target, &link_path)?;

    let result = validate_workspace_path(&PathBuf::from("internal_link/file.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ---------------------------------------------------------------------------
// Dot-segment normalization edge cases
// ---------------------------------------------------------------------------

#[test]
fn current_dir_repeated() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("./././././file.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    assert!(result.to_string_lossy().contains("file.pl"));
    Ok(())
}

#[test]
fn mixed_current_and_parent_staying_inside() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Go into sub, back, into another — should stay in workspace
    let result = validate_workspace_path(&PathBuf::from("a/./b/../c/./d.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn parent_at_boundary_exact() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Go into one dir, then back — exactly at workspace root
    let result = validate_workspace_path(&PathBuf::from("sub/.."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn parent_one_past_boundary() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Go into one dir, then back two — escapes by one level
    let result = validate_workspace_path(&PathBuf::from("sub/../.."), &ws);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Valid paths — positive cases
// ---------------------------------------------------------------------------

#[test]
fn valid_simple_filename() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("script.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    assert!(result.to_string_lossy().ends_with("script.pl"));
    Ok(())
}

#[test]
fn valid_nested_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib/My/Module.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_dotfile() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from(".perltidyrc"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_hidden_directory() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from(".config/perlcritic"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_deeply_nested() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("a/b/c/d/e/f/g/h/i/j/script.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_absolute_inside_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    let target = ws.join("lib");
    std::fs::create_dir(&target)?;
    std::fs::write(target.join("Mod.pm"), "1;")?;

    let abs_path = target.join("Mod.pm");
    let result = validate_workspace_path(&abs_path, ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows-style backslash separators (Linux treats as literal)
// ---------------------------------------------------------------------------

#[test]
fn backslash_traversal_on_unix() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = PathBuf::from("..\\..\\etc\\passwd");
    let result = validate_workspace_path(&path, &ws);

    if cfg!(windows) {
        // On Windows the backslashes are path separators → traversal
        assert!(result.is_err());
    } else {
        // On Unix the whole thing is a literal filename component
        if let Ok(ref resolved) = result {
            assert!(resolved.starts_with(&ws));
        }
    }
    Ok(())
}

#[test]
fn backslash_mixed_separators() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = PathBuf::from("sub\\..\\..\\..\\etc\\passwd");
    let result = validate_workspace_path(&path, &ws);

    if cfg!(windows) {
        assert!(result.is_err());
    } else {
        // Literal filename on Unix — stays in workspace
        if let Ok(ref resolved) = result {
            assert!(resolved.starts_with(&ws));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty and degenerate paths
// ---------------------------------------------------------------------------

#[test]
fn empty_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Empty path joined with workspace = workspace root itself
    let result = validate_workspace_path(&PathBuf::from(""), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn just_dot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn double_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib//Module.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn trailing_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib/"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ---------------------------------------------------------------------------
// Error variant coverage
// ---------------------------------------------------------------------------

#[test]
fn error_display_traversal_attempt() -> TestResult {
    let err = WorkspacePathError::PathTraversalAttempt("test".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("traversal"));
    Ok(())
}

#[test]
fn error_display_outside_workspace() -> TestResult {
    let err = WorkspacePathError::PathOutsideWorkspace("test".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("outside"));
    Ok(())
}

#[test]
fn error_display_invalid_chars() -> TestResult {
    let err = WorkspacePathError::InvalidPathCharacters;
    let msg = format!("{err}");
    assert!(msg.contains("Invalid"));
    Ok(())
}

#[test]
fn error_clone_and_eq() -> TestResult {
    let e1 = WorkspacePathError::InvalidPathCharacters;
    let e2 = e1.clone();
    assert_eq!(e1, e2);
    Ok(())
}

#[test]
fn error_debug_format() -> TestResult {
    let err = WorkspacePathError::PathTraversalAttempt("payload".to_string());
    let debug = format!("{err:?}");
    assert!(debug.contains("PathTraversalAttempt"));
    assert!(debug.contains("payload"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Workspace root edge cases
// ---------------------------------------------------------------------------

#[test]
fn nonexistent_workspace_root() -> TestResult {
    let result =
        validate_workspace_path(&PathBuf::from("file.pl"), &PathBuf::from("/no/such/dir/xyz"));
    assert!(matches!(result, Err(WorkspacePathError::PathOutsideWorkspace(_))));
    Ok(())
}

// ---------------------------------------------------------------------------
// Adversarial long paths
// ---------------------------------------------------------------------------

#[test]
fn extremely_long_path_stays_in_workspace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let long_component = "a".repeat(200);
    let long_path = format!("{long_component}/{long_component}/{long_component}/file.pl");
    let result = validate_workspace_path(&PathBuf::from(&long_path), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn extremely_long_traversal_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(500) + "etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(&evil), &ws);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Path with existing filesystem objects
// ---------------------------------------------------------------------------

#[test]
fn existing_file_inside_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::write(ws.join("hello.pl"), "print 'hi';")?;

    let result = validate_workspace_path(&PathBuf::from("hello.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    assert!(result.to_string_lossy().ends_with("hello.pl"));
    Ok(())
}

#[test]
fn existing_nested_dir_inside_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("lib/My"))?;
    std::fs::write(ws.join("lib/My/Module.pm"), "package My::Module; 1;")?;

    let result = validate_workspace_path(&PathBuf::from("lib/My/Module.pm"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

#[test]
fn traversal_with_existing_intermediate_dirs() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("a/b"))?;

    // a/b/../../.. — goes up past workspace
    let result = validate_workspace_path(&PathBuf::from("a/b/../../.."), ws);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Encoded / percent-encoded paths (should be treated literally)
// ---------------------------------------------------------------------------

#[test]
fn percent_encoded_dot_dot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %2e%2e is NOT the same as ".." at the OS level — treated as literal filename
    let result = validate_workspace_path(&PathBuf::from("%2e%2e/%2e%2e/etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn percent_encoded_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %2f is NOT treated as "/" by the OS path parser
    let result = validate_workspace_path(&PathBuf::from("..%2f..%2fetc%2fpasswd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ---------------------------------------------------------------------------
// Special filenames
// ---------------------------------------------------------------------------

#[test]
fn filename_with_spaces() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("my script.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn filename_with_unicode() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("日本語スクリプト.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn filename_double_extension() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("script.pl.bak"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}
