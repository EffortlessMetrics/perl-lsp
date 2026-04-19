//! Hardened security tests for DAP path traversal prevention.
//!
//! Exercises DAP-specific security validation against adversarial inputs:
//! - Path traversal via `validate_path`
//! - Null byte injection
//! - Symlink traversal
//! - Unicode normalization attacks
//! - Windows-style path separators on Linux
//! - Expression / condition injection
//! - Timeout boundary testing

use perl_dap::security::{
    DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression,
    validate_path, validate_timeout,
};
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

// ===========================================================================
// 1. Path traversal -- classic patterns via DAP validate_path
// ===========================================================================

#[test]
fn dap_traversal_single_parent() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new(".."), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_traversal_deep_escape() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../etc/passwd"), &ws);
    assert!(result.is_err());
    match result {
        Err(SecurityError::PathTraversalAttempt(_))
        | Err(SecurityError::PathOutsideWorkspace(_)) => {}
        Err(e) => return Err(format!("unexpected error: {e:?}").into()),
        Ok(_) => return Err("expected error".into()),
    }
    Ok(())
}

#[test]
fn dap_traversal_interleaved() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("a/../b/../../secret"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_traversal_many_parents() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(50) + "etc/passwd";
    let result = validate_path(Path::new(&evil), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_traversal_with_dot_padding() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("./././../../../etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_valid_relative_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("src/main.pl"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn dap_valid_dotfile() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new(".perltidyrc"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 2. Absolute path injection via DAP
// ===========================================================================

#[test]
fn dap_absolute_etc_passwd() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_absolute_proc_self_environ() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/proc/self/environ"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_absolute_dev_null() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/dev/null"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_absolute_root() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_absolute_inside_workspace_is_valid() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    let file = ws.join("debug_target.pl");
    std::fs::write(&file, "1;")?;

    let result = validate_path(&file, ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ===========================================================================
// 3. Null byte injection via DAP
// ===========================================================================

#[test]
fn dap_null_byte_in_filename() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\0.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_null_byte_after_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../\0etc/passwd"), &ws);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn dap_null_byte_in_extension() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("script.pl\0.txt"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_null_byte_only() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("\0"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_null_byte_multiple() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("a\0b\0c"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

// ===========================================================================
// 4. Control character injection via DAP
// ===========================================================================

#[test]
fn dap_control_newline() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\n.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_control_carriage_return() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\r.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_control_escape_char() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\x1b.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_control_del() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\x7f.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn dap_tab_is_allowed() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("file\t.pl"), &ws);
    // Tab is explicitly allowed -- should not produce InvalidPathCharacters
    assert!(!matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

// ===========================================================================
// 5. Symlink traversal via DAP (Unix only)
// ===========================================================================

#[cfg(unix)]
#[test]
fn dap_symlink_escape_to_etc() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let link = ws.join("escape_link");
    std::os::unix::fs::symlink(Path::new("/etc"), &link)?;

    let result = validate_path(Path::new("escape_link/passwd"), ws);
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn dap_symlink_relative_escape() -> TestResult {
    let outer = tempfile::tempdir()?;
    let ws = outer.path().join("workspace");
    let secret = outer.path().join("secret");
    std::fs::create_dir(&ws)?;
    std::fs::create_dir(&secret)?;
    std::fs::write(secret.join("key.pem"), "SECRET")?;

    let link = ws.join("escape");
    std::os::unix::fs::symlink(Path::new("../secret"), &link)?;

    let result = validate_path(Path::new("escape/key.pem"), &ws);
    assert!(
        result.is_err(),
        "Relative symlink escape via DAP must be blocked"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dap_symlink_chain_escape() -> TestResult {
    let outer = tempfile::tempdir()?;
    let ws = outer.path().join("workspace");
    let secret = outer.path().join("secret");
    std::fs::create_dir(&ws)?;
    std::fs::create_dir(&secret)?;
    std::fs::write(secret.join("data"), "sensitive")?;

    let link2 = ws.join("link2");
    std::os::unix::fs::symlink(&secret, &link2)?;
    let link1 = ws.join("link1");
    std::os::unix::fs::symlink(&link2, &link1)?;

    let result = validate_path(Path::new("link1/data"), &ws);
    assert!(
        result.is_err(),
        "Chained symlink escape via DAP must be blocked"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dap_symlink_to_proc_rejected() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let link = ws.join("proc_link");
    std::os::unix::fs::symlink(Path::new("/proc/self"), &link)?;

    let result = validate_path(Path::new("proc_link/environ"), ws);
    assert!(
        result.is_err(),
        "Symlink to /proc/self must be blocked in DAP"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dap_symlink_within_workspace_is_valid() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();

    let target = ws.join("real_dir");
    std::fs::create_dir(&target)?;
    std::fs::write(target.join("script.pl"), "1;")?;

    let link = ws.join("alias");
    std::os::unix::fs::symlink(&target, &link)?;

    let result = validate_path(Path::new("alias/script.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ===========================================================================
// 6. Unicode normalization attacks via DAP
// ===========================================================================

#[test]
fn dap_unicode_two_dot_leader_u2025() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // U+2025 TWO DOT LEADER is NOT ".." -- on Linux it is a literal filename character.
    // The path resolves inside workspace as literal subdirectories.
    // Security invariant: the resolved path MUST be within the workspace.
    let evil = "\u{2025}/\u{2025}/etc/passwd";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws), "Path must stay within workspace");
    }
    Ok(())
}

#[test]
fn dap_unicode_fullwidth_period_u_ff0e() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "\u{FF0E}\u{FF0E}/etc/passwd";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_unicode_fullwidth_solidus_u_ff0f() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "..\u{FF0F}..\u{FF0F}etc\u{FF0F}passwd";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_unicode_one_dot_leader_u2024() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "\u{2024}\u{2024}/\u{2024}\u{2024}/etc/passwd";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_unicode_rlo_bidi_override() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Right-to-left override can be used to disguise filenames
    let evil = "\u{202E}fdssap/cte/../../../";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_unicode_zero_width_space_in_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = ".\u{200B}./etc/passwd";
    let result = validate_path(Path::new(evil), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 7. Windows-style path separators via DAP
// ===========================================================================

#[test]
fn dap_windows_backslash_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..\\..\\etc\\passwd"), &ws);
    // On Linux, backslash is a literal char -- must not resolve to /etc/passwd
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_windows_drive_letter() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("C:\\Windows\\System32"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dap_windows_unc_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("\\\\server\\share\\file"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 8. Expression validation -- protocol injection
// ===========================================================================

#[test]
fn dap_expression_valid_simple() -> TestResult {
    validate_expression("$x + 1")?;
    validate_expression("my_func()")?;
    validate_expression("$hash{key}")?;
    validate_expression("scalar @array")?;
    validate_expression("defined($var) && $var > 0")?;
    Ok(())
}

#[test]
fn dap_expression_empty_is_valid() -> TestResult {
    validate_expression("")?;
    Ok(())
}

#[test]
fn dap_expression_rejects_newline() -> TestResult {
    let result = validate_expression("1\nprint 'injected'");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

#[test]
fn dap_expression_rejects_carriage_return() -> TestResult {
    let result = validate_expression("1\rprint 'injected'");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

#[test]
fn dap_expression_rejects_crlf() -> TestResult {
    let result = validate_expression("1\r\nprint 'injected'");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

#[test]
fn dap_expression_allows_tabs_and_spaces() -> TestResult {
    validate_expression("$x\t+ 1")?;
    validate_expression("  $x  ")?;
    Ok(())
}

#[test]
fn dap_expression_with_multiline_perl_attack() -> TestResult {
    let result = validate_expression("system('rm -rf /')\nprint 'done'");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

// ===========================================================================
// 9. Condition validation -- protocol injection
// ===========================================================================

#[test]
fn dap_condition_valid() -> TestResult {
    validate_condition("$x > 10")?;
    validate_condition("defined($var)")?;
    validate_condition("$i == 42 && $j < 100")?;
    Ok(())
}

#[test]
fn dap_condition_rejects_newline() -> TestResult {
    let result = validate_condition("$x > 10\nsystem('id')");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

#[test]
fn dap_condition_rejects_cr() -> TestResult {
    let result = validate_condition("$x > 10\rsystem('id')");
    assert!(matches!(result, Err(SecurityError::InvalidExpression)));
    Ok(())
}

// ===========================================================================
// 10. Timeout validation -- boundary testing
// ===========================================================================

#[test]
fn dap_timeout_zero_capped_to_one() -> TestResult {
    assert_eq!(validate_timeout(0)?, 1);
    Ok(())
}

#[test]
fn dap_timeout_one_is_minimum() -> TestResult {
    assert_eq!(validate_timeout(1)?, 1);
    Ok(())
}

#[test]
fn dap_timeout_default_unchanged() -> TestResult {
    assert_eq!(validate_timeout(DEFAULT_TIMEOUT_MS)?, DEFAULT_TIMEOUT_MS);
    Ok(())
}

#[test]
fn dap_timeout_max_unchanged() -> TestResult {
    assert_eq!(validate_timeout(MAX_TIMEOUT_MS)?, MAX_TIMEOUT_MS);
    Ok(())
}

#[test]
fn dap_timeout_over_max_returns_error() {
    assert!(validate_timeout(MAX_TIMEOUT_MS + 1).is_err());
    assert!(validate_timeout(u32::MAX).is_err());
}

#[test]
fn dap_timeout_normal_values_unchanged() -> TestResult {
    assert_eq!(validate_timeout(1000)?, 1000);
    assert_eq!(validate_timeout(5000)?, 5000);
    assert_eq!(validate_timeout(60_000)?, 60_000);
    assert_eq!(validate_timeout(100_000)?, 100_000);
    Ok(())
}

// ===========================================================================
// 11. SecurityError -- from WorkspacePathError conversion
// ===========================================================================

#[test]
fn dap_error_from_traversal() -> TestResult {
    let ws_err =
        perl_path_security::WorkspacePathError::PathTraversalAttempt("test path".to_string());
    let sec_err: SecurityError = ws_err.into();
    match sec_err {
        SecurityError::PathTraversalAttempt(msg) => assert!(msg.contains("test path")),
        other => return Err(format!("Expected PathTraversalAttempt, got: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn dap_error_from_outside_workspace() -> TestResult {
    let ws_err =
        perl_path_security::WorkspacePathError::PathOutsideWorkspace("outside path".to_string());
    let sec_err: SecurityError = ws_err.into();
    match sec_err {
        SecurityError::PathOutsideWorkspace(msg) => assert!(msg.contains("outside path")),
        other => return Err(format!("Expected PathOutsideWorkspace, got: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn dap_error_from_invalid_chars() {
    let ws_err = perl_path_security::WorkspacePathError::InvalidPathCharacters;
    let sec_err: SecurityError = ws_err.into();
    assert!(matches!(sec_err, SecurityError::InvalidPathCharacters));
}

// ===========================================================================
// 12. SecurityError -- Display messages
// ===========================================================================

#[test]
fn dap_error_display_traversal() {
    let err = SecurityError::PathTraversalAttempt("../../../etc".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Path traversal attempt detected"));
    assert!(msg.contains("../../../etc"));
}

#[test]
fn dap_error_display_outside() {
    let err = SecurityError::PathOutsideWorkspace("/tmp/evil".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Path outside workspace"));
    assert!(msg.contains("/tmp/evil"));
}

#[test]
fn dap_error_display_symlink() {
    let err = SecurityError::SymlinkOutsideWorkspace("/etc".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Symlink resolves outside workspace"));
    assert!(msg.contains("/etc"));
}

#[test]
fn dap_error_display_invalid_chars() {
    let err = SecurityError::InvalidPathCharacters;
    let msg = format!("{err}");
    assert!(msg.contains("Invalid path characters detected"));
}

#[test]
fn dap_error_display_invalid_expression() {
    let err = SecurityError::InvalidExpression;
    let msg = format!("{err}");
    assert!(msg.contains("Expression cannot contain newlines"));
}

#[test]
fn dap_error_display_excessive_timeout() {
    let err = SecurityError::ExcessiveTimeout(999_999);
    let msg = format!("{err}");
    assert!(msg.contains("999999ms"));
}

// ===========================================================================
// 13. Double-encoding bypass attempts via DAP
// ===========================================================================

#[test]
fn dap_double_encoded_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %252e%252e is double-URL-encoded ".." -- OS treats as literal
    let result = validate_path(Path::new("%252e%252e/%252e%252e/etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn dap_percent_encoded_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("%2e%2e/%2e%2e/etc/passwd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn dap_percent_encoded_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..%2f..%2fetc%2fpasswd"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

// ===========================================================================
// 14. Concurrent access safety
// ===========================================================================

#[test]
fn dap_concurrent_validations() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().to_path_buf();

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let ws_clone = ws.clone();
            std::thread::spawn(move || {
                let path = if i % 3 == 0 {
                    PathBuf::from(format!("valid_{i}.pl"))
                } else if i % 3 == 1 {
                    PathBuf::from("../../../etc/passwd")
                } else {
                    PathBuf::from("file\0evil.pl")
                };
                (i, validate_path(&path, &ws_clone))
            })
        })
        .collect();

    for handle in handles {
        if let Ok((i, result)) = handle.join() {
            match i % 3 {
                0 => assert!(result.is_ok(), "Expected OK for valid path at index {i}"),
                1 => assert!(result.is_err(), "Expected error for traversal at index {i}"),
                _ => assert!(result.is_err(), "Expected error for null byte at index {i}"),
            }
        }
    }
    Ok(())
}

// ===========================================================================
// 15. Constants are sane
// ===========================================================================

#[test]
fn dap_constants_are_reasonable() {
    assert_eq!(MAX_TIMEOUT_MS, 300_000, "Max timeout should be 5 minutes");
    assert_eq!(
        DEFAULT_TIMEOUT_MS, 5_000,
        "Default timeout should be 5 seconds"
    );
    // Compile-time assertions for constant relationships
    const {
        assert!(DEFAULT_TIMEOUT_MS < MAX_TIMEOUT_MS);
        assert!(DEFAULT_TIMEOUT_MS > 0);
    }
}
