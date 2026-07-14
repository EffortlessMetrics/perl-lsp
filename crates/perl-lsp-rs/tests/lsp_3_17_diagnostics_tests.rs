//! LSP 3.17 Diagnostics, Inlay Hints, Inline Values, and Moniker Contract Tests
//!
//! Tests for textDocument/diagnostic, workspace/diagnostic, textDocument/inlayHint,
//! textDocument/inlineValue, and textDocument/moniker.

mod support;

use perl_lsp::{JsonRpcRequest, LspServer};
use perl_lsp_rs_core::runtime::tuning::RuntimeTuning;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== DIAGNOSTICS PULL MODEL (3.17) ====================

#[test]
fn test_diagnostic_pull_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "$undefined")?;

    let response = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
    );

    if let Ok(report) = response {
        if !report.is_null() {
            assert!(report["kind"].is_string());
            if report["kind"] == "full" {
                assert!(report["items"].is_array());
            }
        }
    }
    Ok(())
}

#[test]
fn test_diagnostic_pull_missing_uri_returns_invalid_params_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let error = match harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": {},
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
    ) {
        Ok(value) => {
            return Err(format!(
                "missing textDocument.uri must return InvalidParams, got success: {value:?}"
            )
            .into());
        }
        Err(error) => error,
    };

    assert!(
        error.contains("-32602") && error.contains("textDocument.uri"),
        "missing textDocument.uri must surface InvalidParams with field context, got: {error}"
    );
    Ok(())
}

#[test]
fn test_diagnostic_pull_invalid_uri_returns_invalid_params_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let error = match harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": ":::not a uri:::" },
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
    ) {
        Ok(value) => {
            return Err(format!(
                "invalid textDocument.uri must return InvalidParams, got success: {value:?}"
            )
            .into());
        }
        Err(error) => error,
    };

    assert!(
        error.contains("-32602") && error.contains("Invalid URI"),
        "invalid textDocument.uri must surface InvalidParams with URI context, got: {error}"
    );
    Ok(())
}

#[test]
fn test_diagnostic_pull_syntax_only_open_document_3_17() -> TestResult {
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    let uri = "file:///syntax_only_pull.pl";
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "sub broken {\n"
            }
        })),
    });

    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
            method: "textDocument/diagnostic".into(),
            params: Some(json!({
                "textDocument": { "uri": uri },
                "identifier": "perl-lsp",
                "previousResultId": null
            })),
        })
        .ok_or("syntax-only diagnostic request must return a response")?;
    let report = response.result.ok_or("syntax-only diagnostic response must include result")?;

    assert_eq!(
        report.get("kind").and_then(|kind| kind.as_str()),
        Some("full"),
        "syntax-only pull diagnostics must return a full report, got: {report:?}"
    );
    let items = report
        .get("items")
        .and_then(|items| items.as_array())
        .ok_or("syntax-only pull diagnostics report must include items array")?;
    assert!(
        !items.is_empty(),
        "syntax-only pull diagnostics must report parse errors for an open broken document"
    );
    for item in items {
        assert_eq!(
            item.get("source").and_then(|source| source.as_str()),
            Some("perl-parser"),
            "syntax-only pull diagnostics must only emit parser diagnostics, got: {item:?}"
        );
    }
    Ok(())
}

#[test]
fn test_diagnostic_pull_syntax_only_clean_open_document_3_17() -> TestResult {
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    let uri = "file:///syntax_only_clean_pull.pl";
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $unused = 1;\n"
            }
        })),
    });

    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
            method: "textDocument/diagnostic".into(),
            params: Some(json!({
                "textDocument": { "uri": uri },
                "identifier": "perl-lsp",
                "previousResultId": null
            })),
        })
        .ok_or("syntax-only clean diagnostic request must return a response")?;
    let report =
        response.result.ok_or("syntax-only clean diagnostic response must include result")?;

    assert_eq!(
        report.get("kind").and_then(|kind| kind.as_str()),
        Some("full"),
        "syntax-only clean pull diagnostics must return a full report, got: {report:?}"
    );
    let items = report
        .get("items")
        .and_then(|items| items.as_array())
        .ok_or("syntax-only clean pull diagnostics report must include items array")?;
    assert!(
        items.is_empty(),
        "syntax-only pull diagnostics must suppress critic/semantic diagnostics for clean syntax, got: {items:?}"
    );
    Ok(())
}

#[test]
fn test_diagnostic_pull_syntax_only_unopened_uri_returns_empty_full_report_3_17() -> TestResult {
    let server = LspServer::new_with_tuning(RuntimeTuning::e2e_defaults());

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
            method: "textDocument/diagnostic".into(),
            params: Some(json!({
                "textDocument": { "uri": "file:///syntax_only_unopened_pull.pl" },
                "identifier": "perl-lsp",
                "previousResultId": null
            })),
        })
        .ok_or("syntax-only unopened diagnostic request must return a response")?;
    let report =
        response.result.ok_or("syntax-only unopened diagnostic response must include result")?;

    assert_eq!(
        report.get("kind").and_then(|kind| kind.as_str()),
        Some("full"),
        "syntax-only unopened pull diagnostics must return a full report, got: {report:?}"
    );
    let items = report
        .get("items")
        .and_then(|items| items.as_array())
        .ok_or("syntax-only unopened pull diagnostics report must include items array")?;
    assert!(
        items.is_empty(),
        "syntax-only unopened pull diagnostics must return an empty report, got: {items:?}"
    );
    Ok(())
}

#[test]
fn test_diagnostic_pull_unopened_uri_returns_empty_full_report_3_17() -> TestResult {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(2)),
            method: "textDocument/diagnostic".into(),
            params: Some(json!({
                "textDocument": { "uri": "file:///unopened_pull.pl" },
                "identifier": "perl-lsp",
                "previousResultId": null
            })),
        })
        .ok_or("unopened diagnostic request must return a response")?;
    let report = response.result.ok_or("unopened diagnostic response must include result")?;

    assert_eq!(
        report,
        json!({"kind": "full", "items": []}),
        "unopened pull diagnostics must return an empty full report"
    );
    Ok(())
}

#[test]
fn test_workspace_diagnostic_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "workspace/diagnostic",
        json!({
            "identifier": "perl-lsp",
            "previousResultIds": [],
            "workDoneToken": "diag-1",
            "partialResultToken": "partial-1"
        }),
    );

    if let Ok(report) = response {
        assert!(report.is_null() || report.is_object());
    }
    Ok(())
}

// ==================== INLAY HINTS (3.17) ====================

#[test]
fn test_inlay_hint_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "substr($str, 0, 5)")?;

    let response = harness.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 18 }
            }
        }),
    );

    if let Ok(hints) = response {
        assert!(hints.is_null() || hints.is_array());
    }
    Ok(())
}

// ==================== INLINE VALUES (3.17) ====================

#[test]
fn test_inline_value_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 42;\nprint $x;")?;

    let response = harness.request(
        "textDocument/inlineValue",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 1, "character": 9 }
            },
            "context": {
                "frameId": 1,
                "stoppedLocation": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": 9 }
                }
            }
        }),
    );

    if let Ok(values) = response {
        assert!(values.is_null() || values.is_array());
    }
    Ok(())
}

// ==================== MONIKER (3.16+) ====================

#[test]
fn test_moniker_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "package Foo::Bar;\nsub test {}")?;

    let response = harness.request(
        "textDocument/moniker",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 4 }
        }),
    );

    if let Ok(monikers) = response {
        assert!(monikers.is_null() || monikers.is_array());
    }
    Ok(())
}
