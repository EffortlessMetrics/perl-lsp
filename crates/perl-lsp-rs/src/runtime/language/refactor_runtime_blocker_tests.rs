use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const REFACTOR_URI: &str = "file:///workspace/lib/Refactor/Runtime.pm";

const REFACTOR_MODULE: &str = r#"package Refactor::Runtime;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(exported_target);

sub renamable {
    return 1;
}

sub exported_target {
    return 1;
}

sub caller {
    exported_target();
}

1;
"#;

const DYNAMIC_URI: &str = "file:///workspace/lib/Refactor/Dynamic.pm";

const DYNAMIC_MODULE: &str = r#"package Refactor::Dynamic;
use strict;
use warnings;

eval "sub dyn_target { return 1; }";

sub caller {
    dyn_target();
}

1;
"#;

const GENERATED_URI: &str = "file:///workspace/lib/Refactor/Generated.pm";

const GENERATED_MODULE: &str = r#"package Refactor::Generated;
use strict;
use warnings;
use Moo;

has name => (is => 'ro');

sub caller {
    shift->name;
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

fn position_of(text: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(byte_offset) = line.find(needle) {
            let line_number = u32::try_from(line_idx)?;
            let character = line[..byte_offset].chars().map(char::len_utf16).sum::<usize>();
            let character = u32::try_from(character)?;
            return Ok((line_number, character));
        }
    }

    Err(format!("needle `{needle}` not found").into())
}

#[test]
fn position_of_reports_utf16_character_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let (line, character) = position_of("# café renamable", "renamable")?;

    assert_eq!((line, character), (0, 7));
    Ok(())
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

fn assert_note_contains(
    receipt: &Value,
    expected_parts: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let notes = receipt_notes(receipt)?.join(" ");
    for expected in expected_parts {
        assert!(notes.contains(expected), "receipt notes must contain `{}`: {}", expected, notes);
    }
    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_compares_live_and_compiler_plans()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "newName": "renamed_target"
    });

    let runtime_receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing rename runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;
    let notes = receipt_notes(compiler)?.join(" ");

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(compiler.get("query").and_then(Value::as_str), Some("rename_plan"));
    assert!(trace_count(compiler)? > 0, "rename receipt must carry fact-source traces");
    assert!(
        notes.contains("rename runtime blocker UX")
            && notes.contains("blocker_count=0")
            && notes.contains("blocker_ux=none")
            && notes.contains("no live refactor behavior change"),
        "rename receipt notes must record safe runtime plan without live cutover: {}",
        notes
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_records_exact_static_plan()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character}
    });

    let runtime_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;
    let notes = receipt_notes(compiler)?.join(" ");

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(compiler.get("query").and_then(Value::as_str), Some("safe_delete_plan"));
    assert!(trace_count(compiler)? > 0, "safe-delete receipt must carry fact-source traces");
    assert!(
        notes.contains("safe-delete runtime blocker UX")
            && notes.contains("compiler_plan_safe=true")
            && notes.contains("blocker_count=0")
            && notes.contains("blocker_ux=none")
            && notes.contains("requires_confirmation=false")
            && notes.contains("no live refactor behavior change"),
        "safe-delete receipt notes must record exact static runtime proof without live cutover: {}",
        notes
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_blocks_dynamic_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, DYNAMIC_URI, DYNAMIC_MODULE)?;
    let (line, character) = position_of(DYNAMIC_MODULE, "dyn_target();")?;
    let params = json!({
        "textDocument": {"uri": DYNAMIC_URI},
        "position": {"line": line, "character": character},
        "newName": "renamed_dynamic"
    });

    let runtime_receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing rename runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert!(trace_count(compiler)? > 0, "rename receipt must carry fact-source traces");
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "blocker_count=",
            "blocker_reasons=DynamicBoundary",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_dynamic_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, DYNAMIC_URI, DYNAMIC_MODULE)?;
    let (line, character) = position_of(DYNAMIC_MODULE, "dyn_target();")?;
    let params = json!({
        "textDocument": {"uri": DYNAMIC_URI},
        "position": {"line": line, "character": character}
    });

    let runtime_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert!(trace_count(compiler)? > 0, "safe-delete receipt must carry fact-source traces");
    assert_note_contains(
        compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_safe=false",
            "blocker_count=",
            "blocker_reasons=DynamicBoundary",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_blocks_generated_member()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, GENERATED_URI, GENERATED_MODULE)?;
    let (line, character) = position_of(GENERATED_MODULE, "name =>")?;
    let params = json!({
        "textDocument": {"uri": GENERATED_URI},
        "position": {"line": line, "character": character},
        "newName": "title"
    });

    let runtime_receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing rename runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert!(trace_count(compiler)? > 0, "rename receipt must carry fact-source traces");
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "blocker_count=1",
            "blocker_reasons=GeneratedMember",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_generated_member()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, GENERATED_URI, GENERATED_MODULE)?;
    let (line, character) = position_of(GENERATED_MODULE, "name =>")?;
    let params = json!({
        "textDocument": {"uri": GENERATED_URI},
        "position": {"line": line, "character": character}
    });

    let runtime_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert!(trace_count(compiler)? > 0, "safe-delete receipt must carry fact-source traces");
    assert_note_contains(
        compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_safe=false",
            "blocker_count=1",
            "blocker_reasons=GeneratedMember",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}
