//! Extended security-focused unit tests for `perl-path-security`.
//!
//! Covers: multi-level traversal combinations, concurrent validation,
//! workspace root variations, boundary conditions, error variant
//! properties, and real-world adversarial patterns not present in the
//! existing comprehensive suite.

use perl_path_security::{WorkspacePathError, validate_workspace_path};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn workspace() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let canonical = tmp.path().canonicalize()?;
    Ok((tmp, canonical))
}

/// Create a nested workspace with a subdir that actually exists.
fn workspace_with_subdir(
    sub: &str,
) -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    std::fs::create_dir_all(tmp.path().join(sub))?;
    let canonical = tmp.path().canonicalize()?;
    Ok((tmp, canonical))
}

// ===========================================================================
// 1. Traversal – layered / interleaved patterns
// ===========================================================================

#[test]
fn traversal_triple_nested_sub_then_escape() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("a/b/c/../../../../secret"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_parent_after_deep_descent() -> TestResult {
    let (_tmp, ws) = workspace_with_subdir("a/b/c/d/e")?;
    // Descend 5, ascend 6 — one past root
    let result = validate_workspace_path(&PathBuf::from("a/b/c/d/e/../../../../../../x"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_exact_depth_returns_workspace_root() -> TestResult {
    let (_tmp, ws) = workspace_with_subdir("a/b/c")?;
    // Descend 3, ascend 3 — should resolve to workspace root
    let result = validate_workspace_path(&PathBuf::from("a/b/c/../../.."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn traversal_interleaved_dots_and_names() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // a/./b/../c/./../../ => net effect: go up 1 past workspace
    let result = validate_workspace_path(&PathBuf::from("a/./b/../c/./../../.."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_leading_dot_segments_then_escape() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("./././../secret"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_only_parent_dirs() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../../../../../../.."), &ws);
    assert!(result.is_err());
    Ok(())
}

// ===========================================================================
// 2. Absolute paths – various forbidden targets
// ===========================================================================

#[test]
fn absolute_dev_null() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/dev/null"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_proc_self() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/proc/self/environ"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_home_directory() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/home"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_usr_bin() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/usr/bin/perl"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_path_with_trailing_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/tmp/foo/.."), &ws);
    assert!(result.is_err());
    Ok(())
}

// ===========================================================================
// 3. Null-byte & control-char – additional patterns
// ===========================================================================

#[test]
fn null_byte_between_valid_components() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib\0Module.pm"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_only() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("\0"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_form_feed() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x0c.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_vertical_tab() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x0b.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_backspace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x08.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_del() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x7f.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn multiple_null_bytes() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("a\0b\0c"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_null_mixed_with_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../\0../etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

// ===========================================================================
// 4. Valid paths – positive cases (broader coverage)
// ===========================================================================

#[test]
fn valid_perl_test_file() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("t/00-basic.t"), &ws)?;
    assert!(result.starts_with(&ws));
    assert!(result.to_string_lossy().ends_with("00-basic.t"));
    Ok(())
}

#[test]
fn valid_perl_module_nested_deep() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib/My/App/Controller/Root.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_makefile_pl() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("Makefile.PL"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_cpanfile() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("cpanfile"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_path_with_hyphens_and_underscores() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("my-module_v2/src.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_path_with_numbers() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib/v5.40/compat.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_path_single_char_components() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("a/b/c"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_path_with_dots_in_dirname() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib.bak/Module.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_hidden_deeply_nested() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from(".local/share/perl5/My/Module.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 5. Existing file-system interaction
// ===========================================================================

#[test]
fn existing_file_canonicalized_stays_in_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    let file = ws.join("real.pl");
    std::fs::write(&file, "1;")?;

    let result = validate_workspace_path(&PathBuf::from("real.pl"), ws)?;
    assert_eq!(result, file.canonicalize()?);
    Ok(())
}

#[test]
fn existing_dir_with_trailing_file() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("lib/My"))?;
    std::fs::write(ws.join("lib/My/App.pm"), "package My::App; 1;")?;

    let result = validate_workspace_path(&PathBuf::from("lib/My/App.pm"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

#[test]
fn parent_into_existing_subdir_stays_in_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("a/b"))?;
    std::fs::create_dir_all(ws.join("a/c"))?;
    std::fs::write(ws.join("a/c/file.pl"), "1;")?;

    // a/b/../c/file.pl resolves to a/c/file.pl — valid
    let result = validate_workspace_path(&PathBuf::from("a/b/../c/file.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

#[test]
fn traversal_from_existing_deep_dir_rejected() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("x/y/z"))?;

    // x/y/z/../../../.. = one past workspace
    let result = validate_workspace_path(&PathBuf::from("x/y/z/../../../.."), ws);
    assert!(result.is_err());
    Ok(())
}

// ===========================================================================
// 6. Symlink – additional patterns (Unix only)
// ===========================================================================

#[cfg(unix)]
#[test]
fn symlink_chain_escape() -> TestResult {
    let outer = tempfile::tempdir()?;
    let ws_dir = outer.path().join("workspace");
    let secret = outer.path().join("secret");
    std::fs::create_dir(&ws_dir)?;
    std::fs::create_dir(&secret)?;
    std::fs::write(secret.join("key.txt"), "top-secret")?;

    // link1 -> link2 -> secret (chain of symlinks)
    let link2 = ws_dir.join("link2");
    std::os::unix::fs::symlink(&secret, &link2)?;
    let link1 = ws_dir.join("link1");
    std::os::unix::fs::symlink(&link2, &link1)?;

    let result = validate_workspace_path(&PathBuf::from("link1/key.txt"), &ws_dir);
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_self_referential_stays_in_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    let target = ws.join("real_dir");
    std::fs::create_dir(&target)?;
    std::fs::write(target.join("data.pl"), "1;")?;

    // Symlink inside workspace pointing to another dir inside workspace
    let link = ws.join("alias");
    std::os::unix::fs::symlink(&target, &link)?;

    let result = validate_workspace_path(&PathBuf::from("alias/data.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_root_rejected() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let link = ws.join("root_link");
    std::os::unix::fs::symlink(Path::new("/"), &link)?;

    let result = validate_workspace_path(&PathBuf::from("root_link/etc/passwd"), ws);
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_dev_rejected() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let link = ws.join("dev_link");
    std::os::unix::fs::symlink(Path::new("/dev"), &link)?;

    let result = validate_workspace_path(&PathBuf::from("dev_link/null"), ws);
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn relative_symlink_within_workspace_ok() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("src"))?;
    std::fs::write(ws.join("src/lib.pl"), "1;")?;

    // Relative symlink: links/lib.pl -> ../src/lib.pl
    std::fs::create_dir_all(ws.join("links"))?;
    std::os::unix::fs::symlink(Path::new("../src/lib.pl"), ws.join("links/lib.pl"))?;

    let result = validate_workspace_path(&PathBuf::from("links/lib.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ===========================================================================
// 7. Workspace root edge cases
// ===========================================================================

#[test]
fn workspace_root_is_slash_tmp() -> TestResult {
    // /tmp exists on Linux; validate a relative path inside it
    let ws = Path::new("/tmp");
    if !ws.exists() {
        return Ok(());
    }
    let result = validate_workspace_path(&PathBuf::from("test_subdir_for_lsp"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

#[test]
fn workspace_root_with_trailing_slash() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws_str = format!("{}/", tmp.path().display());
    let ws = PathBuf::from(&ws_str);
    let result = validate_workspace_path(&PathBuf::from("file.pl"), &ws)?;
    assert!(result.to_string_lossy().contains("file.pl"));
    Ok(())
}

#[test]
fn workspace_root_with_dot_component() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().join(".");
    let result = validate_workspace_path(&PathBuf::from("file.pl"), &ws)?;
    assert!(result.to_string_lossy().contains("file.pl"));
    Ok(())
}

#[test]
fn workspace_root_canonicalization_resolves_symlink() -> TestResult {
    if cfg!(not(unix)) {
        return Ok(());
    }
    let outer = tempfile::tempdir()?;
    let real_ws = outer.path().join("real_workspace");
    std::fs::create_dir(&real_ws)?;

    let ws_link = outer.path().join("ws_link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_ws, &ws_link)?;

    // Using the symlinked workspace root should still work
    let result = validate_workspace_path(&PathBuf::from("script.pl"), &ws_link)?;
    assert!(result.starts_with(real_ws.canonicalize()?));
    Ok(())
}

// ===========================================================================
// 8. Error variant property tests
// ===========================================================================

#[test]
fn error_ne_different_variants() -> TestResult {
    let e1 = WorkspacePathError::InvalidPathCharacters;
    let e2 = WorkspacePathError::PathTraversalAttempt("x".into());
    assert_ne!(e1, e2);
    Ok(())
}

#[test]
fn error_ne_same_variant_different_payload() -> TestResult {
    let e1 = WorkspacePathError::PathTraversalAttempt("a".into());
    let e2 = WorkspacePathError::PathTraversalAttempt("b".into());
    assert_ne!(e1, e2);
    Ok(())
}

#[test]
fn error_eq_same_variant_same_payload() -> TestResult {
    let e1 = WorkspacePathError::PathOutsideWorkspace("x".into());
    let e2 = WorkspacePathError::PathOutsideWorkspace("x".into());
    assert_eq!(e1, e2);
    Ok(())
}

#[test]
fn error_clone_preserves_payload() -> TestResult {
    let original = WorkspacePathError::PathTraversalAttempt("payload-data".into());
    let cloned = original.clone();
    assert_eq!(original, cloned);
    let display = format!("{cloned}");
    assert!(display.contains("payload-data"));
    Ok(())
}

#[test]
fn error_implements_std_error() -> TestResult {
    let err = WorkspacePathError::InvalidPathCharacters;
    // Verify it implements std::error::Error by using it as &dyn Error
    let err_ref: &dyn std::error::Error = &err;
    let _ = format!("{err_ref}");
    Ok(())
}

#[test]
fn error_display_outside_workspace_contains_path_info() -> TestResult {
    let err = WorkspacePathError::PathOutsideWorkspace("/evil/path".into());
    let msg = format!("{err}");
    assert!(msg.contains("/evil/path"));
    Ok(())
}

#[test]
fn error_debug_outside_workspace() -> TestResult {
    let err = WorkspacePathError::PathOutsideWorkspace("detail".into());
    let debug = format!("{err:?}");
    assert!(debug.contains("PathOutsideWorkspace"));
    assert!(debug.contains("detail"));
    Ok(())
}

#[test]
fn error_debug_invalid_chars() -> TestResult {
    let err = WorkspacePathError::InvalidPathCharacters;
    let debug = format!("{err:?}");
    assert!(debug.contains("InvalidPathCharacters"));
    Ok(())
}

// ===========================================================================
// 9. Unicode – additional adversarial patterns
// ===========================================================================

#[test]
fn unicode_zero_width_space_in_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+200B zero-width space inserted between dots
    let evil = ".\u{200B}./etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    // Should either reject or stay in workspace (literal filename)
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_zero_width_joiner_in_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "..\u{200D}/../etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_bom_prefix() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // UTF-8 BOM at start of path
    let path_with_bom = "\u{FEFF}script.pl";
    let result = validate_workspace_path(&PathBuf::from(path_with_bom), &ws);
    // BOM is not a control char that gets rejected; it's a valid Unicode char
    // If accepted, it must stay in workspace
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_overlong_dot_not_normalized() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+2024 ONE DOT LEADER (looks like "." but isn't)
    let evil = "\u{2024}\u{2024}/\u{2024}\u{2024}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_left_to_right_mark() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "..\u{200E}/../etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_emoji_in_filename() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("🐪/script.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 10. Multiple slashes and whitespace edge cases
// ===========================================================================

#[test]
fn triple_slash_prefix() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("///etc/passwd"), &ws);
    // Triple slash resolves to absolute /etc/passwd — outside workspace
    assert!(result.is_err());
    Ok(())
}

#[test]
fn path_with_only_slashes() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("///"), &ws);
    // Resolves to root "/" which is outside workspace
    assert!(result.is_err());
    Ok(())
}

#[test]
fn path_with_trailing_whitespace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file.pl   "), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_leading_whitespace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("   file.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn multiple_consecutive_slashes_in_middle() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib////My////Module.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 11. Adversarial patterns – real-world attack vectors
// ===========================================================================

#[test]
fn dot_dot_slash_dot_dot_slash_repeated() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = ".././.././.././../etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn path_with_tilde_home_expansion() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Tilde is not expanded by the OS path API — treated as literal
    let result = validate_workspace_path(&PathBuf::from("~/secret"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_dollar_env_variable() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Shell variable syntax is literal in path API
    let result = validate_workspace_path(&PathBuf::from("$HOME/secret"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_backtick_injection() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("`rm -rf /`"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_semicolon_injection() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file.pl; rm -rf /"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_pipe_injection() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file.pl | cat /etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_with_ampersand_injection() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file.pl && cat /etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 12. Boundary: path at exact workspace root
// ===========================================================================

#[test]
fn descend_one_ascend_one_equals_root() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("x/.."), &ws)?;
    assert_eq!(result, ws);
    Ok(())
}

#[test]
fn descend_two_ascend_two_equals_root() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("x/y/../.."), &ws)?;
    assert_eq!(result, ws);
    Ok(())
}

#[test]
fn dot_resolves_to_workspace_root() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("."), &ws)?;
    assert_eq!(result, ws);
    Ok(())
}

// ===========================================================================
// 13. Percent-encoded and URL-style paths (treated literally)
// ===========================================================================

#[test]
fn percent_encoded_null_is_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %00 is NOT a null byte at the OS level — just the string "%00"
    let result = validate_workspace_path(&PathBuf::from("file%00.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn percent_encoded_parent_components_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("%2e%2e%2f%2e%2e%2f"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn url_file_scheme_is_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // "file:///etc/passwd" is treated as a literal relative path component
    let result = validate_workspace_path(&PathBuf::from("file:///etc/passwd"), &ws);
    // On Linux, "file:" is a directory name, "" is empty, "etc" etc.
    // The leading "file:" makes it relative, so this stays in workspace
    // unless the "/" after ":" creates absolute path behavior.
    // Actually "file:///etc/passwd" starts with "/" in the middle... let's just
    // verify it doesn't silently return /etc/passwd
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 14. Extremely long paths
// ===========================================================================

#[test]
fn very_long_single_component() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let long_name = "x".repeat(4096);
    let result = validate_workspace_path(&PathBuf::from(&long_name), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn many_short_components() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let many_dirs: String = (0..200).map(|i| format!("d{i}")).collect::<Vec<_>>().join("/");
    let path = format!("{many_dirs}/file.pl");
    let result = validate_workspace_path(&PathBuf::from(&path), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn long_traversal_followed_by_valid() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let downs: String = (0..100).map(|i| format!("d{i}")).collect::<Vec<_>>().join("/");
    let ups = "../".repeat(100);
    let path = format!("{downs}/{ups}safe.pl");
    let result = validate_workspace_path(&PathBuf::from(&path), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 15. Concurrent validation (thread safety)
// ===========================================================================

#[test]
fn concurrent_validations_are_safe() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().to_path_buf();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let ws_clone = ws.clone();
            std::thread::spawn(move || {
                let path = PathBuf::from(format!("file_{i}.pl"));
                validate_workspace_path(&path, &ws_clone)
            })
        })
        .collect();

    for handle in handles {
        if let Ok(Ok(resolved)) = handle.join() {
            if let Ok(canonical) = ws.canonicalize() {
                assert!(resolved.starts_with(&canonical));
            }
        }
    }
    Ok(())
}

#[test]
fn concurrent_adversarial_and_valid_mixed() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().to_path_buf();

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let ws_clone = ws.clone();
            std::thread::spawn(move || {
                let path = if i % 2 == 0 {
                    PathBuf::from(format!("safe_{i}.pl"))
                } else {
                    PathBuf::from("../../../etc/passwd")
                };
                (i, validate_workspace_path(&path, &ws_clone))
            })
        })
        .collect();

    for handle in handles {
        if let Ok((i, result)) = handle.join() {
            if i % 2 == 0 {
                // Even indices are valid paths — should succeed
                assert!(result.is_ok(), "Expected OK for safe path at index {i}");
            } else {
                // Odd indices are traversal attempts — should fail
                assert!(result.is_err(), "Expected error for traversal at index {i}");
            }
        }
    }
    Ok(())
}

// ===========================================================================
// 16. Special filesystem names
// ===========================================================================

#[test]
fn filename_is_just_dot_dot_literally() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // ".." as a path component IS parent traversal on all platforms
    let result = validate_workspace_path(&PathBuf::from(".."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn filename_dot_dot_dot_is_valid() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // "..." is a valid filename on Linux (not a special component)
    let result = validate_workspace_path(&PathBuf::from("..."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn filename_four_dots_is_valid() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("...."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn path_component_named_dot_dot_literally() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // "sub/.." resolves to workspace root — valid
    let result = validate_workspace_path(&PathBuf::from("sub/.."), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn filename_with_only_dots_and_extension() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("..../file.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}
