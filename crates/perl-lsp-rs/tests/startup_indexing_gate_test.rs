//! E2E startup indexing gate tests (PR 3 of the 0.15.1 Neovim latency lane).
//!
//! These tests pin the three claims from the spec:
//!
//! 1. `normal_mode_starts_workspace_indexing` - default editor sessions
//!    still index eagerly on `initialized`.
//! 2. `e2e_mode_does_not_start_workspace_indexing` - e2e harnesses skip
//!    the scan so latency tests don't pay for indexing.
//! 3. `workspace_symbol_normal_mode_unchanged` - predicate semantics for
//!    normal mode are stable and not accidentally inverted.
//!
//! Run with:
//!
//!     RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!         --features expose_lsp_test_api \
//!         --test startup_indexing_gate_test -- --test-threads=2

#![cfg(feature = "expose_lsp_test_api")]

use perl_lsp::LspServer;
use perl_lsp_rs_core::runtime::tuning::{RuntimeMode, RuntimeTuning};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn initialize_with_workspace(server: &LspServer, root_uri: &str) -> TestResult {
    let params = json!({
        "processId": null,
        "rootUri": root_uri,
        "workspaceFolders": [{
            "uri": root_uri,
            "name": "test",
        }],
        "capabilities": {},
    });
    server
        .test_handle_initialize_dispatch(Some(params))
        .map_err(|err| std::io::Error::other(format!("initialize failed: {err:?}")))?;
    server
        .test_handle_initialized_dispatch()
        .map_err(|err| std::io::Error::other(format!("initialized failed: {err:?}")))?;
    Ok(())
}

#[test]
fn normal_mode_starts_workspace_indexing() -> TestResult {
    let server = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    assert!(
        server.should_start_workspace_indexing(),
        "Normal mode must allow eager workspace indexing"
    );

    let temp = tempfile::tempdir()?;
    let root_uri = format!("file://{}", temp.path().display());

    let before = server.workspace_indexing_invocation_count();
    initialize_with_workspace(&server, &root_uri)?;
    let after = server.workspace_indexing_invocation_count();

    assert!(
        after > before,
        "Normal mode must invoke start_workspace_indexing on `initialized` (before={before} after={after})"
    );
    Ok(())
}

#[test]
fn e2e_mode_does_not_start_workspace_indexing() -> TestResult {
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());
    assert!(
        !server.should_start_workspace_indexing(),
        "E2E defaults must gate off eager workspace indexing"
    );

    let temp = tempfile::tempdir()?;
    let root_uri = format!("file://{}", temp.path().display());

    let before = server.workspace_indexing_invocation_count();
    initialize_with_workspace(&server, &root_uri)?;
    let after = server.workspace_indexing_invocation_count();

    assert_eq!(
        after, before,
        "E2E mode must NOT invoke start_workspace_indexing on `initialized` (before={before} after={after})"
    );
    Ok(())
}

#[test]
fn workspace_symbol_normal_mode_unchanged() -> TestResult {
    // Regression: in normal mode, the runtime mode field, indexing gate,
    // and eager flag stay consistent; nothing accidentally inverted.
    let server = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    let tuning = server.runtime_tuning();
    assert_eq!(tuning.runtime_mode, RuntimeMode::Normal);
    assert!(tuning.eager_workspace_indexing);
    assert!(server.should_start_workspace_indexing());
    Ok(())
}

#[test]
fn explicit_eager_flag_overrides_e2e_default() -> TestResult {
    // CLI override: `--runtime-mode e2e --eager-workspace-indexing=true`
    // should still index.
    let mut tuning = RuntimeTuning::e2e_defaults();
    tuning.eager_workspace_indexing = true;
    let server = LspServer::new_with_tuning(tuning);
    assert!(
        server.should_start_workspace_indexing(),
        "Explicit eager flag must win even under e2e mode"
    );
    Ok(())
}

#[test]
fn explicit_disable_in_normal_mode_skips_indexing() -> TestResult {
    // CLI override: `--eager-workspace-indexing=false` under normal mode
    // should still gate off; symmetric to the e2e case.
    let mut tuning = RuntimeTuning::normal_defaults();
    tuning.eager_workspace_indexing = false;
    let server = LspServer::new_with_tuning(tuning);
    assert!(!server.should_start_workspace_indexing());

    let temp = tempfile::tempdir()?;
    let root_uri = format!("file://{}", temp.path().display());
    let before = server.workspace_indexing_invocation_count();
    initialize_with_workspace(&server, &root_uri)?;
    let after = server.workspace_indexing_invocation_count();
    assert_eq!(after, before, "explicit disable must skip indexing even in normal mode");
    Ok(())
}
