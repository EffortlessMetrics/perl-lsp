use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const TRACE_URI: &str = "file:///workspace/lib/Trace/Live.pm";
const TRACE_DOC: &str = r#"package Trace::Live;
use strict;
use warnings;

sub target {
    my $value = 1;
    return $value;
}

my $ready = 1;
my $call = target();
my $prefix = $re;
"#;

const WORKSPACE_SYMBOL_URI: &str = "file:///workspace/lib/Trace/Symbols.pm";
const WORKSPACE_SYMBOL_DOC: &str = r#"package Trace::Symbols;
use strict;
use warnings;

sub greet {
    return "hello";
}

1;
"#;

const GENERATED_WORKSPACE_SYMBOL_URI: &str = "file:///workspace/lib/Trace/GeneratedSymbols.pm";
const GENERATED_WORKSPACE_SYMBOL_DOC: &str = r#"package Trace::GeneratedSymbols;
use Moo;

has display_name => (is => 'rw');

1;
"#;
const NO_SUB_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/NoSub.pm";
const NO_SUB_SEMANTIC_TOKEN_DOC: &str = r#"package Trace::NoSub;
1;
"#;

fn create_server() -> LspServer {
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn request(id: i64, method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        method: method.to_string(),
        params,
    }
}

fn response_result(
    response: Option<JsonRpcResponse>,
    context: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = response.ok_or_else(|| format!("{context}: missing JSON-RPC response"))?;
    if let Some(error) = response.error {
        return Err(format!("{context}: JSON-RPC error {}: {}", error.code, error.message).into());
    }
    Ok(response.result.unwrap_or(Value::Null))
}

fn initialize(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    let result = response_result(
        server.handle_request(request(1, "initialize", Some(json!({})))),
        "initialize",
    )?;
    if result.is_null() {
        return Err("initialize returned null result".into());
    }
    Ok(())
}

fn open_trace_document(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TRACE_URI,
            "text": TRACE_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_workspace_symbol_document(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": WORKSPACE_SYMBOL_URI,
            "text": WORKSPACE_SYMBOL_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_generated_workspace_symbol_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": GENERATED_WORKSPACE_SYMBOL_URI,
            "text": GENERATED_WORKSPACE_SYMBOL_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_no_sub_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": NO_SUB_SEMANTIC_TOKEN_URI,
            "text": NO_SUB_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn position_after(needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in TRACE_DOC.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character + needle.len())?;
            return Ok((line, character));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn position_on(needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in TRACE_DOC.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character)?;
            return Ok((line, character));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn explain_provider_decision(
    server: &LspServer,
    provider: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = server
        .handle_execute_command(Some(json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": provider
            }]
        })))?
        .ok_or("missing explain-provider-decision response")?;
    Ok(response)
}

fn request_receipt<'a>(
    explanation: &'a Value,
    provider: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    assert_eq!(explanation.get("provider").and_then(Value::as_str), Some(provider));
    explanation
        .get("request_receipt")
        .ok_or_else(|| format!("missing {provider} request_receipt").into())
}

fn assert_live_trace(receipt: &Value, provider: &str, action: &str) {
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some(provider));
    assert_eq!(receipt.get("provider_action").and_then(Value::as_str), Some(action));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("provider_runtime"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert!(receipt.get("fallback").and_then(Value::as_str).is_some());
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("not_proven_by_dispatch_trace")
    );
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).is_some(),
        "live trace must include a result count: {receipt}"
    );
}

#[test]
fn live_completion_request_keeps_provider_specific_trace() -> Result<(), Box<dyn std::error::Error>>
{
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;
    let (line, character) = position_after("$re")?;

    response_result(
        server.handle_request(request(
            2,
            "textDocument/completion",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "completion",
    )?;

    let explanation = explain_provider_decision(&server, "completion")?;
    let receipt = request_receipt(&explanation, "completion")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("completion"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/completion")
    );
    assert_eq!(
        receipt.get("claim_boundary").and_then(Value::as_str),
        Some(
            "records existing completion response only; no new completion candidates or ranking changes"
        )
    );
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        None,
        "dispatcher-level trace must not overwrite completion's provider-specific receipt: {receipt}"
    );
    assert!(receipt.get("item_count").and_then(Value::as_u64).is_some());
    Ok(())
}

#[test]
fn live_hover_request_persists_provider_trace() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;
    let (line, character) = position_on("target {")?;

    response_result(
        server.handle_request(request(
            4,
            "textDocument/hover",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "hover",
    )?;

    let explanation = explain_provider_decision(&server, "hover")?;
    let receipt = request_receipt(&explanation, "hover")?;
    assert_live_trace(receipt, "hover", "textDocument/hover");
    Ok(())
}

#[test]
fn live_diagnostic_request_persists_provider_trace() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            4,
            "textDocument/diagnostic",
            Some(json!({
                "textDocument": {"uri": TRACE_URI}
            })),
        )),
        "diagnostic",
    )?;

    let explanation = explain_provider_decision(&server, "diagnostics")?;
    let receipt = request_receipt(&explanation, "diagnostics")?;
    assert_live_trace(receipt, "diagnostics", "textDocument/diagnostic");
    assert_eq!(receipt.get("live_provider_result_kind").and_then(Value::as_str), Some("items"));
    Ok(())
}

#[test]
fn live_symbol_requests_persist_provider_traces() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/documentSymbol",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1}
            })),
        )),
        "document symbols",
    )?;
    let explanation = explain_provider_decision(&server, "document_symbols")?;
    let receipt = request_receipt(&explanation, "document_symbols")?;
    assert_live_trace(receipt, "document_symbols", "textDocument/documentSymbol");

    open_workspace_symbol_document(&server)?;
    response_result(
        server.handle_request(request(6, "workspace/symbol", Some(json!({"query": "greet"})))),
        "workspace symbols",
    )?;
    let explanation = explain_provider_decision(&server, "workspace_symbols")?;
    let receipt = request_receipt(&explanation, "workspace_symbols")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("workspace_symbols"));
    assert_eq!(receipt.get("provider_action").and_then(Value::as_str), Some("workspace/symbol"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    match receipt.get("decision").and_then(Value::as_str) {
        Some("acted") => {
            assert_eq!(
                receipt.get("reason").and_then(Value::as_str),
                Some("source_backed_high_confidence")
            );
            assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
            assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
            assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
            assert_eq!(
                receipt.get("source_backed_state").and_then(Value::as_str),
                Some("ready_workspace_index")
            );
            assert_eq!(
                receipt.get("live_cutover").and_then(Value::as_str),
                Some("partial_live_source_backed")
            );
        }
        Some("fallback") => {
            assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("partial_index"));
            assert_eq!(
                receipt.get("fact_source").and_then(Value::as_str),
                Some("legacy_workspace")
            );
            assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
            assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
            assert_eq!(
                receipt.get("source_backed_state").and_then(Value::as_str),
                Some("partial_index_not_full_workspace")
            );
            assert_eq!(receipt.get("live_cutover").and_then(Value::as_str), Some("fallback_only"));
        }
        other => return Err(format!("unexpected workspace-symbol decision: {other:?}").into()),
    }
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).is_some(),
        "workspace symbol trace must include a result count: {receipt}"
    );
    Ok(())
}

#[test]
fn live_workspace_symbol_generated_pilot_persists_labeled_provider_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_generated_workspace_symbol_document(&server)?;

    response_result(
        server.handle_request(request(
            6,
            "workspace/symbol",
            Some(json!({"query": "display_name"})),
        )),
        "workspace generated symbols",
    )?;

    let explanation = explain_provider_decision(&server, "workspace_symbols")?;
    let receipt = request_receipt(&explanation, "workspace_symbols")?;
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_generated_label_pilot")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("framework_adapter"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("ready_workspace_index_generated_label_pilot")
    );
    assert_eq!(
        receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed_generated_pilot")
    );
    assert_eq!(receipt.get("generated_pilot_count").and_then(Value::as_u64), Some(1));
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("not exact generated method bodies"),
        "generated pilot trace must avoid exact-location overclaim: {boundary}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_persists_compiler_token_live_slice_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1}
            })),
        )),
        "semantic tokens",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/semanticTokens/full")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_subroutine_declaration_live_token_match")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(
        receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed_compiler_token")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("subroutine_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("function"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).unwrap_or(0) > 0,
        "semantic-token live slice must include a live token count: {receipt}"
    );
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary")
            && boundary.contains("low-confidence"),
        "semantic-token live slice must preserve blocked boundaries: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message
                .contains("source-backed compiler subroutine-declaration live slice")),
        "explanation must surface the live-slice request detail: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_falls_back_without_compiler_token_slice()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_no_sub_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": NO_SUB_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens without compiler slice",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/semanticTokens/full")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("no_compiler_token_class"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("parser_syntax"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("compiler_token_live_slice_not_proven")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("legacy_provider"));
    assert_eq!(receipt.get("live_cutover").and_then(Value::as_str), Some("fallback_only"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));
    Ok(())
}
