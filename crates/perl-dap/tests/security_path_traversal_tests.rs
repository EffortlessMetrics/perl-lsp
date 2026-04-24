//! P1 security boundary tests for DAP path traversal prevention.
//!
//! Attack vectors exercised:
//! - Classic `../../etc/passwd` payloads and variants
//! - Windows-style `..\..\windows\system32` payloads
//! - Symlink attack vectors (escape, circular, deeply nested chains)
//! - Null byte injection at various positions
//! - Payload stacking (traversal + null byte + control chars)
//! - Long path traversal bombs
//! - Mixed separator confusion attacks

use perl_dap::security::{SecurityError, validate_path};
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

/// Assert that a validate_path result is a traversal or outside-workspace error.
fn assert_path_rejected(
    result: &Result<PathBuf, SecurityError>,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match result {
        Err(SecurityError::PathTraversalAttempt(_))
        | Err(SecurityError::PathOutsideWorkspace(_))
        | Err(SecurityError::InvalidPathCharacters)
        | Err(SecurityError::SymlinkOutsideWorkspace(_)) => Ok(()),
        Err(e) => {
            Err(format!("Expected traversal/boundary/char error for {context}, got: {e:?}").into())
        }
        Ok(p) => Err(format!("Expected error for {context}, got Ok({p:?})").into()),
    }
}

// ===========================================================================
// 1. Classic ../../etc/passwd payloads
// ===========================================================================

#[test]
fn passwd_single_level_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../etc/passwd"), &ws);
    assert!(result.is_err(), "Single-level traversal to /etc/passwd must be blocked");
    assert_path_rejected(&result, "../etc/passwd")?;
    Ok(())
}

#[test]
fn passwd_double_level_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../etc/passwd"), &ws);
    assert!(result.is_err(), "Double-level traversal to /etc/passwd must be blocked");
    assert_path_rejected(&result, "../../etc/passwd")?;
    Ok(())
}

#[test]
fn passwd_triple_level_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../etc/passwd"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "../../../etc/passwd")?;
    Ok(())
}

#[test]
fn passwd_deep_traversal_10_levels() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(10) + "etc/passwd";
    let result = validate_path(Path::new(&evil), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "10-level etc/passwd")?;
    Ok(())
}

#[test]
fn passwd_deep_traversal_100_levels() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(100) + "etc/passwd";
    let result = validate_path(Path::new(&evil), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "100-level etc/passwd")?;
    Ok(())
}

#[test]
fn shadow_file_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../etc/shadow"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "../../../etc/shadow")?;
    Ok(())
}

#[test]
fn proc_self_cmdline_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../proc/self/cmdline"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "../../../proc/self/cmdline")?;
    Ok(())
}

#[test]
fn proc_self_maps_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../proc/self/maps"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "../../../proc/self/maps")?;
    Ok(())
}

#[test]
fn traversal_interleaved_with_real_dirs() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Create a real subdirectory so the first part resolves, then escape
    std::fs::create_dir_all(ws.join("subdir/deep"))?;
    let result = validate_path(Path::new("subdir/deep/../../../etc/passwd"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "interleaved traversal")?;
    Ok(())
}

#[test]
fn traversal_with_current_dir_padding() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("./././../../.././etc/./passwd"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "current-dir padded traversal")?;
    Ok(())
}

#[test]
fn traversal_absolute_etc_passwd() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/etc/passwd"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "absolute /etc/passwd")?;
    Ok(())
}

#[test]
fn traversal_absolute_var_log() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("/var/log/auth.log"), &ws);
    assert!(result.is_err());
    assert_path_rejected(&result, "absolute /var/log/auth.log")?;
    Ok(())
}

// ===========================================================================
// 2. Windows-style ..\..\windows\system32 payloads
// ===========================================================================

#[test]
fn windows_backslash_traversal_system32() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..\\..\\windows\\system32"), &ws);
    // On Linux, backslash is a literal character -- the path should resolve within workspace
    if let Ok(ref resolved) = result {
        assert!(
            resolved.starts_with(&ws),
            "Windows backslash path must stay within workspace, got: {resolved:?}"
        );
    }
    Ok(())
}

#[test]
fn windows_backslash_traversal_system32_cmd() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..\\..\\windows\\system32\\cmd.exe"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_backslash_deep_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..\\..\\..\\..\\windows\\system32\\config\\sam"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_mixed_separator_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../..\\../windows/system32"), &ws);
    // Mix of forward and backward slashes -- must either reject or stay within workspace
    match &result {
        Err(_) => {} // rejected is fine
        Ok(resolved) => {
            assert!(resolved.starts_with(&ws), "Mixed separator path must stay within workspace");
        }
    }
    Ok(())
}

#[test]
fn windows_drive_letter_c_system32() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("C:\\Windows\\System32"), &ws);
    // On Linux this is relative path, must stay within workspace
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_drive_letter_d_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("D:\\sensitive\\data"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_unc_path_traversal() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("\\\\server\\share\\secret"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn windows_backslash_single_parent() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("..\\secret.txt"), &ws);
    // On Linux this is a literal filename containing backslash
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

// ===========================================================================
// 3. Symlink attack vectors (Unix only)
// ===========================================================================

#[cfg(unix)]
mod symlink_attacks {
    use super::*;

    #[test]
    fn symlink_to_etc_passwd_directly() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let ws = tmp.path();

        let link = ws.join("passwd_link");
        std::os::unix::fs::symlink(Path::new("/etc/passwd"), &link)?;

        let result = validate_path(Path::new("passwd_link"), ws);
        assert!(result.is_err(), "Symlink directly to /etc/passwd must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_to_root() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let ws = tmp.path();

        let link = ws.join("root_link");
        std::os::unix::fs::symlink(Path::new("/"), &link)?;

        let result = validate_path(Path::new("root_link/etc/passwd"), ws);
        assert!(result.is_err(), "Symlink to / then traversal to /etc/passwd must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_chain_three_hops_escaping() -> TestResult {
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        let secret = outer.path().join("secret");
        std::fs::create_dir(&ws)?;
        std::fs::create_dir(&secret)?;
        std::fs::write(secret.join("key.pem"), "SECRET_KEY")?;

        // link3 -> secret, link2 -> link3, link1 -> link2
        let link3 = ws.join("link3");
        std::os::unix::fs::symlink(&secret, &link3)?;
        let link2 = ws.join("link2");
        std::os::unix::fs::symlink(&link3, &link2)?;
        let link1 = ws.join("link1");
        std::os::unix::fs::symlink(&link2, &link1)?;

        let result = validate_path(Path::new("link1/key.pem"), &ws);
        assert!(result.is_err(), "3-hop symlink chain escaping workspace must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_to_parent_directory() -> TestResult {
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        std::fs::create_dir(&ws)?;

        let link = ws.join("parent_escape");
        std::os::unix::fs::symlink(outer.path(), &link)?;

        let result = validate_path(Path::new("parent_escape"), &ws);
        assert!(result.is_err(), "Symlink to parent directory must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_to_sibling_directory() -> TestResult {
        // Use an outer tempdir with workspace and a sibling "tmp_data" to avoid
        // self-referential issues when the workspace itself lives under /tmp.
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        let sibling = outer.path().join("tmp_data");
        std::fs::create_dir(&ws)?;
        std::fs::create_dir(&sibling)?;
        std::fs::write(sibling.join("some_file"), "data")?;

        let link = ws.join("tmp_link");
        std::os::unix::fs::symlink(&sibling, &link)?;

        let result = validate_path(Path::new("tmp_link/some_file"), &ws);
        assert!(result.is_err(), "Symlink to sibling directory must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_to_outside_home_style_dir() -> TestResult {
        // Simulate a symlink to an external "home" directory outside workspace
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        let fake_home = outer.path().join("fake_home");
        std::fs::create_dir(&ws)?;
        std::fs::create_dir_all(fake_home.join(".ssh"))?;
        std::fs::write(fake_home.join(".ssh/id_rsa"), "PRIVATE_KEY")?;

        let link = ws.join("home_link");
        std::os::unix::fs::symlink(&fake_home, &link)?;

        let result = validate_path(Path::new("home_link/.ssh/id_rsa"), &ws);
        assert!(result.is_err(), "Symlink to external home-style dir must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_relative_dotdot_escape() -> TestResult {
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        let secret = outer.path().join("secret_data");
        std::fs::create_dir(&ws)?;
        std::fs::create_dir(&secret)?;
        std::fs::write(secret.join("credentials.json"), "{}")?;

        // Relative symlink using ..
        let link = ws.join("sneaky");
        std::os::unix::fs::symlink(Path::new("../secret_data"), &link)?;

        let result = validate_path(Path::new("sneaky/credentials.json"), &ws);
        assert!(result.is_err(), "Relative symlink with .. escaping workspace must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_within_workspace_is_safe() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let ws = tmp.path();

        let target = ws.join("lib");
        std::fs::create_dir(&target)?;
        std::fs::write(target.join("Module.pm"), "package Module;")?;

        let link = ws.join("lib_alias");
        std::os::unix::fs::symlink(&target, &link)?;

        let result = validate_path(Path::new("lib_alias/Module.pm"), ws)?;
        assert!(
            result.starts_with(ws.canonicalize()?),
            "Symlink within workspace should be allowed"
        );
        Ok(())
    }

    #[test]
    fn symlink_nested_inside_subdirectory_escaping() -> TestResult {
        let outer = tempfile::tempdir()?;
        let ws = outer.path().join("workspace");
        let secret = outer.path().join("secret");
        std::fs::create_dir_all(ws.join("src/modules"))?;
        std::fs::create_dir(&secret)?;
        std::fs::write(secret.join("token"), "bearer xyz")?;

        let link = ws.join("src/modules/escape_link");
        std::os::unix::fs::symlink(&secret, &link)?;

        let result = validate_path(Path::new("src/modules/escape_link/token"), &ws);
        assert!(result.is_err(), "Symlink in nested dir escaping workspace must be blocked");
        Ok(())
    }

    #[test]
    fn symlink_to_proc_self_environ() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let ws = tmp.path();

        let link = ws.join("environ_link");
        if std::os::unix::fs::symlink(Path::new("/proc/self/environ"), &link).is_ok() {
            let result = validate_path(Path::new("environ_link"), ws);
            assert!(result.is_err(), "Symlink to /proc/self/environ must be blocked");
        }
        Ok(())
    }
}

// ===========================================================================
// 4. Null byte injection attacks
// ===========================================================================

#[test]
fn null_byte_at_start() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("\0etc/passwd"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_at_end() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("script.pl\0"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_between_traversal_and_payload() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../\0etc/passwd"), &ws);
    assert!(result.is_err(), "Null byte between traversal and payload must be blocked");
    Ok(())
}

#[test]
fn null_byte_truncation_attack() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // Classic C-string truncation: "safe.pl\0../../etc/passwd"
    // After null byte the rest is ignored in C, but Rust sees the full path
    let result = validate_path(Path::new("safe.pl\0../../etc/passwd"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_in_directory_component() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("src/\0module/lib.pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_in_extension() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("script.\0pl"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn multiple_null_bytes() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("\0\0\0"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

#[test]
fn null_byte_with_windows_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("C:\\Windows\0\\System32"), &ws);
    assert!(matches!(result, Err(SecurityError::InvalidPathCharacters)));
    Ok(())
}

// ===========================================================================
// 5. Combined/stacked attack payloads
// ===========================================================================

#[test]
fn traversal_with_control_chars() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../\x01etc/passwd"), &ws);
    assert!(result.is_err(), "Traversal + control char must be blocked");
    Ok(())
}

#[test]
fn traversal_with_carriage_return() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../etc\r/passwd"), &ws);
    assert!(result.is_err(), "Traversal + CR must be blocked");
    Ok(())
}

#[test]
fn traversal_with_newline() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../etc\n/passwd"), &ws);
    assert!(result.is_err(), "Traversal + LF must be blocked");
    Ok(())
}

#[test]
fn traversal_with_bell_char() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../etc\x07/passwd"), &ws);
    assert!(result.is_err(), "Traversal + BEL must be blocked");
    Ok(())
}

#[test]
fn traversal_with_backspace() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../etc\x08/passwd"), &ws);
    assert!(result.is_err(), "Traversal + BS must be blocked");
    Ok(())
}

#[test]
fn traversal_with_escape_sequence() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../\x1b[31metc/passwd"), &ws);
    assert!(result.is_err(), "Traversal + ANSI escape must be blocked");
    Ok(())
}

// ===========================================================================
// 6. Long path traversal bombs
// ===========================================================================

#[test]
fn traversal_bomb_1000_levels() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let evil = "../".repeat(1000) + "etc/passwd";
    let result = validate_path(Path::new(&evil), &ws);
    assert!(result.is_err(), "1000-level traversal bomb must be blocked");
    Ok(())
}

#[test]
fn very_long_path_component() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // 4096 character filename -- exceeds most filesystem limits
    let long_name = "a".repeat(4096);
    let evil = format!("../../{long_name}");
    let result = validate_path(Path::new(&evil), &ws);
    // Must either reject or stay within workspace
    match &result {
        Err(_) => {}
        Ok(resolved) => {
            assert!(resolved.starts_with(&ws), "Long path must stay within workspace");
        }
    }
    Ok(())
}

// ===========================================================================
// 7. Traversal via encoded/obfuscated patterns
// ===========================================================================

#[test]
fn url_encoded_dotdot_slash() -> TestResult {
    let (_tmp, ws) = workspace()?;
    // %2e%2e%2f == "../" URL-encoded -- OS treats as literal filename
    let result = validate_path(Path::new("%2e%2e/%2e%2e/etc/passwd"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws), "URL-encoded path must stay within workspace");
    }
    Ok(())
}

#[test]
fn double_url_encoded_dotdot() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("%252e%252e/%252e%252e/etc/passwd"), &ws);
    if let Ok(ref resolved) = result {
        assert!(resolved.starts_with(&ws));
    }
    Ok(())
}

#[test]
fn dot_dot_with_trailing_slash_variants() -> TestResult {
    let (_tmp, ws) = workspace()?;

    let payloads = ["../", "..\\/", "..%00/", "..;/"];
    for payload in &payloads {
        let evil = format!("{payload}{payload}{payload}etc/passwd");
        let result = validate_path(Path::new(&evil), &ws);
        match &result {
            Err(_) => {} // rejected is correct
            Ok(resolved) => {
                assert!(
                    resolved.starts_with(&ws),
                    "Obfuscated traversal variant '{payload}' must stay within workspace"
                );
            }
        }
    }
    Ok(())
}

// ===========================================================================
// 8. Edge cases: valid paths that should succeed
// ===========================================================================

#[test]
fn valid_nested_relative_path() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("src/lib/Module/Deep/File.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_dotfile_in_subdirectory() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("src/.hidden_config"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_file_with_dots_in_name() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("module.v2.backup.pm"), &ws)?;
    assert!(result.starts_with(&ws));
    Ok(())
}

#[test]
fn valid_path_with_safe_dotdot_that_stays_inside() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join("a/b"))?;

    // a/b/../c resolves to a/c which is still inside workspace
    let result = validate_path(Path::new("a/b/../c.pl"), ws)?;
    assert!(result.starts_with(ws.canonicalize()?));
    Ok(())
}

// ===========================================================================
// 9. Sensitive file targets beyond /etc/passwd
// ===========================================================================

#[test]
fn traversal_to_ssh_keys() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let targets = [
        "../../../root/.ssh/id_rsa",
        "../../../root/.ssh/authorized_keys",
        "../../../home/user/.ssh/id_ed25519",
    ];
    for target in &targets {
        let result = validate_path(Path::new(target), &ws);
        assert!(result.is_err(), "Traversal to {target} must be blocked");
    }
    Ok(())
}

#[test]
fn traversal_to_environment_files() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let targets =
        ["../../../etc/environment", "../../../proc/1/environ", "../../../proc/self/environ"];
    for target in &targets {
        let result = validate_path(Path::new(target), &ws);
        assert!(result.is_err(), "Traversal to {target} must be blocked");
    }
    Ok(())
}

#[test]
fn traversal_to_docker_socket() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(Path::new("../../../var/run/docker.sock"), &ws);
    assert!(result.is_err(), "Traversal to docker.sock must be blocked");
    Ok(())
}

#[test]
fn traversal_to_kubernetes_secrets() -> TestResult {
    let (_tmp, ws) = workspace()?;
    let result = validate_path(
        Path::new("../../../var/run/secrets/kubernetes.io/serviceaccount/token"),
        &ws,
    );
    assert!(result.is_err(), "Traversal to k8s secrets must be blocked");
    Ok(())
}
