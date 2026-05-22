//! LSP-side wiring tests for `RuntimeTuning` (PR 1 of the 0.15.1 Neovim latency lane).
//!
//! These tests verify that the `RuntimeTuning` config defined in
//! `perl-lsp-rs-core` is correctly threaded into `LspServer` and that the
//! debouncer interval and immediate-publication path honour it.

use perl_lsp::LspServer;
use perl_lsp_rs_core::runtime::tuning::{DiagnosticMode, RuntimeMode, RuntimeTuning};

#[test]
fn runtime_mode_normal_defaults_unchanged() {
    let server = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    let tuning = server.runtime_tuning();
    assert_eq!(tuning.runtime_mode, RuntimeMode::Normal);
    assert_eq!(tuning.diagnostic_mode, DiagnosticMode::Normal);
    assert_eq!(tuning.diagnostic_debounce_ms, 250);
    assert!(tuning.eager_workspace_indexing);
    assert!(tuning.file_watchers);
}

#[test]
fn runtime_mode_e2e_defaults() {
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());
    let tuning = server.runtime_tuning();
    assert_eq!(tuning.runtime_mode, RuntimeMode::E2e);
    assert_eq!(tuning.diagnostic_mode, DiagnosticMode::SyntaxOnly);
    assert_eq!(tuning.diagnostic_debounce_ms, 0);
    assert!(!tuning.eager_workspace_indexing);
    assert!(!tuning.file_watchers);
}

#[test]
fn diagnostic_debounce_zero_is_immediate() {
    // The server constructed with debounce_ms = 0 must report immediate semantics.
    // The wiring guarantee is that publish_diagnostics_debounced bypasses the
    // debouncer thread entirely so the "first useful answer" measurement
    // doesn't include the debounce window.
    let mut tuning = RuntimeTuning::normal_defaults();
    tuning.diagnostic_debounce_ms = 0;
    let server = LspServer::new_with_tuning(tuning);
    assert!(server.runtime_tuning().diagnostic_debounce_is_immediate());

    let e2e = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());
    assert!(e2e.runtime_tuning().diagnostic_debounce_is_immediate());

    let normal = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    assert!(!normal.runtime_tuning().diagnostic_debounce_is_immediate());
}

#[test]
fn server_keeps_tuning_across_lifecycle() {
    // Constructor-supplied tuning must survive any subsequent state changes.
    // (Tuning is intentionally immutable post-construction; this guards
    // against an accidental mut-aliased field down the line.)
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());
    let before = server.runtime_tuning();
    // Touching independent server state doesn't perturb tuning.
    let _ = server.is_initialized();
    let after = server.runtime_tuning();
    assert_eq!(before, after);
}
