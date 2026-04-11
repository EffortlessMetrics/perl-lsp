//! Integration tests for the perlcritic diagnostic pipeline.
//!
//! These tests exercise `collect_external_perlcritic_diagnostics` end-to-end via
//! the pull-diagnostics path (`textDocument/diagnostic`) without needing a real
//! `perlcritic` binary.  A mock subprocess runtime is injected through the test
//! API exposed by `LspServer::test_install_mock_critic_runtime` and
//! `LspServer::test_bypass_perlcritic_command_check`.
//!
//! Require the `expose_lsp_test_api` feature (which unlocks the internal test
//! helpers on `LspServer`) and a non-WASM target.
//!
//! Run with:
//!   RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!     --features expose_lsp_test_api -- perlcritic --test-threads=2
//!
//! Issue: #2018

#![cfg(all(not(target_arch = "wasm32"), feature = "expose_lsp_test_api"))]

use perl_lsp::LspServer;
use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use serde_json::json;
use std::sync::Arc;

/// Open `uri` with `text` via `didOpen`, then issue a pull-diagnostics request
/// and return the result.
fn pull_diagnostics(server: &LspServer, uri: &str, text: &str) -> serde_json::Value {
    server
        .test_handle_did_open(Some(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        })))
        .expect("didOpen should succeed");

    must(server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    }))))
    .unwrap_or(json!({"items": []}))
}

// ── Test A ────────────────────────────────────────────────────────────────────

/// Violations must appear in pull diagnostics when perlcritic is enabled and the
/// mock runtime returns a severity-3 violation.
///
/// Perlcritic severity 3 = Harsh → maps to LSP Warning (severity value 2).
#[test]
fn test_a_violations_appear_in_pull_diagnostics_when_enabled() {
    let server = LspServer::new();

    // Enable perlcritic with severity threshold 3.
    server.test_configure_perlcritic(true, 3, None);

    // Install a mock runtime returning one severity-3 violation for the file.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mock_line =
        b"test.pl:5:1:3:TestingAndDebugging::RequireUseStrict:Code does not use strict\n";
    runtime.add_response(MockResponse::success(mock_line.to_vec()));
    server.test_install_mock_critic_runtime(runtime);
    server.test_bypass_perlcritic_command_check();

    // Use a file:// URI that resolves to a real-looking path.
    #[cfg(windows)]
    let uri = "file:///C:/tmp/test.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test.pl";

    let result = pull_diagnostics(&server, uri, "print 'hello';\n");

    // There must be at least one diagnostic with code
    // "TestingAndDebugging::RequireUseStrict" and severity 2 (Warning).
    //
    // The pull-diagnostics response has the shape:
    //   { "kind": "full", "items": [ { "code": "...", "severity": N, ... } ], "resultId": "..." }
    let diags = result["items"].as_array().cloned().unwrap_or_default();

    let found = diags.iter().any(|d| {
        d["code"].as_str() == Some("TestingAndDebugging::RequireUseStrict")
            && d["severity"].as_u64() == Some(2)
    });

    assert!(
        found,
        "Expected a Warning diagnostic with code \
         TestingAndDebugging::RequireUseStrict in the pull response; \
         got: {result}"
    );
}

#[test]
fn test_a1_severity_five_maps_to_error() {
    let server = LspServer::new();
    server.test_configure_perlcritic(true, 5, None);

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(
        b"test.pl:2:1:5:InputOutput::RequireThreeArgOpen:Use three-arg open\n".to_vec(),
    ));
    server.test_install_mock_critic_runtime(runtime);
    server.test_bypass_perlcritic_command_check();

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_sev5.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_sev5.pl";

    let result = pull_diagnostics(&server, uri, "open FH, $path;\n");
    let diags = result["items"].as_array().cloned().unwrap_or_default();
    assert!(diags.iter().any(|d| {
        d["code"].as_str() == Some("InputOutput::RequireThreeArgOpen")
            && d["severity"].as_u64() == Some(1)
    }));
}

#[test]
fn test_a2_severity_one_maps_to_hint() {
    let server = LspServer::new();
    server.test_configure_perlcritic(true, 1, None);

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(
        b"test.pl:2:1:1:InputOutput::ProhibitBarewordFileHandles:Bareword filehandle 'FH'\n"
            .to_vec(),
    ));
    server.test_install_mock_critic_runtime(runtime);
    server.test_bypass_perlcritic_command_check();

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_sev1.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_sev1.pl";

    let result = pull_diagnostics(&server, uri, "open FH, $path;\n");
    let diags = result["items"].as_array().cloned().unwrap_or_default();
    assert!(diags.iter().any(|d| {
        d["code"].as_str() == Some("InputOutput::ProhibitBarewordFileHandles")
            && d["severity"].as_u64() == Some(4)
    }));
}

// ── Test B ────────────────────────────────────────────────────────────────────

/// No subprocess must be invoked when perlcritic is disabled (the default).
#[test]
fn test_b_no_subprocess_invocation_when_perlcritic_disabled() {
    let server = LspServer::new();

    // Install a mock runtime that records calls.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    // perlcritic_enabled defaults to false; the early-return fires before any
    // command check or subprocess call.

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_disabled.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_disabled.pl";

    pull_diagnostics(&server, uri, "use strict;\nuse warnings;\n");

    assert_eq!(
        runtime.invocations().len(),
        0,
        "mock runtime must not be called when perlcritic is disabled"
    );
}

// ── Test C ────────────────────────────────────────────────────────────────────

/// When the `perlcritic` binary is absent from PATH, diagnostics are empty and
/// no file-local perlcritic diagnostics are emitted.
///
/// This test is skipped when perlcritic *is* installed because there is no
/// portable way to temporarily hide a binary from PATH in a single test.
#[test]
fn test_c_graceful_skip_when_perlcritic_not_installed() {
    // Only meaningful when perlcritic is NOT on the PATH.
    if which::which("perlcritic").is_ok() {
        return;
    }

    let server = LspServer::new();
    server.test_configure_perlcritic(true, 3, None);
    // Do NOT call test_bypass_perlcritic_command_check — let the guard fire.

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_not_installed.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_not_installed.pl";

    let result = pull_diagnostics(&server, uri, "use strict;\n");

    let diags = result["items"].as_array().cloned().unwrap_or_default();
    let perlcritic_diags: Vec<_> =
        diags.iter().filter(|d| d["code"].as_str().is_some_and(|c| c.contains("::"))).collect();

    assert_eq!(
        perlcritic_diags.len(),
        0,
        "No perlcritic diagnostics expected when binary is not installed; \
         got: {result}"
    );
    assert_eq!(
        runtime.invocations().len(),
        0,
        "Mock runtime must not be called when perlcritic binary is absent"
    );
}

#[test]
fn test_c1_missing_configured_profile_skips_subprocess_and_file_diagnostics() {
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let module_path = root.join("MyModule.pm");
    std::fs::write(&module_path, "package MyModule;\n1;\n").expect("write MyModule.pm");

    let missing_profile = root.join("does-not-exist.perlcriticrc");

    let server = LspServer::new();
    server.test_set_root_path(root);
    server.test_configure_perlcritic(true, 3, Some(missing_profile.to_string_lossy().to_string()));

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(
        b"MyModule.pm:2:1:3:Custom::ExternalOnly:only-from-external-runtime\n"
            .to_vec(),
    ));
    server.test_install_mock_critic_runtime(runtime.clone());
    server.test_bypass_perlcritic_command_check();

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    let result = pull_diagnostics(&server, &uri, "package MyModule;\n1;\n");
    let diags = result["items"].as_array().cloned().unwrap_or_default();
    let external_mock_diag = diags.iter().find(|d| d["code"].as_str() == Some("Custom::ExternalOnly"));

    assert!(
        external_mock_diag.is_none(),
        "No external perlcritic diagnostics expected when configured profile path is missing; got: {result}"
    );
    assert_eq!(
        runtime.invocations().len(),
        0,
        "perlcritic subprocess must not run when configured profile is missing"
    );
}

// ── Test D ────────────────────────────────────────────────────────────────────

/// `.perlcriticrc` walk-up: a config at the workspace root must be discovered
/// even when the file being analysed lives in a sub-directory.
///
/// Tree: `<root>/.perlcriticrc` and `<root>/lib/MyModule.pm`.
/// After opening `MyModule.pm`, the analyzer must be invoked with
/// `--profile=<root>/.perlcriticrc`.
#[test]
fn test_d_perlcriticrc_walkup_finds_workspace_root_config() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib/");

    let rc_path = root.join(".perlcriticrc");
    fs::write(&rc_path, "severity = 3\n").expect("write .perlcriticrc");

    let module_path = lib_dir.join("MyModule.pm");
    fs::write(&module_path, "package MyModule;\n1;\n").expect("write MyModule.pm");

    let server = LspServer::new();
    server.test_configure_perlcritic(true, 3, None);

    // Tell the server where the workspace root is so the walk-up stops there.
    server.test_set_root_path(root.clone());

    // Mock runtime records calls.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    server.test_bypass_perlcritic_command_check();

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();

    pull_diagnostics(&server, &uri, "package MyModule;\n1;\n");

    let invocations = runtime.invocations();
    assert_eq!(
        invocations.len(),
        1,
        "Mock runtime should be called exactly once; got: {invocations:?}"
    );

    let expected_profile = rc_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations[0].args.contains(&profile_arg),
        "perlcritic must be invoked with --profile pointing to the workspace root \
         .perlcriticrc; args: {:?}",
        invocations[0].args
    );
}

#[test]
fn test_e_empty_profile_falls_back_to_walkup_config() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib/");

    let rc_path = root.join(".perlcriticrc");
    fs::write(&rc_path, "severity = 3\n").expect("write .perlcriticrc");

    let module_path = lib_dir.join("MyModule.pm");
    fs::write(&module_path, "package MyModule;\n1;\n").expect("write MyModule.pm");

    let server = LspServer::new();
    server.test_configure_perlcritic(true, 3, Some(String::new()));
    server.test_set_root_path(root.clone());

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    server.test_bypass_perlcritic_command_check();

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();

    pull_diagnostics(&server, &uri, "package MyModule;\n1;\n");

    let invocations = runtime.invocations();
    assert_eq!(
        invocations.len(),
        1,
        "empty profile values should not suppress perlcritic execution; got: {invocations:?}"
    );

    let expected_profile = rc_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations[0].args.contains(&profile_arg),
        "empty profile should fall back to workspace walk-up .perlcriticrc; args: {:?}",
        invocations[0].args
    );
}
