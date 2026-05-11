use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const MODULE_URI: &str = "file:///workspace/lib/Symbols/Quality.pm";
const SCRIPT_URI: &str = "file:///workspace/script.pl";

const MODULE: &str = r#"package Symbols::Quality;
use strict;
use warnings;

=head1 NAME

Symbols::Quality - Runtime quality receipt test fixture

=head1 METHODS

=head2 new

Constructor.

=cut

sub new {
    my ($class, %args) = @_;
    return bless { name => $args{name} }, $class;
}

sub name {
    my ($self) = @_;
    return $self->{name};
}

sub greet {
    my ($self) = @_;
    return "Hello, " . $self->name();
}

1;
"#;

const SCRIPT: &str = r#"use strict;
use warnings;
use lib 'lib';
use Symbols::Quality;

my $obj = Symbols::Quality->new(name => 'World');
print $obj->greet(), "\n";
"#;

fn create_server() -> LspServer {
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn open_document(
    server: &LspServer,
    uri: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": uri,
            "text": text,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_symbol_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, MODULE_URI, MODULE)?;
    open_document(server, SCRIPT_URI, SCRIPT)?;
    Ok(())
}

fn symbol_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        _ => 0,
    }
}

fn receipt_notes(receipt: &Value) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let notes = receipt.get("notes").and_then(Value::as_array).ok_or("missing notes")?;
    Ok(notes.iter().filter_map(Value::as_str).collect())
}

// --- document symbol runtime quality receipt tests ---

#[test]
fn document_symbols_runtime_quality_receipt_has_correct_provider_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("provider").and_then(Value::as_str),
        Some("document_symbols"),
        "provider field must identify the document_symbols surface"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_reports_no_live_behavior_change()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must confirm no live behavior change"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_count_matches_live_result()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let live_result = server.test_handle_document_symbols(Some(params.clone()))?;
    let expected_count = symbol_count(live_result.as_ref());

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(expected_count)?),
        "live_provider_count must match the actual live document symbol count"
    );
    // live_provider_result is captured in a single internal call; its count must
    // match live_provider_count (symbol ordering is non-deterministic across calls)
    let receipt_result_count = symbol_count(receipt.get("live_provider_result"));
    assert_eq!(
        receipt_result_count, expected_count,
        "receipt live_provider_result item count must match live_provider_count"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_finds_symbols_in_module_with_subs()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    let count =
        receipt.get("live_provider_count").and_then(Value::as_u64).ok_or("missing count")?;

    assert!(count > 0, "module with package and subs must have at least one document symbol");
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_shadow_state_is_shadowed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("shadowed"),
        "document symbols must report shadowed state (not yet partial live)"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_notes_record_quality_proof()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    let notes = receipt_notes(&receipt)?;
    assert!(!notes.is_empty(), "document symbol receipt must include quality proof notes");
    assert!(
        notes.iter().any(|note| note.contains("document-symbol runtime quality receipt")),
        "notes must identify this as a document-symbol runtime quality receipt: {notes:?}"
    );
    assert!(
        notes.iter().any(|note| note.contains("no live document-symbol behavior change")),
        "notes must confirm no live behavior change: {notes:?}"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_handles_unknown_uri_gracefully()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let params = json!({"textDocument": {"uri": "file:///nonexistent/file.pm"}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must report no live behavior change even for unknown URIs"
    );
    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(0),
        "unknown URI must yield zero symbols"
    );
    Ok(())
}

// --- workspace symbol runtime quality receipt tests ---

#[test]
fn workspace_symbols_runtime_quality_receipt_has_correct_provider_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "new"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("provider").and_then(Value::as_str),
        Some("workspace_symbols"),
        "provider field must identify the workspace_symbols surface"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_reports_no_live_behavior_change()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must confirm no live behavior change"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_count_matches_live_result()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "name"});

    let live_result = server.test_handle_workspace_symbols(Some(params.clone()))?;
    let expected_count = symbol_count(live_result.as_ref());

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(expected_count)?),
        "live_provider_count must match the actual live workspace symbol count"
    );
    assert_eq!(
        receipt.get("live_provider_result"),
        live_result.as_ref(),
        "live_provider_result must equal the live handler result"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_echoes_query_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("query").and_then(Value::as_str),
        Some("greet"),
        "receipt must echo the query field for traceability"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_shadow_state_is_shadowed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": ""});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("shadowed"),
        "workspace symbols must report shadowed state (not yet partial live)"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_notes_record_quality_proof()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "Quality"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    let notes = receipt_notes(&receipt)?;
    assert!(!notes.is_empty(), "workspace symbol receipt must include quality proof notes");
    assert!(
        notes.iter().any(|note| note.contains("workspace-symbol runtime quality receipt")),
        "notes must identify this as a workspace-symbol runtime quality receipt: {notes:?}"
    );
    assert!(
        notes.iter().any(|note| note.contains("no live workspace-symbol behavior change")),
        "notes must confirm no live behavior change: {notes:?}"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_handles_empty_query()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": ""});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must report no live behavior change for empty query"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_handles_no_match_query()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "zzz_no_such_symbol_xyzzy"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(0),
        "unmatched query must yield zero symbols"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must report no live behavior change for zero-result queries"
    );
    Ok(())
}
