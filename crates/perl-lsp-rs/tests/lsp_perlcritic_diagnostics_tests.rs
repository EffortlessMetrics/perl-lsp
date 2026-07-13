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
// Tests are permitted to use `.expect()`/`.expect_err()` on Result/Option per
// the repo's coding standards (unlike production code, where they are banned).
#![allow(clippy::expect_used)]

use perl_lsp::LspServer;
use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use serde_json::json;
use std::sync::Arc;

/// Open `uri` with `text` via `didOpen`, then issue a pull-diagnostics request
/// and return the result.
fn pull_diagnostics(server: &LspServer, uri: &str, text: &str) -> serde_json::Value {
    if let Ok(parsed_uri) = url::Url::parse(uri) {
        if let Ok(path) = parsed_uri.to_file_path() {
            let _ = std::fs::write(path, text);
        }
    }

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
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 3, None);

    // Install a mock runtime returning one severity-3 violation for the file.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mock_line =
        b"test.pl:5:1:3:TestingAndDebugging::RequireUseStrict:Code does not use strict\n";
    runtime.add_response(MockResponse::success(mock_line.to_vec()));
    runtime.add_response(MockResponse::success(mock_line.to_vec()));
    server.test_install_mock_critic_runtime(runtime);
    server.test_bypass_perlcritic_command_check();

    // Use a file:// URI that resolves to a real-looking path.
    #[cfg(windows)]
    let uri = "file:///C:/tmp/test.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test.pl";

    let result =
        pull_diagnostics(&server, uri, "# line 1\n# line 2\n# line 3\n# line 4\nprint 'hello';\n");

    // There must be at least one diagnostic with code
    // "TestingAndDebugging::RequireUseStrict", severity 2 (Warning),
    // perlcritic source attribution, and fixable=true metadata.
    //
    // The pull-diagnostics response has the shape:
    //   { "kind": "full", "items": [ { "code": "...", "severity": N, ... } ], "resultId": "..." }
    let diags = result["items"].as_array().cloned().unwrap_or_default();

    let found = diags.iter().any(|d| {
        d["code"].as_str() == Some("TestingAndDebugging::RequireUseStrict")
            && d["severity"].as_u64() == Some(2)
            && d["data"]["fixable"].as_bool() == Some(true)
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
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 5, None);

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(
        b"test.pl:2:1:5:InputOutput::RequireThreeArgOpen:Use three-arg open\n".to_vec(),
    ));
    runtime.add_response(MockResponse::success(
        b"test.pl:2:1:5:InputOutput::RequireThreeArgOpen:Use three-arg open\n".to_vec(),
    ));
    server.test_install_mock_critic_runtime(runtime);
    server.test_bypass_perlcritic_command_check();

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_sev5.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_sev5.pl";

    let result = pull_diagnostics(&server, uri, "# line 1\nopen FH, $path;\n");
    let diags = result["items"].as_array().cloned().unwrap_or_default();
    assert!(
        diags.iter().any(|d| {
            d["code"].as_str() == Some("InputOutput::RequireThreeArgOpen")
                && d["severity"].as_u64() == Some(1)
        }),
        "expected severity-5 external diagnostic; got: {result}"
    );
}

#[test]
fn test_a2_severity_one_maps_to_hint() {
    let server = LspServer::new();
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 1, None);

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(
        b"test.pl:2:1:1:InputOutput::ProhibitBarewordFileHandles:Bareword filehandle 'FH'\n"
            .to_vec(),
    ));
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

    let result = pull_diagnostics(&server, uri, "# line 1\nopen FH, $path;\n");
    let diags = result["items"].as_array().cloned().unwrap_or_default();
    assert!(
        diags.iter().any(|d| {
            d["code"].as_str() == Some("InputOutput::ProhibitBarewordFileHandles")
                && d["severity"].as_u64() == Some(4)
        }),
        "expected severity-1 external diagnostic; got: {result}"
    );
}

// ── Test B ────────────────────────────────────────────────────────────────────

/// No subprocess must be invoked for default native critic diagnostics.
#[test]
fn test_b_no_subprocess_invocation_for_default_native_critic() {
    let server = LspServer::new();

    // Install a mock runtime that records calls.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    // The default critic engine is native, so the external Perl::Critic
    // subprocess path must not run.

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_disabled.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_disabled.pl";

    pull_diagnostics(&server, uri, "use strict;\nuse warnings;\n");

    assert_eq!(
        runtime.invocations().len(),
        0,
        "mock runtime must not be called for default native critic diagnostics"
    );
}

// ── Test C ────────────────────────────────────────────────────────────────────

/// When the `perlcritic` binary is absent from PATH, diagnostics are empty and
/// no subprocess runs.
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
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 3, None);
    // Do NOT call test_bypass_perlcritic_command_check — let the guard fire.

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    #[cfg(windows)]
    let uri = "file:///C:/tmp/test_not_installed.pl";
    #[cfg(not(windows))]
    let uri = "file:///tmp/test_not_installed.pl";

    pull_diagnostics(&server, uri, "use strict;\n");

    // Legacy mode may still emit built-in policy diagnostics. The subprocess
    // guard is the contract this test owns: no external invocation occurs.
    assert_eq!(
        runtime.invocations().len(),
        0,
        "Mock runtime must not be called when perlcritic binary is absent"
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
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
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
    assert!(!invocations.is_empty(), "mock runtime should be called; got: {invocations:?}");

    let expected_profile = rc_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations.iter().any(|invocation| invocation.args.contains(&profile_arg)),
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
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 3, Some(String::new()));
    server.test_set_root_path(root.clone());

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    server.test_bypass_perlcritic_command_check();

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();

    pull_diagnostics(&server, &uri, "package MyModule;\n1;\n");

    let invocations = runtime.invocations();
    assert!(
        !invocations.is_empty(),
        "empty profile values should not suppress perlcritic execution; got: {invocations:?}"
    );

    let expected_profile = rc_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations.iter().any(|invocation| invocation.args.contains(&profile_arg)),
        "empty profile should fall back to workspace walk-up .perlcriticrc; args: {:?}",
        invocations[0].args
    );
}

#[test]
fn test_f_missing_configured_profile_skips_subprocess_and_diagnostics() {
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let missing_profile = root.join("missing.perlcriticrc");
    let module_path = root.join("NoProfile.pm");
    std::fs::write(&module_path, "package NoProfile;\n1;\n").expect("write module");

    let server = LspServer::new();
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 3, Some(missing_profile.to_string_lossy().to_string()));
    server.test_set_root_path(root);

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());
    server.test_bypass_perlcritic_command_check();

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    pull_diagnostics(&server, &uri, "package NoProfile;\n1;\n");

    // Legacy mode may still emit built-in policy diagnostics. A missing
    // configured profile must prevent the external subprocess from running.
    assert_eq!(
        runtime.invocations().len(),
        0,
        "subprocess should not run when configured profile path does not exist"
    );
}

#[test]
fn test_f2_relative_configured_profile_resolves_from_workspace_root() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let cfg_dir = root.join("config");
    fs::create_dir_all(&cfg_dir).expect("create config/");

    let profile_path = cfg_dir.join("perlcriticrc");
    fs::write(&profile_path, "severity = 3\n").expect("write profile");

    let module_path = root.join("RelativeProfile.pm");
    fs::write(&module_path, "package RelativeProfile;\n1;\n").expect("write module");

    let server = LspServer::new();
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_set_root_path(root.clone());
    server.test_configure_perlcritic(true, 3, Some("config/perlcriticrc".to_string()));
    server.test_bypass_perlcritic_command_check();

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    pull_diagnostics(&server, &uri, "package RelativeProfile;\n1;\n");

    let invocations = runtime.invocations();
    assert!(
        !invocations.is_empty(),
        "expected perlcritic subprocess invocation; got: {invocations:?}"
    );

    let expected_profile = profile_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations.last().is_some_and(|invocation| invocation.args.contains(&profile_arg)),
        "relative configured profile should resolve from workspace root; args: {:?}",
        invocations.last().map(|invocation| &invocation.args)
    );
}

#[test]
fn test_f3_walkup_finds_perlcriticrc_without_dot_prefix() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib/");

    let rc_path = root.join("perlcriticrc");
    fs::write(&rc_path, "severity = 3\n").expect("write perlcriticrc");

    let module_path = lib_dir.join("NoDotRc.pm");
    fs::write(&module_path, "package NoDotRc;\n1;\n").expect("write module");

    let server = LspServer::new();
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(true, 3, None);
    server.test_set_root_path(root.clone());
    server.test_bypass_perlcritic_command_check();

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    pull_diagnostics(&server, &uri, "package NoDotRc;\n1;\n");

    let invocations = runtime.invocations();
    assert!(
        !invocations.is_empty(),
        "expected perlcritic subprocess invocation; got: {invocations:?}"
    );

    let expected_profile = rc_path.to_string_lossy().to_string();
    let profile_arg = format!("--profile={expected_profile}");
    assert!(
        invocations.last().is_some_and(|invocation| invocation.args.contains(&profile_arg)),
        "walk-up should discover workspace perlcriticrc without dot; args: {:?}",
        invocations.last().map(|invocation| &invocation.args)
    );
}

#[test]
fn test_g_did_change_configuration_resets_pull_perlcritic_analyzer() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let profile_a = root.join("profile-a.perlcriticrc");
    let profile_b = root.join("profile-b.perlcriticrc");
    fs::write(&profile_a, "severity = 3\n").expect("write profile-a");
    fs::write(&profile_b, "severity = 3\n").expect("write profile-b");

    let module_path = root.join("ConfigSwitch.pm");
    fs::write(&module_path, "package ConfigSwitch;\n1;\n").expect("write module");

    let server = LspServer::new();
    server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
    server.test_configure_perlcritic(false, 3, None);
    server.test_set_root_path(root);
    server.test_bypass_perlcritic_command_check();

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    server
        .test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "package ConfigSwitch;\n1;\n"
            }
        })))
        .expect("didOpen should succeed");

    server.test_handle_did_change_configuration(Some(json!({
        "settings": {
            "perl": {
                "perlcritic": {
                    "enabled": true,
                    "severity": 3,
                    "profile": profile_a.to_string_lossy().to_string()
                }
            }
        }
    })));
    must(server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    }))));

    server.test_handle_did_change_configuration(Some(json!({
        "settings": {
            "perl": {
                "perlcritic": {
                    "enabled": true,
                    "severity": 3,
                    "profile": profile_b.to_string_lossy().to_string()
                }
            }
        }
    })));
    must(server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    }))));

    let invocations = runtime.invocations();
    assert_eq!(
        invocations.len(),
        2,
        "expected two pull perlcritic invocations; got: {invocations:?}"
    );
    assert!(
        invocations
            .iter()
            .any(|call| call.args.contains(&format!("--profile={}", profile_a.to_string_lossy()))),
        "at least one invocation should use profile-a; invocations: {invocations:?}"
    );
    let last = invocations.last().expect("at least one invocation recorded");
    assert!(
        last.args.contains(&format!("--profile={}", profile_b.to_string_lossy())),
        "last invocation should use profile-b after didChangeConfiguration; args: {:?}",
        last.args
    );
}

/// A severity change delivered via the native `critic.severity` key must also
/// reset the shared analyzer. The native `critic.*` keys fold into the same
/// `perlcritic_severity` config field, so the reset predicate must detect the
/// change regardless of which key carried it (regression for the #3308 review:
/// the old predicate re-parsed only `perlcritic.severity` and missed native-key
/// severity updates, leaving a warmed analyzer stale).
#[test]
fn test_h_native_critic_severity_change_resets_analyzer() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let profile = root.join("profile.perlcriticrc");
    fs::write(&profile, "severity = 3\n").expect("write profile");

    let module_path = root.join("SeveritySwitch.pm");
    fs::write(&module_path, "package SeveritySwitch;\n1;\n").expect("write module");

    let server = LspServer::new();
    server.test_set_root_path(root);
    server.test_bypass_perlcritic_command_check();

    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"".to_vec()));
    runtime.add_response(MockResponse::success(b"".to_vec()));
    server.test_install_mock_critic_runtime(runtime.clone());

    let uri = url::Url::from_file_path(&module_path).expect("file url").to_string();
    server
        .test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "package SeveritySwitch;\n1;\n"
            }
        })))
        .expect("didOpen should succeed");

    // Establish the analyzer at severity 3 via the legacy engine + profile.
    server.test_handle_did_change_configuration(Some(json!({
        "settings": {
            "perl": {
                "critic": { "engine": "legacy" },
                "perlcritic": {
                    "enabled": true,
                    "severity": 3,
                    "profile": profile.to_string_lossy().to_string()
                }
            }
        }
    })));
    must(server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    }))));

    // Change severity ONLY via the native `critic.severity` key. This must reset
    // the analyzer and force a fresh invocation on the next diagnostic cycle.
    server.test_handle_did_change_configuration(Some(json!({
        "settings": { "perl": { "critic": { "severity": 4 } } }
    })));
    must(server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    }))));

    let invocations = runtime.invocations();
    assert_eq!(
        invocations.len(),
        2,
        "native critic.severity change must reset the analyzer and re-invoke; got: {invocations:?}"
    );
}
