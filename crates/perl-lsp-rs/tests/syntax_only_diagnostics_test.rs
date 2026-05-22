//! Syntax-only diagnostic mode tests (PR 2 of the 0.15.1 Neovim latency lane).
//!
//! These tests pin the four claims from the spec:
//!
//! 1. `syntax_only_reports_parse_errors` — parse errors still surface.
//! 2. `syntax_only_clears_when_parse_errors_clear` — fixing the syntax
//!    clears diagnostics (empty publishDiagnostics or empty pull report).
//! 3. `syntax_only_skips_critic_dead_code_and_module_resolution` — the
//!    full pipeline's dead-code / native-critic / use-Module-not-found
//!    diagnostics are suppressed.
//! 4. `pull_diagnostics_respect_syntax_only_mode` — `textDocument/diagnostic`
//!    honours the same gate.
//!
//! Run with:
//!
//!     RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!         --features expose_lsp_test_api \
//!         --test syntax_only_diagnostics_test -- --test-threads=2

#![cfg(feature = "expose_lsp_test_api")]

use perl_lsp::LspServer;
use perl_lsp_rs_core::runtime::tuning::RuntimeTuning;
use serde_json::Value;
use serde_json::json;

fn syntax_only_server() -> LspServer {
    let mut tuning = RuntimeTuning::normal_defaults();
    tuning.diagnostic_mode = perl_lsp_rs_core::runtime::tuning::DiagnosticMode::SyntaxOnly;
    LspServer::new_with_tuning(tuning)
}

fn pull_request_for(uri: &str) -> Option<Value> {
    Some(json!({
        "textDocument": { "uri": uri },
    }))
}

fn pull_items(result: Option<Value>) -> Vec<Value> {
    let report = result.expect("pull diagnostic must return a result");
    report
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .expect("pull diagnostic result must include an items array")
}

#[test]
fn syntax_only_reports_parse_errors() {
    let server = syntax_only_server();
    // Missing closing `}` is a clear parse error.
    let bad = "sub broken {\n";
    server.test_apply_did_open("file:///broken.pl", bad, 1);

    let report = server
        .test_handle_document_diagnostic(pull_request_for("file:///broken.pl"))
        .expect("pull diagnostic must succeed");
    let items = pull_items(report);

    assert!(!items.is_empty(), "syntax-only mode must report parse errors; got {items:?}");
    for item in &items {
        let source = item.get("source").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(
            source, "perl-parser",
            "syntax-only must only emit parse errors; saw source={source}"
        );
        let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            code.starts_with("PL") || code == "parse-error",
            "syntax-only diagnostic must use parse-error code; got {code}"
        );
    }
}

#[test]
fn syntax_only_clears_when_parse_errors_clear() {
    let server = syntax_only_server();
    server.test_apply_did_open("file:///c.pl", "sub broken {\n", 1);
    let bad = pull_items(
        server.test_handle_document_diagnostic(pull_request_for("file:///c.pl")).unwrap(),
    );
    assert!(!bad.is_empty(), "broken parse must report errors");

    // Now fix the file: parse succeeds, parse_errors should be empty.
    server.test_apply_did_change("file:///c.pl", "sub broken {}\n", 2);
    let cleared = pull_items(
        server.test_handle_document_diagnostic(pull_request_for("file:///c.pl")).unwrap(),
    );
    assert!(
        cleared.is_empty(),
        "syntax-only mode must publish an empty diagnostic list after a clean parse; got {cleared:?}"
    );
}

#[test]
fn syntax_only_skips_critic_dead_code_and_module_resolution() {
    // This buffer normally produces:
    //   - native critic (no `use strict`/`use warnings`)
    //   - dead-code (unused $x)
    //   - module-resolution (cannot resolve `NoSuchModule`)
    // Under syntax-only mode every one of those must be suppressed.
    let server = syntax_only_server();
    let src = "use NoSuchModule::Definitely::Missing;\nmy $x = 1;\n";
    server.test_apply_did_open("file:///clean.pl", src, 1);

    let items = pull_items(
        server.test_handle_document_diagnostic(pull_request_for("file:///clean.pl")).unwrap(),
    );

    for item in &items {
        let source = item.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
        assert_ne!(
            source, "perl-lsp-critic",
            "syntax-only must suppress critic diagnostics; got {code}: {item:?}"
        );
        assert!(
            !code.starts_with("dead-code"),
            "syntax-only must suppress dead-code diagnostics; got {code}"
        );
        assert!(
            !code.contains("module") && !code.contains("Module"),
            "syntax-only must suppress module-resolution diagnostics; got {code}"
        );
    }

    // Same source under the default (Normal) diagnostic mode produces a
    // non-empty diagnostic list (sanity check that the test source actually
    // triggers the full pipeline).
    let full = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    full.test_apply_did_open("file:///clean.pl", src, 1);
    let full_items = pull_items(
        full.test_handle_document_diagnostic(pull_request_for("file:///clean.pl")).unwrap(),
    );
    assert!(
        full_items.len() > items.len(),
        "Normal mode must produce strictly more diagnostics than syntax-only on the same source.\n\
         syntax-only: {items:?}\nnormal: {full_items:?}"
    );
}

#[test]
fn pull_diagnostics_respect_syntax_only_mode() {
    // textDocument/diagnostic for a syntactically clean buffer with critic-
    // worthy code must return an empty items list under syntax-only mode.
    let server = syntax_only_server();
    let src = "my $unused_var = 1;\n";
    server.test_apply_did_open("file:///pull.pl", src, 1);

    let report = server
        .test_handle_document_diagnostic(pull_request_for("file:///pull.pl"))
        .expect("pull diagnostic must succeed");
    assert_eq!(report.as_ref().and_then(|v| v.get("kind")).and_then(|k| k.as_str()), Some("full"));
    let items = pull_items(report);
    assert!(
        items.is_empty(),
        "syntax-only pull diagnostic must produce an empty items list for a syntactically clean source; got {items:?}"
    );
}

#[test]
fn syntax_only_normal_mode_unchanged() {
    // Regression guard: default Normal mode still produces non-empty
    // diagnostics for the same critic-heavy source.
    let server = LspServer::new_with_tuning(RuntimeTuning::normal_defaults());
    let src = "my $unused = 1;\n";
    server.test_apply_did_open("file:///n.pl", src, 1);
    let items = pull_items(
        server.test_handle_document_diagnostic(pull_request_for("file:///n.pl")).unwrap(),
    );
    assert!(
        !items.is_empty(),
        "Normal mode must continue to produce diagnostics for unused vars / missing strict"
    );
}
