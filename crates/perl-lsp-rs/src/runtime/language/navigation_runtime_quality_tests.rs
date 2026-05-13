use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const MODULE_URI: &str = "file:///workspace/lib/Real/Nav.pm";
const MAIN_URI: &str = "file:///workspace/main.pl";

const MODULE: &str = r#"package Real::Nav;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(target);

sub target {
    return 1;
}

sub caller {
    target();
    return Real::Nav::target();
}

1;
"#;

const MAIN: &str = r#"use strict;
use warnings;
use lib 'lib';
use Real::Nav qw(target);

my $first = target();
my $second = Real::Nav::target();
"#;

const LIVE_REFS_URI: &str = "file:///workspace/lib/Live/Refs.pm";

const LIVE_REFS: &str = r#"package Live::Refs;
use strict;
use warnings;

sub target {
    return 1;
}

sub caller {
    target();
    Live::Refs::target();
}

1;
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

fn open_navigation_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, MODULE_URI, MODULE)?;
    open_document(server, MAIN_URI, MAIN)?;
    Ok(())
}

fn open_live_references_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, LIVE_REFS_URI, LIVE_REFS)?;
    Ok(())
}

fn position_of(text: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character)?;
            return Ok((line, character));
        }
    }

    Err(format!("needle `{needle}` not found").into())
}

fn location_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        Some(Value::Object(obj)) if obj.contains_key("uri") || obj.contains_key("targetUri") => 1,
        _ => 0,
    }
}

fn compiler_receipt<'a>(receipt: &'a Value) -> Result<&'a Value, Box<dyn std::error::Error>> {
    let value = receipt.get("compiler_receipt").ok_or("missing compiler_receipt")?;
    if value.is_null() {
        return Err(format!("expected compiler receipt, got runtime receipt: {receipt}").into());
    }
    Ok(value)
}

fn receipt_notes(receipt: &Value) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let notes = receipt.get("notes").and_then(Value::as_array).ok_or("missing notes")?;
    Ok(notes.iter().filter_map(Value::as_str).collect())
}

fn trace_count(receipt: &Value) -> Result<usize, Box<dyn std::error::Error>> {
    let traces = receipt
        .get("fact_source_traces")
        .and_then(Value::as_array)
        .ok_or("missing fact_source_traces")?;
    Ok(traces.len())
}

#[test]
fn navigation_runtime_quality_definition_receipt_compares_live_and_compiler_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_navigation_workspace(&server)?;
    let (line, character) = position_of(MAIN, "target()")?;
    let params = json!({
        "textDocument": {"uri": MAIN_URI},
        "position": {"line": line, "character": character}
    });

    let live_result = server.test_handle_definition(Some(params.clone()))?;
    let runtime_receipt = server
        .test_definition_runtime_quality_receipt(Some(params))?
        .ok_or("missing definition runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;
    let notes = receipt_notes(compiler)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("definition"));
    assert_eq!(
        runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        runtime_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_exact_imported")
    );
    assert_eq!(
        runtime_receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(location_count(live_result.as_ref()))?)
    );
    assert_eq!(runtime_receipt.get("live_provider_result"), live_result.as_ref());
    let first_live_location = live_result
        .as_ref()
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or("expected live definition location")?;
    assert_eq!(
        first_live_location.get("uri").and_then(Value::as_str),
        Some(MODULE_URI),
        "bare imported target should navigate to exporter source"
    );
    assert_eq!(compiler.get("query").and_then(Value::as_str), Some("find_definition"));
    assert!(
        compiler
            .get("new_result")
            .and_then(|r| r.get("match_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert!(trace_count(compiler)? > 0, "definition receipt must carry fact-source traces");
    assert!(
        notes.iter().any(|note| note.contains("definition runtime proof"))
            && notes.iter().any(|note| note.contains("live_import_export_candidates=1"))
            && notes.iter().any(|note| note.contains("partial live exact/imported cutover")),
        "definition receipt notes must record runtime proof and partial live cutover: {notes:?}"
    );

    Ok(())
}

#[test]
fn navigation_runtime_quality_references_receipt_compares_live_and_compiler_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_live_references_workspace(&server)?;
    let (line, character) = position_of(LIVE_REFS, "target();")?;
    let params = json!({
        "textDocument": {"uri": LIVE_REFS_URI},
        "position": {"line": line, "character": character},
        "context": {"includeDeclaration": false}
    });

    let live_result = server.test_handle_references(Some(params.clone()))?;
    let runtime_receipt = server
        .test_references_runtime_quality_receipt(Some(params))?
        .ok_or("missing references runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;
    let notes = receipt_notes(compiler)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("references"));
    assert_eq!(
        runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        runtime_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_exact_imported")
    );
    assert_eq!(
        runtime_receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(location_count(live_result.as_ref()))?)
    );
    assert_eq!(runtime_receipt.get("live_provider_result"), live_result.as_ref());
    assert_eq!(compiler.get("query").and_then(Value::as_str), Some("find_references"));
    assert!(
        compiler
            .get("new_result")
            .and_then(|r| r.get("match_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    );
    assert!(trace_count(compiler)? > 0, "references receipt must carry fact-source traces");
    assert!(
        notes.iter().any(|note| note.contains("references runtime proof"))
            && notes
                .iter()
                .any(|note| note.contains("partial live exact/imported references cutover")),
        "references receipt notes must record runtime proof and partial live cutover: {notes:?}"
    );

    Ok(())
}
