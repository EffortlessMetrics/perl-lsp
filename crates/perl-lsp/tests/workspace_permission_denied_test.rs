//! Tests for permission-denied surfacing during workspace indexing.
//!
//! Issue #4194: when workspace files cannot be read due to permission errors,
//! the server must emit a ONE-TIME `window/showMessage` warning and a
//! per-file `textDocument/publishDiagnostics` diagnostic instead of silently
//! skipping the file.

#![allow(unused_imports, dead_code)]

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;
use support::test_workspace::TempWorkspace;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn indexing_timeout() -> Duration {
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    if is_ci { Duration::from_secs(12) } else { Duration::from_secs(6) }
}

// ---------------------------------------------------------------------------
// Unit test: is_permission_denied helper covers both Unix and Windows codes.
// ---------------------------------------------------------------------------

/// Helper that mirrors the production `is_permission_denied` logic.
/// We test the logic here so it runs on all platforms without real FS ops.
fn is_permission_denied(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    // Windows ERROR_ACCESS_DENIED = os error 5
    #[cfg(windows)]
    if e.raw_os_error() == Some(5) {
        return true;
    }
    false
}

#[test]
fn is_permission_denied_detects_standard_kind() {
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    assert!(is_permission_denied(&e), "should detect PermissionDenied kind");
}

#[test]
fn is_permission_denied_ignores_other_errors() {
    let e = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    assert!(!is_permission_denied(&e), "should not flag NotFound as permission-denied");

    let e2 = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
    assert!(!is_permission_denied(&e2), "should not flag BrokenPipe as permission-denied");
}

#[test]
#[cfg(windows)]
fn is_permission_denied_detects_windows_error_5() {
    // ERROR_ACCESS_DENIED is os error 5 on Windows
    let e = std::io::Error::from_raw_os_error(5);
    assert!(is_permission_denied(&e), "should detect Windows ERROR_ACCESS_DENIED (os error 5)");
}

// ---------------------------------------------------------------------------
// Integration test: permission-denied file during workspace scan.
//
// Unix only — Windows has different ACL semantics that make it impossible
// to reliably create a permission-denied file from a non-admin process.
// The production code handles Windows os_error(5) via `raw_os_error()`,
// but we can only exercise that code path on Windows proper.
// ---------------------------------------------------------------------------

/// Returns true if we can create a permission-denied file on this platform.
/// Returns false when running as root (root bypasses permission checks).
#[cfg(unix)]
fn can_create_permission_denied() -> bool {
    use std::os::unix::fs::PermissionsExt;
    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(_) => return false,
    };
    let probe = dir.path().join("probe.pm");
    if std::fs::write(&probe, "1;").is_err() {
        return false;
    }
    let mut perms = match std::fs::metadata(&probe) {
        Ok(m) => m.permissions(),
        Err(_) => return false,
    };
    perms.set_mode(0o000);
    let _ = std::fs::set_permissions(&probe, &perms);
    // If we can still read it, we're root
    let readable = std::fs::read_to_string(&probe).is_ok();
    // Restore so tempdir cleanup succeeds
    perms.set_mode(0o644);
    let _ = std::fs::set_permissions(&probe, &perms);
    !readable
}

/// Create a workspace with one normal Perl file and one permission-denied file.
#[cfg(unix)]
fn make_workspace_with_permission_denied_file()
-> Result<(TempWorkspace, std::path::PathBuf), String> {
    use std::os::unix::fs::PermissionsExt;
    let ws = TempWorkspace::new()?;

    // Normal file — indexed without error
    ws.write("lib/Normal.pm", "package Normal;\nsub new { bless {}, shift }\n1;\n")?;

    // Create an unreadable file: write it, then chmod 000
    let secret_path = ws.dir.path().join("lib/Secret.pm");
    std::fs::write(&secret_path, "package Secret;\n1;\n")
        .map_err(|e| format!("write Secret.pm: {e}"))?;

    let mut perms =
        std::fs::metadata(&secret_path).map_err(|e| format!("metadata: {e}"))?.permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&secret_path, perms).map_err(|e| format!("chmod: {e}"))?;

    Ok((ws, secret_path))
}

/// Restore permissions so TempDir cleanup succeeds.
#[cfg(unix)]
fn restore_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o644);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[test]
#[cfg(unix)]
fn permission_denied_file_emits_show_message_once() -> TestResult {
    if !can_create_permission_denied() {
        eprintln!("Skipping: cannot create permission-denied file (likely running as root)");
        return Ok(());
    }

    let (ws, secret_path) = make_workspace_with_permission_denied_file()?;

    let mut harness = LspHarness::new_raw();
    // Advertise no workDoneProgress so we don't need to handle progress flow.
    harness.initialize_with_root(
        &ws.root_uri,
        Some(json!({ "window": { "workDoneProgress": false } })),
    )?;

    let timeout = indexing_timeout();

    // Give the background indexing thread time to attempt the scan.
    std::thread::sleep(timeout.min(Duration::from_secs(3)));

    // Collect all window/showMessage notifications.
    let show_messages = harness.drain_notifications(Some("window/showMessage"), 500);

    // Restore permissions before any assertions so cleanup can succeed.
    restore_permissions(&secret_path);

    // There must be exactly ONE window/showMessage about permission-denied.
    let permission_msgs: Vec<_> = show_messages
        .iter()
        .filter(|n| {
            n.pointer("/params/message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_lowercase().contains("permission"))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(
        permission_msgs.len(),
        1,
        "expected exactly ONE window/showMessage about permission-denied, got {}: {:?}",
        permission_msgs.len(),
        show_messages
    );

    // The message type must be Warning (2) or Error (1).
    let msg_type = permission_msgs[0].pointer("/params/type").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(
        msg_type == 1 || msg_type == 2,
        "expected Warning(2) or Error(1) message type, got {msg_type}"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn permission_denied_file_emits_per_file_diagnostic() -> TestResult {
    if !can_create_permission_denied() {
        eprintln!("Skipping: cannot create permission-denied file (likely running as root)");
        return Ok(());
    }

    let (ws, secret_path) = make_workspace_with_permission_denied_file()?;

    let secret_uri =
        url::Url::from_file_path(&secret_path).map_err(|_| "failed to build URI")?.to_string();

    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(
        &ws.root_uri,
        Some(json!({ "window": { "workDoneProgress": false } })),
    )?;

    let timeout = indexing_timeout();
    std::thread::sleep(timeout.min(Duration::from_secs(3)));

    // Collect publishDiagnostics notifications.
    let diagnostics = harness.drain_notifications(Some("textDocument/publishDiagnostics"), 500);

    restore_permissions(&secret_path);

    // There must be a diagnostic for the unreadable file.
    let secret_diag = diagnostics.iter().find(|n| {
        n.pointer("/params/uri")
            .and_then(|v| v.as_str())
            .map(|u| u.eq_ignore_ascii_case(&secret_uri))
            .unwrap_or(false)
            && n.pointer("/params/diagnostics")
                .and_then(|v| v.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false)
    });

    assert!(
        secret_diag.is_some(),
        "expected a publishDiagnostics for the unreadable file {secret_uri}; \
         diagnostics received: {diagnostics:?}"
    );

    Ok(())
}

#[test]
#[cfg(unix)]
fn permission_denied_show_message_fires_only_once_for_multiple_files() -> TestResult {
    if !can_create_permission_denied() {
        eprintln!("Skipping: cannot create permission-denied file (likely running as root)");
        return Ok(());
    }

    use std::os::unix::fs::PermissionsExt;
    let ws = TempWorkspace::new()?;
    ws.write("lib/Normal.pm", "package Normal;\n1;\n")?;

    // Create TWO unreadable files.
    let secret1 = ws.dir.path().join("lib/Secret1.pm");
    let secret2 = ws.dir.path().join("lib/Secret2.pm");
    std::fs::write(&secret1, "package Secret1;\n1;\n").map_err(|e| e.to_string())?;
    std::fs::write(&secret2, "package Secret2;\n1;\n").map_err(|e| e.to_string())?;

    for path in [&secret1, &secret2] {
        let mut perms = std::fs::metadata(path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }

    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(
        &ws.root_uri,
        Some(json!({ "window": { "workDoneProgress": false } })),
    )?;

    let timeout = indexing_timeout();
    std::thread::sleep(timeout.min(Duration::from_secs(3)));

    let show_messages = harness.drain_notifications(Some("window/showMessage"), 500);

    // Restore permissions.
    for path in [&secret1, &secret2] {
        restore_permissions(path);
    }

    // Even with two unreadable files, only ONE window/showMessage should be sent.
    let permission_msgs: Vec<_> = show_messages
        .iter()
        .filter(|n| {
            n.pointer("/params/message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_lowercase().contains("permission"))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(
        permission_msgs.len(),
        1,
        "expected exactly ONE showMessage even for multiple unreadable files, \
         got {}: {show_messages:?}",
        permission_msgs.len()
    );

    Ok(())
}
