//! Hardened security tests for path traversal prevention in `perl-path-security`.
//!
//! Focuses on adversarial attack vectors not covered by existing test suites:
//! - Unicode normalization attacks (U+2025 two-dot leader, U+2024 one-dot leader, etc.)
//! - Null byte injection in varied positions
//! - Symlink traversal with deeply nested chains
//! - Windows-style path separators on Linux
//! - Double-encoding / multi-layer encoding bypasses
//! - `sanitize_completion_path_input` hardening
//! - `is_safe_completion_filename` edge cases

use perl_path_security::{
    WorkspacePathError, build_completion_path, is_hidden_or_forbidden_entry_name,
    is_safe_completion_filename, resolve_completion_base_directory, sanitize_completion_path_input,
    split_completion_path_components, validate_workspace_path,
};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn normalize_canonical_path(path: PathBuf) -> PathBuf {
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

fn canonicalized(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(normalize_canonical_path(path.canonicalize()?))
}

fn workspace() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let canonical = canonicalized(tmp.path())?;
    Ok((tmp, canonical))
}

// ===========================================================================
// 1. Unicode normalization attacks -- U+2025 TWO DOT LEADER
// ===========================================================================

#[test]
fn unicode_two_dot_leader_u2025_not_treated_as_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+2025 TWO DOT LEADER looks like ".." but must not be treated as parent traversal.
    // On Linux, U+2025 is a literal filename character -- the path stays within workspace
    // as a subdirectory tree like: <workspace>/\u{2025}/\u{2025}/etc/passwd
    // The security invariant: the resolved path MUST be within the workspace.
    let evil = "\u{2025}/\u{2025}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws), "Path must stay within workspace");
    }
    Ok(())
}

#[test]
fn unicode_two_dot_leader_in_subdir() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Attempt: sub/<TWO DOT LEADER>/<TWO DOT LEADER>/../../etc/passwd
    let evil = "sub/\u{2025}/\u{2025}/../../etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_two_dot_leader_mixed_with_real_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Mix real ".." with U+2025 to confuse parsers
    let evil = "sub/../\u{2025}/../secret";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    // The real ".." is the traversal vector; U+2025 is a literal dir name
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 2. Unicode normalization -- additional confusable characters
// ===========================================================================

#[test]
fn unicode_small_full_stop_u_fe52() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+FE52 SMALL FULL STOP -- visually similar to "."
    let evil = "\u{FE52}\u{FE52}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_ideographic_full_stop_u_3002() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+3002 IDEOGRAPHIC FULL STOP -- CJK period
    let evil = "\u{3002}\u{3002}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_fullwidth_solidus_u_ff0f_as_separator() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+FF0F FULLWIDTH SOLIDUS -- looks like "/" but must not be treated as path separator
    let evil = "..\u{FF0F}..\u{FF0F}etc\u{FF0F}passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_division_slash_u_2215_as_separator() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+2215 DIVISION SLASH
    let evil = "..\u{2215}..\u{2215}etc\u{2215}passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_reverse_solidus_operator_u_29f5() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+29F5 REVERSE SOLIDUS OPERATOR -- backslash lookalike
    let evil = "..\u{29F5}..\u{29F5}etc\u{29F5}passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_set_minus_u_2216_as_backslash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+2216 SET MINUS -- visually similar to backslash
    let evil = "..\u{2216}..\u{2216}etc\u{2216}passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn unicode_combining_long_solidus_overlay() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+0338 COMBINING LONG SOLIDUS OVERLAY added to "."
    let evil = ".\u{0338}.\u{0338}/etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 3. Null byte injection -- additional adversarial positions
// ===========================================================================

#[test]
fn null_byte_after_traversal_sequence() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("../../\0etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn null_byte_within_extension() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("script.pl\0.txt"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_before_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("lib\0/Module.pm"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_multiple_scattered() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("a\0/b\0/c\0"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_with_unicode_payload() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = "\u{2025}\0../../etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(path), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

// ===========================================================================
// 4. Windows-style path separators on Linux
// ===========================================================================

#[test]
fn windows_backslash_simple_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = PathBuf::from("..\\etc\\passwd");
    let result = validate_workspace_path(&path, &ws);
    // On Linux, backslash is a literal character in filenames.
    // Must never resolve to /etc/passwd.
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
        assert!(
            !resolved.ends_with("etc/passwd"),
            "Backslash traversal must not resolve to /etc/passwd on Linux"
        );
    }
    Ok(())
}

#[test]
fn windows_drive_letter_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // C:\Windows\System32 -- on Linux this is a relative literal path
    let path = PathBuf::from("C:\\Windows\\System32");
    let result = validate_workspace_path(&path, &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_unc_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // UNC path \\server\share -- on Linux treated as literal
    let path = PathBuf::from("\\\\server\\share\\file.pl");
    let result = validate_workspace_path(&path, &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn mixed_forward_and_back_slash_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = PathBuf::from("sub/..\\..\\etc/passwd");
    let result = validate_workspace_path(&path, &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_dot_backslash_prefix() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let path = PathBuf::from(".\\..\\..\\etc\\passwd");
    let result = validate_workspace_path(&path, &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 5. Symlink traversal -- advanced scenarios (Unix only)
// ===========================================================================

#[cfg(unix)]
#[test]
fn symlink_circular_does_not_escape() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    // Create circular symlinks: a -> b, b -> a
    let link_a = ws.join("link_a");
    let link_b = ws.join("link_b");
    std::os::unix::fs::symlink(&link_b, &link_a)?;
    std::os::unix::fs::symlink(&link_a, &link_b)?;

    // Attempting to resolve should fail (circular) -- the important thing is
    // it does not somehow escape the workspace
    let result = validate_workspace_path(&PathBuf::from("link_a/file.pl"), ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&canonicalized(ws)?));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_deep_chain_three_levels_escape() -> TestResult {
    let outer = tempfile::tempdir()?;
    let ws = outer.path().join("workspace");
    let secret = outer.path().join("secret");
    std::fs::create_dir(&ws)?;
    std::fs::create_dir(&secret)?;
    std::fs::write(secret.join("key.pem"), "SECRET KEY")?;

    // link3 -> secret, link2 -> link3, link1 -> link2
    let link3 = ws.join("link3");
    std::os::unix::fs::symlink(&secret, &link3)?;
    let link2 = ws.join("link2");
    std::os::unix::fs::symlink(&link3, &link2)?;
    let link1 = ws.join("link1");
    std::os::unix::fs::symlink(&link2, &link1)?;

    let result = validate_workspace_path(&PathBuf::from("link1/key.pem"), &ws);
    assert!(result.is_err(), "Three-level symlink chain escaping workspace must be blocked");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_relative_escape_via_dotdot() -> TestResult {
    let outer = tempfile::tempdir()?;
    let ws = outer.path().join("workspace");
    let secret = outer.path().join("secret");
    std::fs::create_dir(&ws)?;
    std::fs::create_dir(&secret)?;
    std::fs::write(secret.join("data.txt"), "sensitive")?;

    // Create a symlink that uses relative ".." to escape
    // ws/escape -> ../secret
    let link = ws.join("escape");
    std::os::unix::fs::symlink(std::path::Path::new("../secret"), &link)?;

    let result = validate_workspace_path(&PathBuf::from("escape/data.txt"), &ws);
    assert!(result.is_err(), "Relative symlink escaping workspace must be blocked");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_proc_self() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let link = ws.join("proc_link");
    std::os::unix::fs::symlink(std::path::Path::new("/proc/self"), &link)?;

    let result = validate_workspace_path(&PathBuf::from("proc_link/environ"), ws);
    assert!(result.is_err(), "Symlink to /proc/self must be blocked");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_workspace_subdirectory_is_valid() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    std::fs::create_dir_all(ws.join("src/deep"))?;
    std::fs::write(ws.join("src/deep/mod.pl"), "1;")?;

    // Create a shortcut link within workspace
    let link = ws.join("shortcut");
    std::os::unix::fs::symlink(ws.join("src/deep"), &link)?;

    let result = validate_workspace_path(&PathBuf::from("shortcut/mod.pl"), ws)?;
    assert!(result.starts_with(&canonicalized(ws)?));
    Ok(())
}

// ===========================================================================
// 6. sanitize_completion_path_input -- hardened tests
// ===========================================================================

#[test]
fn sanitize_completion_rejects_null_byte() {
    assert!(sanitize_completion_path_input("lib/Foo\0.pm").is_none());
}

#[test]
fn sanitize_completion_rejects_parent_traversal() {
    assert!(sanitize_completion_path_input("../etc/passwd").is_none());
    assert!(sanitize_completion_path_input("sub/../../etc").is_none());
    assert!(sanitize_completion_path_input("..").is_none());
}

#[test]
fn sanitize_completion_rejects_absolute_path() {
    assert!(sanitize_completion_path_input("/etc/passwd").is_none());
    assert!(sanitize_completion_path_input("/usr/bin/perl").is_none());
}

#[test]
fn sanitize_completion_rejects_backslash_traversal() {
    assert!(sanitize_completion_path_input("..\\etc\\passwd").is_none());
    assert!(sanitize_completion_path_input("sub\\..\\..\\etc").is_none());
}

#[test]
fn sanitize_completion_normalizes_backslash_to_forward_slash() {
    // Valid path with backslashes should be normalized
    let result = sanitize_completion_path_input("lib\\Foo.pm");
    assert_eq!(result, Some("lib/Foo.pm".to_string()));
}

#[test]
fn sanitize_completion_allows_root_slash() {
    // Special case: "/" alone is allowed
    assert_eq!(sanitize_completion_path_input("/"), Some("/".to_string()));
}

#[test]
fn sanitize_completion_allows_valid_relative() {
    assert_eq!(
        sanitize_completion_path_input("lib/My/Module.pm"),
        Some("lib/My/Module.pm".to_string())
    );
}

#[test]
fn sanitize_completion_allows_simple_filename() {
    assert_eq!(sanitize_completion_path_input("Foo.pm"), Some("Foo.pm".to_string()));
}

#[test]
fn sanitize_completion_allows_current_dir_prefix() {
    // Current directory prefix should be allowed since it does not escape
    let result = sanitize_completion_path_input("./lib");
    // Component::CurDir is not blocked by the function
    assert!(result.is_some());
}

#[test]
fn sanitize_completion_unicode_two_dot_leader() {
    // U+2025 TWO DOT LEADER should pass sanitization (it's not "..")
    let result = sanitize_completion_path_input("\u{2025}/etc");
    assert!(result.is_some());
}

#[test]
fn sanitize_completion_windows_drive_letter() {
    // Windows drive prefix should be rejected
    if cfg!(windows) {
        assert!(sanitize_completion_path_input("C:\\Windows\\System32").is_none());
    }
}

// ===========================================================================
// 7. is_safe_completion_filename -- edge cases
// ===========================================================================

#[test]
fn safe_filename_rejects_empty() {
    assert!(!is_safe_completion_filename(""));
}

#[test]
fn safe_filename_rejects_too_long() {
    let long_name = "a".repeat(256);
    assert!(!is_safe_completion_filename(&long_name));
}

#[test]
fn safe_filename_accepts_max_length() {
    let max_name = "a".repeat(255);
    assert!(is_safe_completion_filename(&max_name));
}

#[test]
fn safe_filename_rejects_null_byte() {
    assert!(!is_safe_completion_filename("foo\0bar"));
}

#[test]
fn safe_filename_rejects_control_chars() {
    assert!(!is_safe_completion_filename("foo\x01bar"));
    assert!(!is_safe_completion_filename("foo\x1fbar"));
    assert!(!is_safe_completion_filename("foo\x7fbar")); // DEL
}

#[test]
fn safe_filename_rejects_windows_reserved() {
    assert!(!is_safe_completion_filename("CON"));
    assert!(!is_safe_completion_filename("PRN"));
    assert!(!is_safe_completion_filename("AUX"));
    assert!(!is_safe_completion_filename("NUL"));
    assert!(!is_safe_completion_filename("COM1"));
    assert!(!is_safe_completion_filename("LPT1"));
    assert!(!is_safe_completion_filename("CON.txt"));
    assert!(!is_safe_completion_filename("nul.pl")); // case insensitive
}

#[test]
fn safe_filename_accepts_normal_names() {
    assert!(is_safe_completion_filename("Module.pm"));
    assert!(is_safe_completion_filename("script.pl"));
    assert!(is_safe_completion_filename("test.t"));
    assert!(is_safe_completion_filename(".gitignore"));
    assert!(is_safe_completion_filename("Makefile.PL"));
}

#[test]
fn safe_filename_accepts_unicode_names() {
    assert!(is_safe_completion_filename("\u{65E5}\u{672C}\u{8A9E}.pm")); // Japanese
    assert!(is_safe_completion_filename("\u{0410}\u{0411}\u{0412}.pm")); // Cyrillic
}

// ===========================================================================
// 8. is_hidden_or_forbidden_entry_name -- coverage
// ===========================================================================

#[test]
fn hidden_entries_detected() {
    assert!(is_hidden_or_forbidden_entry_name(".git"));
    assert!(is_hidden_or_forbidden_entry_name(".svn"));
    assert!(is_hidden_or_forbidden_entry_name(".hg"));
    assert!(is_hidden_or_forbidden_entry_name(".cargo"));
    assert!(is_hidden_or_forbidden_entry_name(".rustup"));
    assert!(is_hidden_or_forbidden_entry_name("node_modules"));
    assert!(is_hidden_or_forbidden_entry_name("target"));
    assert!(is_hidden_or_forbidden_entry_name("build"));
    assert!(is_hidden_or_forbidden_entry_name("__pycache__"));
    assert!(is_hidden_or_forbidden_entry_name(".pytest_cache"));
    assert!(is_hidden_or_forbidden_entry_name(".mypy_cache"));
    assert!(is_hidden_or_forbidden_entry_name("System Volume Information"));
    assert!(is_hidden_or_forbidden_entry_name("$RECYCLE.BIN"));
}

#[test]
fn non_hidden_entries_pass() {
    assert!(!is_hidden_or_forbidden_entry_name("lib"));
    assert!(!is_hidden_or_forbidden_entry_name("src"));
    assert!(!is_hidden_or_forbidden_entry_name("t"));
    assert!(!is_hidden_or_forbidden_entry_name("blib"));
    assert!(!is_hidden_or_forbidden_entry_name(".")); // single dot (current dir) has len 1
}

#[test]
fn hidden_dotfiles_with_longer_names() {
    // Any file starting with "." and len > 1 is hidden
    assert!(is_hidden_or_forbidden_entry_name(".perltidyrc"));
    assert!(is_hidden_or_forbidden_entry_name(".perlcriticrc"));
    assert!(is_hidden_or_forbidden_entry_name(".env"));
}

// ===========================================================================
// 9. split_completion_path_components -- edge cases
// ===========================================================================

#[test]
fn split_path_with_nested_dirs() {
    assert_eq!(
        split_completion_path_components("lib/My/Module"),
        ("lib/My".to_string(), "Module".to_string())
    );
}

#[test]
fn split_path_with_trailing_slash() {
    assert_eq!(split_completion_path_components("lib/"), ("lib".to_string(), String::new()));
}

#[test]
fn split_path_bare_filename() {
    assert_eq!(split_completion_path_components("Foo.pm"), (".".to_string(), "Foo.pm".to_string()));
}

#[test]
fn split_path_empty_string() {
    assert_eq!(split_completion_path_components(""), (".".to_string(), String::new()));
}

// ===========================================================================
// 10. build_completion_path -- edge cases
// ===========================================================================

#[test]
fn build_path_dot_dir_file() {
    assert_eq!(build_completion_path(".", "script.pl", false), "script.pl");
}

#[test]
fn build_path_dot_dir_directory() {
    assert_eq!(build_completion_path(".", "lib", true), "lib/");
}

#[test]
fn build_path_nested_dir() {
    assert_eq!(build_completion_path("lib/My", "Module.pm", false), "lib/My/Module.pm");
}

#[test]
fn build_path_trailing_slash_normalized() {
    assert_eq!(build_completion_path("lib/", "Module.pm", false), "lib/Module.pm");
}

// ===========================================================================
// 11. resolve_completion_base_directory -- edge cases
// ===========================================================================

#[test]
fn resolve_base_dir_dot() {
    let result = resolve_completion_base_directory(".");
    assert!(result.is_some());
}

#[test]
fn resolve_base_dir_absolute_non_root_rejected() {
    assert!(resolve_completion_base_directory("/etc").is_none());
    assert!(resolve_completion_base_directory("/usr/bin").is_none());
}

#[test]
fn resolve_base_dir_root_slash_not_rejected() {
    // "/" is the only absolute path that is allowed
    // The function allows it (path.is_absolute() && dir_part != "/")
    let result = resolve_completion_base_directory("/");
    // "/" exists on Linux, so it should canonicalize or return Some
    if std::path::Path::new("/").exists() {
        assert!(result.is_some());
    }
}

// ===========================================================================
// 12. Multi-encoding / double-encoding bypass attempts
// ===========================================================================

#[test]
fn double_encoded_dot_dot_stays_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %252e%252e -> double URL encoding of ".." -- OS treats as literal
    let result = validate_workspace_path(&PathBuf::from("%252e%252e/%252e%252e/etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn html_entity_encoded_dot_dot_stays_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // &#46;&#46; -> HTML entities for ".." -- OS treats as literal
    let result = validate_workspace_path(&PathBuf::from("&#46;&#46;/&#46;&#46;/etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn backslash_u_encoded_dot_dot_stays_literal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // \u002e\u002e -> unicode escape for ".." -- OS treats as literal string
    let result =
        validate_workspace_path(&PathBuf::from("\\u002e\\u002e/\\u002e\\u002e/etc/passwd"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 13. Control character injection -- comprehensive
// ===========================================================================

#[test]
fn control_char_soh_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x01.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_stx_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x02.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_etx_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x03.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_eot_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x04.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_nak_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x15.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn control_char_sub_rejected() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("file\x1a.pl"), &ws);
    assert!(matches!(result, Err(WorkspacePathError::InvalidPathCharacters)));
    Ok(())
}

// ===========================================================================
// 14. Adversarial traversal patterns -- real-world attack vectors
// ===========================================================================

#[test]
fn traversal_with_current_dir_padding() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Pad with "./" to try to confuse depth counting
    let result =
        validate_workspace_path(&PathBuf::from("./sub/./././../././../../../etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_alternating_down_up() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Pattern: a/../b/../c/../d/../../../etc/passwd
    // Net: go up 3 from workspace (escape)
    let result =
        validate_workspace_path(&PathBuf::from("a/../b/../c/../d/../../../etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_thousand_parent_dirs() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(1000) + "etc/passwd";
    let result = validate_workspace_path(&PathBuf::from(&evil), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn traversal_hidden_in_long_valid_prefix() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Long valid prefix followed by escape
    let prefix = "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z";
    let ups = "../".repeat(27); // 26 dirs down + 1 more to escape
    let evil = format!("{prefix}/{ups}etc/passwd");
    let result = validate_workspace_path(&PathBuf::from(&evil), &ws);
    assert!(result.is_err());
    Ok(())
}

// ===========================================================================
// 15. Absolute path injection -- additional patterns
// ===========================================================================

#[test]
fn absolute_path_to_etc_hosts() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/etc/hosts"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_path_to_root_bashrc() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/root/.bashrc"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_path_to_var_log() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_workspace_path(&PathBuf::from("/var/log/syslog"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn absolute_inside_workspace_is_valid() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    let file = ws.join("script.pl");
    std::fs::write(&file, "1;")?;

    let result = validate_workspace_path(&file, ws)?;
    assert!(result.starts_with(&canonicalized(ws)?));
    Ok(())
}
