//! Snapshot tests for provider decision explanation payloads.
//!
//! These tests lock the user-facing `perl.explainProviderDecision` response
//! shape so bug-report payload changes stay intentional.

use insta::assert_yaml_snapshot;
use perl_lsp::{JsonRpcId, JsonRpcRequest, LspServer};
use serde_json::{Value, json};

fn setup_server() -> LspServer {
    let server = LspServer::new();

    notify(
        &server,
        "initialize",
        Some(json!({
            "processId": null,
            "rootPath": null,
            "capabilities": {}
        })),
        Some(JsonRpcId::Integer(1)),
    );
    notify(&server, "initialized", Some(json!({})), None);

    server
}

fn notify(server: &LspServer, method: &str, params: Option<Value>, id: Option<JsonRpcId>) {
    let request =
        JsonRpcRequest { _jsonrpc: "2.0".to_string(), method: method.to_string(), params, id };
    let _ = server.handle_request(request);
}

fn request(
    server: &LspServer,
    method: &str,
    params: Value,
    id: i64,
) -> Result<Value, Box<dyn std::error::Error>> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params: Some(params),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((id) as i64)),
    };
    let response =
        server.handle_request(request).ok_or("expected JSON-RPC response from server")?;
    if let Some(error) = response.error {
        return Err(format!("request {method} returned error: {error:?}").into());
    }
    response.result.ok_or_else(|| format!("request {method} returned no result").into())
}

fn execute_command(
    server: &LspServer,
    command: &str,
    arguments: Value,
    id: i64,
) -> Result<Value, Box<dyn std::error::Error>> {
    request(
        server,
        "workspace/executeCommand",
        json!({
            "command": command,
            "arguments": arguments
        }),
        id,
    )
}

fn seed_completion_trace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///workspace/lib/Trace.pm";
    notify(
        server,
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "package Trace;\nsub helper { return 1; }\nhel\n"
            }
        })),
        None,
    );
    let _ = request(
        server,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 2, "character": 3 },
            "context": { "triggerKind": 1 }
        }),
        2,
    )?;
    Ok(())
}

fn scrub_version(mut value: Value) -> Value {
    if let Some(payload) = value.get_mut("copyable_payload").and_then(Value::as_object_mut) {
        if payload.contains_key("perl_lsp_version") {
            payload.insert("perl_lsp_version".to_string(), Value::String("<version>".to_string()));
        }
        if payload.contains_key("workspace_root_hash") {
            payload.insert(
                "workspace_root_hash".to_string(),
                Value::String("<workspace-root-hash>".to_string()),
            );
        }
    }
    value
}

#[test]
fn snapshot_provider_decision_schema_with_caller_receipt_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server();
    seed_completion_trace(&server)?;

    let result = execute_command(
        &server,
        "perl.explainProviderDecision",
        json!([{
            "provider": "completion",
            "receipt_id": "completion-schema-snapshot",
            "scenario": "provider-decision-schema",
            "request_receipt": {
                "provider": "completion",
                "decision": "fallback",
                "reason": "caller_supplied_receipt",
                "fact_source": "compiler_fact",
                "confidence": "medium",
                "freshness": "fresh",
                "fallback_state": "legacy_provider",
                "user_message": "Caller supplied receipt won over the persisted completion trace."
            },
            "request_position": {
                "uri_scheme": "file",
                "line": 2,
                "character": 3
            }
        }]),
        3,
    )?;

    assert_eq!(result.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert!(
        result
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("Caller supplied receipt won")),
        "user_message must include caller receipt detail: {result}"
    );
    let request_receipt = result
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing request_receipt")?;
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("caller_supplied_receipt")
    );
    assert!(
        request_receipt.get("provider_action").is_none(),
        "caller-provided receipt must not be replaced by persisted provider trace: {request_receipt:?}"
    );

    let copyable_payload = result
        .get("copyable_payload")
        .and_then(Value::as_object)
        .ok_or("missing copyable_payload")?;
    assert_eq!(
        copyable_payload.get("schema_version").and_then(Value::as_str),
        Some("provider_decision_bug_report.v1")
    );
    let copyable_receipt = copyable_payload
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing copyable request_receipt")?;
    assert_eq!(
        copyable_receipt.get("reason").and_then(Value::as_str),
        Some("caller_supplied_receipt")
    );
    assert!(
        copyable_receipt.get("provider_action").is_none(),
        "copyable payload must preserve caller receipt precedence: {copyable_receipt:?}"
    );

    assert_yaml_snapshot!(
        "provider_decision_schema_with_caller_receipt_precedence",
        scrub_version(result)
    );
    Ok(())
}

#[test]
fn snapshot_provider_decision_schema_unknown_provider_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server();

    let result = execute_command(
        &server,
        "perl.explainProviderDecision",
        json!([{
            "provider": "unknown"
        }]),
        2,
    )?;

    assert_eq!(result.get("provider").and_then(Value::as_str), Some("unknown"));
    assert_eq!(result.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(result.get("reason").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(result.get("fact_source").and_then(Value::as_str), Some("unknown"));
    assert_eq!(result.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(result.get("freshness").and_then(Value::as_str), Some("unknown"));
    assert_eq!(result.get("fallback").and_then(Value::as_str), Some("no_result"));
    assert!(result.get("user_message").and_then(Value::as_str).is_some());

    let copyable_payload = result
        .get("copyable_payload")
        .and_then(Value::as_object)
        .ok_or("missing copyable_payload")?;
    assert_eq!(copyable_payload.get("provider").and_then(Value::as_str), Some("unknown"));
    assert_eq!(copyable_payload.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(copyable_payload.get("reason").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(copyable_payload.get("fact_source").and_then(Value::as_str), Some("unknown"));
    assert_eq!(copyable_payload.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(copyable_payload.get("freshness").and_then(Value::as_str), Some("unknown"));
    assert_eq!(copyable_payload.get("fallback").and_then(Value::as_str), Some("no_result"));

    assert_yaml_snapshot!(
        "provider_decision_schema_unknown_provider_fallback",
        scrub_version(result)
    );
    Ok(())
}
