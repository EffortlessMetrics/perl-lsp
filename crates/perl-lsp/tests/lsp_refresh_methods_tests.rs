//! Tests for LSP server→client refresh requests
//!
//! These tests verify that the server correctly sends refresh requests to the
//! client for various features (code lenses, semantic tokens, diagnostics, etc.)

use perl_lsp::LspServer;

/// Test that refresh requests succeed when client doesn't support them (no-op behavior)
#[test]
fn lsp_refresh_code_lens_not_sent_without_support() {
    let server = LspServer::new();
    // Default client capabilities don't support refresh - should be no-op
    assert!(server.request_code_lens_refresh().is_ok());
}

#[test]
fn lsp_refresh_semantic_tokens_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_semantic_tokens_refresh().is_ok());
}

#[test]
fn lsp_refresh_inlay_hint_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_inlay_hint_refresh().is_ok());
}

#[test]
fn lsp_refresh_inline_value_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_inline_value_refresh().is_ok());
}

#[test]
fn lsp_refresh_diagnostic_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_diagnostic_refresh().is_ok());
}

#[test]
fn lsp_refresh_folding_range_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_folding_range_refresh().is_ok());
}
