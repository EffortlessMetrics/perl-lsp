use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
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

const SAFE_DELETE_BOUNDARY_URI: &str = "file:///workspace/lib/SafeDelete/Boundary.pm";

const SAFE_DELETE_BOUNDARY_MODULE: &str = r#"package SafeDelete::Boundary;
use strict;
use warnings;

our $CONFIG = 1;

sub keep {
    return 1;
}

1;
"#;

const CROSS_PROJECT_SOURCE_URI: &str = "file:///workspace/lib/CrossProject/Source.pm";
const CROSS_PROJECT_CALLER_URI: &str = "file:///workspace/lib/CrossProject/Caller.pm";

const CROSS_PROJECT_SOURCE_MODULE: &str = r#"package CrossProject::Shared;
use strict;
use warnings;

sub used_target {
    return 1;
}

1;
"#;

const CROSS_PROJECT_CALLER_MODULE: &str = r#"package CrossProject::Shared;
use strict;
use warnings;

sub caller {
    return CrossProject::Shared::used_target();
}

1;
"#;

const DANCER2_DSL_URI: &str = "file:///workspace/lib/Dancer2/Core/DSL.pm";
const DANCER2_APP_URI: &str = "file:///workspace/lib/Dancer2/Core/App.pm";
const DANCER2_RESPONSE_URI: &str = "file:///workspace/lib/Dancer2/Core/Response.pm";
const DANCER2_PLUGIN_URI: &str = "file:///workspace/lib/Dancer2/Plugin.pm";
const CATALYST_DISPATCHER_URI: &str = "file:///workspace/lib/Catalyst/Dispatcher.pm";
const REAL_BASELINE_BASE_URI: &str = "file:///workspace/lib/RealBaseline/Base.pm";
const REAL_BASELINE_UTIL_URI: &str = "file:///workspace/lib/RealBaseline/Util.pm";
const REAL_BASELINE_APP_URI: &str = "file:///workspace/lib/RealBaseline/App.pm";

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

fn change_document(
    server: &LspServer,
    uri: &str,
    version: i32,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_change(Some(json!({
        "textDocument": {
            "uri": uri,
            "version": version
        },
        "contentChanges": [
            { "text": text }
        ]
    })))?;
    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("CARGO_MANIFEST_DIR must be nested under the workspace root")?;
    Ok(root.to_path_buf())
}

fn is_perl_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "pm" | "pl" | "t"))
}

fn collect_perl_files(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_perl_files(root, &path, files)?;
        } else if is_perl_source(&path) {
            let relative_path = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            let content = fs::read_to_string(&path)?;
            files.insert(relative_path, content);
        }
    }
    Ok(())
}

fn load_dancer2_fixture_files() -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    load_real_project_fixture_files("dancer2_skeleton")
}

fn load_semantic_real_workspace_files()
-> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let root = workspace_root()?
        .join("crates")
        .join("perl-workspace")
        .join("tests")
        .join("fixtures")
        .join("semantic_real_workspace")
        .join("cpan_style");
    let mut files = BTreeMap::new();
    collect_perl_files(&root, &root, &mut files)?;
    Ok(files)
}

fn load_real_project_fixture_files(
    project: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let root = workspace_root()?.join("test_corpus").join("real_projects").join(project);
    let mut files = BTreeMap::new();
    collect_perl_files(&root, &root, &mut files)?;
    Ok(files)
}

fn open_dancer2_workspace(
    server: &LspServer,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let files = load_dancer2_fixture_files()?;
    for (relative_path, content) in &files {
        open_document(server, &format!("file:///workspace/{relative_path}"), content)?;
    }
    Ok(files)
}

fn open_catalyst_workspace(
    server: &LspServer,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let files = load_real_project_fixture_files("catalyst_skeleton")?;
    for (relative_path, content) in &files {
        open_document(server, &format!("file:///workspace/{relative_path}"), content)?;
    }
    Ok(files)
}

fn open_semantic_real_workspace(
    server: &LspServer,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let files = load_semantic_real_workspace_files()?;
    for (relative_path, content) in &files {
        open_document(server, &format!("file:///workspace/{relative_path}"), content)?;
    }
    Ok(files)
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

fn assert_trace_contains(
    receipt: &Value,
    expected_source: &str,
    expected_confidence: &str,
    expected_freshness: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let traces = receipt
        .get("fact_source_traces")
        .and_then(Value::as_array)
        .ok_or("missing fact_source_traces")?;
    let found = traces.iter().any(|trace| {
        trace.get("source").and_then(Value::as_str) == Some(expected_source)
            && trace.get("confidence").and_then(Value::as_str) == Some(expected_confidence)
            && trace.get("freshness").and_then(Value::as_str) == Some(expected_freshness)
    });
    assert!(
        found,
        "expected trace source={expected_source} confidence={expected_confidence} freshness={expected_freshness}; traces={traces:?}"
    );
    Ok(())
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

fn assert_json_array_contains(
    value: &Value,
    field: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = value.get(field).and_then(Value::as_array).ok_or("missing array field")?;
    assert!(
        values.iter().filter_map(Value::as_str).any(|actual| actual.contains(expected)),
        "expected `{field}` to contain `{expected}`: {values:?}"
    );
    Ok(())
}

fn request_receipt<'a>(explanation: &'a Value) -> Result<&'a Value, Box<dyn std::error::Error>> {
    explanation.get("request_receipt").ok_or_else(|| "missing persisted request_receipt".into())
}

fn copyable_request_receipt(explanation: &Value) -> Result<&Value, Box<dyn std::error::Error>> {
    explanation
        .pointer("/copyable_payload/request_receipt")
        .ok_or_else(|| "missing copyable request_receipt".into())
}

fn workspace_edit_texts_for_uri<'a>(
    edit: &'a Value,
    uri: &str,
) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> {
    let edits = edit
        .get("changes")
        .and_then(|changes| changes.get(uri))
        .and_then(Value::as_array)
        .ok_or("missing workspace edit changes for URI")?;
    Ok(edits.iter().filter_map(|edit| edit.get("newText").and_then(Value::as_str)).collect())
}

fn workspace_edit_change_count(edit: &Value) -> Result<usize, Box<dyn std::error::Error>> {
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("missing workspace edit changes")?;
    Ok(changes.values().filter_map(Value::as_array).map(Vec::len).sum())
}

fn assert_safe_delete_decision_trace(
    receipt: &Value,
    decision: &str,
    reason: &str,
    fallback_state: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    let provider_action =
        receipt.get("provider_action").and_then(Value::as_str).ok_or("missing provider_action")?;
    assert!(
        provider_action == "safeDelete/runtimeBlockerUxReceipt"
            || provider_action == "perl.previewSafeDelete"
            || provider_action == "perl.safeDeleteSymbol",
        "unexpected safe-delete provider_action: {provider_action}"
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some(decision));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some(reason));
    assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some(fallback_state));
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("not_proven_by_safe_delete_trace")
    );
    let claim_boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing claim_boundary")?;
    assert!(
        claim_boundary
            == "records safe-delete blocker proof only; no live symbol-level delete behavior changes"
            || claim_boundary
                == "scoped safe-delete UX preview only; no live symbol-level delete edits are applied"
            || claim_boundary
                == "narrow safe-delete live pilot only; returns a source-backed symbol-delete WorkspaceEdit when compiler proof, exact source guard, current-source reference guard, workspace identity guard, and rollback proof all pass"
            || claim_boundary
                == "narrow safe-delete live pilot only; returns a source-backed symbol-delete WorkspaceEdit when compiler proof, exact source guard, current-source/workspace reference guards, workspace identity guard, and rollback proof all pass",
        "unexpected safe-delete claim boundary: {claim_boundary}"
    );
    Ok(())
}

fn assert_safe_delete_live_source_guard_blocked(
    receipt: &Value,
    expected_symbol: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some(expected_symbol));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("current_source"));
    assert_eq!(
        receipt.get("live_pilot_source_guard").and_then(Value::as_str),
        Some("not_source_backed_exact_subroutine_definition")
    );
    assert_eq!(
        receipt.get("current_source_delete_guard").and_then(Value::as_str),
        Some("not_source_backed_exact_subroutine_definition")
    );
    assert_eq!(
        receipt.get("live_pilot_workspace_identity_guard").and_then(Value::as_str),
        Some("not_evaluated")
    );
    assert_eq!(receipt.get("live_symbol_delete_enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(receipt.get("returned_workspace_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        receipt
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(
        receipt,
        "blocked",
        "not_source_backed_exact_subroutine_definition",
        "no_edit",
    )?;

    let live_blocker_ux = receipt.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_json_array_contains(
        live_blocker_ux,
        "blocker_reasons",
        "NotSourceBackedExactSubroutineDefinition",
    )?;

    let message = receipt
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing safe-delete source guard user_message")?;
    assert!(
        message.contains("Safe delete refused")
            && message.contains(expected_symbol)
            && message.contains("No edits were returned"),
        "source guard message should explain the no-edit refusal: {message}"
    );

    Ok(())
}

fn explain_provider_decision_with_request_receipt(
    server: &LspServer,
    provider: &str,
    receipt_id: &str,
    scenario: &str,
    request_receipt: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = server
        .handle_execute_command(Some(json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": provider,
                "receipt_id": receipt_id,
                "scenario": scenario,
                "request_receipt": request_receipt
            }]
        })))?
        .ok_or("missing explain-provider-decision response")?;
    Ok(response)
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

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_blocks_low_confidence_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "newName": "renamed_target",
        "compilerPlanFixture": "low_confidence"
    });

    let runtime_receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing rename runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        runtime_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("low_confidence")
    );
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_trace_contains(compiler, "SemanticFact", "Low", "Fresh")?;
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "compiler_plan_fixture=low_confidence",
            "blocker_reasons=AmbiguousReference",
            "low_confidence=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_blocks_stale_fact_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "newName": "renamed_target",
        "compilerPlanFixture": "stale_fact"
    });

    let runtime_receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing rename runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        runtime_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("stale_fact")
    );
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_trace_contains(compiler, "CompilerFact", "Low", "Stale")?;
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "compiler_plan_fixture=stale_fact",
            "blocker_reasons=StaleFact",
            "stale_fact=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_low_confidence_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "compilerPlanFixture": "low_confidence"
    });

    let runtime_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        runtime_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("low_confidence")
    );
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_trace_contains(compiler, "SemanticFact", "Low", "Fresh")?;
    assert_note_contains(
        compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=low_confidence",
            "compiler_plan_safe=false",
            "blocker_reasons=AmbiguousReference",
            "low_confidence=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_stale_fact_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "compilerPlanFixture": "stale_fact"
    });

    let runtime_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete runtime receipt")?;
    let compiler = compiler_receipt(&runtime_receipt)?;

    assert_eq!(runtime_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        runtime_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("stale_fact")
    );
    assert_eq!(runtime_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_trace_contains(compiler, "CompilerFact", "Low", "Stale")?;
    assert_note_contains(
        compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=stale_fact",
            "compiler_plan_safe=false",
            "blocker_reasons=StaleFact",
            "stale_fact=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

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
fn refactor_runtime_blocker_ux_rename_receipt_records_package_fallback_noise()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let helper_params = json!({
        "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
        "position": {"line": helper_line, "character": helper_character},
        "newName": "renamed_helper"
    });
    let receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(helper_params))?
        .ok_or("missing real-workspace rename fallback/noise receipt")?;
    let compiler = compiler_receipt(&receipt)?;
    let fallback_noise = receipt.get("fallback_noise").ok_or("missing fallback_noise")?;

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(receipt.get("new_name").and_then(Value::as_str), Some("renamed_helper"));
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    let package_pilot = receipt.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(package_pilot.get("eligible").and_then(Value::as_bool), Some(false));
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("blocked"));
    assert_eq!(package_pilot.get("edit_count").and_then(Value::as_u64), Some(1));
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
    assert_json_array_contains(package_pilot, "blocker_reasons", "ImportedSymbol")?;
    assert_eq!(package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool), Some(true));
    assert_eq!(fallback_noise.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(fallback_noise.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(fallback_noise.get("new_name").and_then(Value::as_str), Some("renamed_helper"));
    assert_eq!(
        fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_blocked")
    );
    let live_edit_count = fallback_noise
        .get("live_provider_edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing live_provider_edit_count")?;
    let compiler_edit_count = fallback_noise
        .get("compiler_plan_edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing compiler_plan_edit_count")?;
    assert_eq!(compiler_edit_count, 1, "compiler plan should include the definition edit");
    let live_state =
        fallback_noise.get("live_provider_state").and_then(Value::as_str).ok_or("missing state")?;
    assert_eq!(live_state, "error", "unexpected live provider state: {fallback_noise}");
    assert!(
        fallback_noise
            .get("live_provider_error")
            .and_then(Value::as_str)
            .is_some_and(|message| !message.is_empty()),
        "error state must include the live provider error: {fallback_noise}"
    );
    assert_eq!(
        live_edit_count, 0,
        "live provider should not produce edits after refusal: {fallback_noise}"
    );
    assert!(
        fallback_noise.get("live_provider_error").and_then(Value::as_str).is_some_and(|message| {
            message.contains("ambiguous symbol identity") && message.contains("helper")
        }),
        "package/compiler-backed rename receipt should expose the live fallback/noise reason: {fallback_noise}"
    );
    assert_trace_contains(compiler, "CompilerFact", "High", "Fresh")?;
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "compiler_plan_edits=",
            "blocker_count=1",
            "ImportedSymbol",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_records_real_workspace_package_pilot_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (shared_line, shared_character) = position_of(base, "shared {")?;
    let shared_params = json!({
        "textDocument": {"uri": REAL_BASELINE_BASE_URI},
        "position": {"line": shared_line, "character": shared_character},
        "newName": "renamed_shared"
    });
    let receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(shared_params))?
        .ok_or("missing real-workspace package pilot rename receipt")?;
    let compiler = compiler_receipt(&receipt)?;
    let package_pilot = receipt.get("package_pilot").ok_or("missing package_pilot")?;
    let fallback_noise = receipt.get("fallback_noise").ok_or("missing fallback_noise")?;

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("shared"));
    assert_eq!(receipt.get("new_name").and_then(Value::as_str), Some("renamed_shared"));
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(package_pilot.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        package_pilot.get("eligible").and_then(Value::as_bool),
        Some(true),
        "unexpected package pilot classification: {package_pilot}"
    );
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    assert_eq!(package_pilot.get("edit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(package_pilot.get("blocker_count").and_then(Value::as_u64), Some(0));
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
    assert_eq!(package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool), Some(true));
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );
    assert_eq!(
        fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(false)
    );
    assert_trace_contains(compiler, "SemanticFact", "High", "Fresh")?;
    assert_note_contains(
        compiler,
        &[
            "rename package pilot proof",
            "eligible=true",
            "reason=none",
            "edit_count=1",
            "claim_boundary=receipt-only package/compiler-backed pilot",
            "no_live_rename_cutover=true",
            "rename runtime blocker UX",
            "requires_confirmation=false",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_rename_preview_command_returns_scoped_no_edit_ux()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (shared_line, shared_character) = position_of(base, "shared {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_BASE_URI},
                "position": {"line": shared_line, "character": shared_character},
                "newName": "renamed_shared"
            }]
        })))?
        .ok_or("missing package rename preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        preview_result.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(preview_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("live_package_rename_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        preview_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert!(
        preview_result.get("planned_workspace_edit").is_some(),
        "package rename preview should return the planned live-provider edit shape: {preview_result}"
    );
    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        rollback_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );
    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(package_pilot.get("eligible").and_then(Value::as_bool), Some(true));
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    assert_eq!(package_pilot.get("edit_count").and_then(Value::as_u64), Some(1));
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
    assert_eq!(package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool), Some(true));
    assert_eq!(
        preview_result.get("claim_boundary").and_then(Value::as_str),
        Some("scoped package rename preview only; no package rename edits are applied")
    );

    let preview_message = preview_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing package rename preview user_message")?;
    assert!(
        preview_message.contains("Package rename preview")
            && preview_message.contains("shared")
            && preview_message.contains("renamed_shared")
            && preview_message.contains("no package rename edits were applied"),
        "package rename preview message should explain the no-edit allowed proof: {preview_message}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(request_receipt.get("user_message").and_then(Value::as_str), Some(preview_message));

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_rename_preview_records_imported_call_noise_and_rollback()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let app = files.get("lib/RealBaseline/App.pm").ok_or("missing RealBaseline App fixture")?;

    let (alias_line, alias_character) = position_of(app, "alias($self->shared)")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_APP_URI},
                "position": {"line": alias_line, "character": alias_character},
                "newName": "renamed_alias"
            }]
        })))?
        .ok_or("missing imported-call package rename preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(
        preview_result.get("fallback_state").and_then(Value::as_str),
        Some("compiler_missing")
    );
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("live_package_rename_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        preview_result.get("planned_live_provider_edit_count").and_then(Value::as_u64),
        Some(1),
        "preview should record the legacy same-file fallback edit count without returning edits: {preview_result}"
    );
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0),
        "package rename preview must return no live edits: {preview_result}"
    );
    assert_eq!(
        preview_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0),
        "preview workspace edit must remain empty: {preview_result}"
    );

    let fallback_noise = preview_result.get("fallback_noise").ok_or("missing fallback_noise")?;
    assert_eq!(fallback_noise.get("symbol").and_then(Value::as_str), Some("alias"));
    assert_eq!(fallback_noise.get("new_name").and_then(Value::as_str), Some("renamed_alias"));
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_missing")
    );
    assert_eq!(fallback_noise.get("compiler_available").and_then(Value::as_bool), Some(false));
    assert_eq!(fallback_noise.get("live_provider_state").and_then(Value::as_str), Some("edits"));
    assert_eq!(fallback_noise.get("live_provider_edit_count").and_then(Value::as_u64), Some(1));

    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(
        rollback_receipt.get("planned_live_provider_edit_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        rollback_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_missing")
    );
    assert!(
        rollback_receipt
            .get("claim_boundary")
            .and_then(Value::as_str)
            .is_some_and(|boundary| boundary.contains("no package rename edits")),
        "rollback receipt must keep package rename preview no-edit: {rollback_receipt}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(
        request_receipt.pointer("/rollback_receipt/fallback_state").and_then(Value::as_str),
        Some("compiler_missing")
    );
    assert_eq!(
        request_receipt
            .pointer("/rollback_receipt/returned_workspace_edit_count")
            .and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_rename_preview_records_compiler_allowed_live_pilot_without_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let uri = "file:///workspace/lib/Rename/Pilot.pm";
    let source = r#"package Rename::Pilot;
use strict;
use warnings;

sub pilot_target {
    return 1;
}

sub caller {
    return pilot_target();
}

1;
"#;
    open_document(&server, uri, source)?;

    let (target_line, target_character) = position_of(source, "pilot_target {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": uri},
                "position": {"line": target_line, "character": target_character},
                "newName": "renamed_pilot_target",
                "compilerPlanFixture": "package_pilot_allowed"
            }]
        })))?
        .ok_or("missing package rename live-pilot preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(
        preview_result.get("decision").and_then(Value::as_str),
        Some("allowed"),
        "unexpected package rename preview live-pilot receipt: {preview_result}"
    );
    assert_eq!(
        preview_result.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(preview_result.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(preview_result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(preview_result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(preview_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("live_package_rename_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        preview_result.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        preview_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0),
        "package rename live-pilot preview must not return live edits: {preview_result}"
    );
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        package_pilot.get("eligible").and_then(Value::as_bool),
        Some(true),
        "package/compiler-backed pilot should be eligible before live cutover: {package_pilot}"
    );
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    assert_eq!(package_pilot.get("blocker_count").and_then(Value::as_u64), Some(0));
    assert_eq!(package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool), Some(true));
    assert!(
        package_pilot.get("edit_count").and_then(Value::as_u64).is_some_and(|count| count >= 2),
        "package/compiler-backed pilot should cover definition and reference edits: {package_pilot}"
    );
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
    assert_json_array_contains(package_pilot, "edit_categories", "Reference")?;

    let fallback_noise = preview_result.get("fallback_noise").ok_or("missing fallback_noise")?;
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );
    assert_eq!(
        fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        fallback_noise
            .get("compiler_plan_edit_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count >= 2),
        "fallback/noise receipt should preserve eligible compiler plan edit count: {fallback_noise}"
    );

    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(
        rollback_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );

    let user_message = preview_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing package rename preview user_message")?;
    assert!(
        user_message.contains("Package rename preview")
            && user_message.contains("pilot_target")
            && user_message.contains("renamed_pilot_target")
            && user_message.contains("no package rename edits were applied"),
        "allowed preview message should explain the no-edit live-pilot proof: {user_message}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(
        request_receipt.pointer("/package_pilot/eligible").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        request_receipt.pointer("/rollback_receipt/rollback_safe").and_then(Value::as_bool),
        Some(true)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_rename_preview_records_dancer2_source_backed_pilot_without_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let response =
        files.get("lib/Dancer2/Core/Response.pm").ok_or("missing Dancer2 Core Response fixture")?;

    let (to_psgi_line, to_psgi_character) = position_of(response, "to_psgi {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": to_psgi_line, "character": to_psgi_character},
                "newName": "renamed_to_psgi"
            }]
        })))?
        .ok_or("missing Dancer2 package rename live-pilot preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        preview_result.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(preview_result.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(preview_result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(preview_result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(preview_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("live_package_rename_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        preview_result.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        preview_result.get("source_backed_state").and_then(Value::as_str),
        Some("not_authorized_by_package_rename_preview")
    );
    assert_eq!(
        preview_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0),
        "Dancer2 package rename preview must not return live edits: {preview_result}"
    );
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        package_pilot.get("eligible").and_then(Value::as_bool),
        Some(true),
        "Dancer2 package/compiler-backed pilot should be eligible before live cutover: {package_pilot}"
    );
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    assert_eq!(package_pilot.get("blocker_count").and_then(Value::as_u64), Some(0));
    assert_eq!(package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool), Some(true));
    assert_eq!(
        package_pilot.get("edit_count").and_then(Value::as_u64),
        Some(1),
        "Dancer2 source-backed pilot should record the package-local definition edit only: {package_pilot}"
    );
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;

    let fallback_noise = preview_result.get("fallback_noise").ok_or("missing fallback_noise")?;
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );
    assert_eq!(
        fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(fallback_noise.get("compiler_plan_edit_count").and_then(Value::as_u64), Some(1));

    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(
        rollback_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_allowed")
    );

    let user_message = preview_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing Dancer2 package rename preview user_message")?;
    assert!(
        user_message.contains("Package rename preview")
            && user_message.contains("to_psgi")
            && user_message.contains("renamed_to_psgi")
            && user_message.contains("no package rename edits were applied"),
        "Dancer2 preview message should explain the no-edit live-pilot proof: {user_message}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        request_receipt.pointer("/package_pilot/eligible").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        request_receipt.pointer("/rollback_receipt/rollback_safe").and_then(Value::as_bool),
        Some(true)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_dancer2_edit_freshness_falls_back_to_current_source()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let response =
        files.get("lib/Dancer2/Core/Response.pm").ok_or("missing Dancer2 Core Response fixture")?;

    let (to_psgi_line, to_psgi_character) = position_of(response, "to_psgi {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": to_psgi_line, "character": to_psgi_character},
                "newName": "renamed_to_psgi"
            }]
        })))?
        .ok_or("missing Dancer2 package rename edit-freshness preview result")?;

    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(package_pilot.get("eligible").and_then(Value::as_bool), Some(true));
    let compiler_edit_count = package_pilot
        .get("edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing package pilot edit count")?;
    assert_eq!(
        compiler_edit_count, 1,
        "Dancer2 compiler preview should start with only the source-backed definition edit: {package_pilot}"
    );
    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));

    let updated_response = response.replace(
        "sub is_forwarded",
        "sub as_array {\n    my $self = shift;\n    return $self->to_psgi;\n}\n\nsub is_forwarded",
    );
    change_document(&server, DANCER2_RESPONSE_URI, 2, &updated_response)?;

    let rename_result = server
        .handle_rename_workspace(Some(json!({
            "textDocument": {"uri": DANCER2_RESPONSE_URI},
            "position": {"line": to_psgi_line, "character": to_psgi_character},
            "newName": "renamed_to_psgi"
        })))?
        .ok_or("missing Dancer2 package-local fresh fallback result")?;
    let edit_count = workspace_edit_change_count(&rename_result)?;
    assert!(
        edit_count > usize::try_from(compiler_edit_count)?,
        "Dancer2 current-source fallback must not promote the stale one-edit compiler preview after didChange: compiler={compiler_edit_count}, live={edit_count}, result={rename_result}"
    );
    let response_texts = workspace_edit_texts_for_uri(&rename_result, DANCER2_RESPONSE_URI)?;
    assert!(
        response_texts.iter().filter(|text| **text == "renamed_to_psgi").count() >= 2,
        "fresh fallback should include the Dancer2 definition and newly added call: {rename_result}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("full_index_workspace_edit")
    );
    assert_eq!(
        request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("workspace_index")
    );
    assert_eq!(
        request_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(edit_count).ok()
    );
    let copyable_receipt = copyable_request_receipt(&explanation)?;
    assert_eq!(
        copyable_receipt.get("reason").and_then(Value::as_str),
        Some("full_index_workspace_edit")
    );
    assert_eq!(
        copyable_receipt.get("fallback_state").and_then(Value::as_str),
        Some("workspace_index")
    );
    assert_eq!(
        copyable_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(edit_count).ok()
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_applies_exact_source_backed_plan()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let uri = "file:///workspace/lib/Rename/Pilot.pm";
    let source = r#"package Rename::Pilot;
use strict;
use warnings;

sub pilot_target {
    return 1;
}

sub caller {
    return 1;
}

1;
"#;
    open_document(&server, uri, source)?;

    let (target_line, target_character) = position_of(source, "pilot_target {")?;
    let rename_result = server
        .handle_rename_workspace(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": target_line, "character": target_character},
            "newName": "renamed_pilot_target"
        })))?
        .ok_or("missing package-local live rename result")?;

    let changes = rename_result
        .get("changes")
        .and_then(Value::as_object)
        .ok_or("missing package-local live rename changes")?;
    let edit_count = changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>();
    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert!(
        edit_count == 1,
        "package-local live pilot should apply only the exact source-backed edit set: {rename_result}; receipt={request_receipt}"
    );

    let base_texts = workspace_edit_texts_for_uri(&rename_result, uri)?;
    assert!(
        base_texts.iter().filter(|text| **text == "renamed_pilot_target").count() == 1,
        "package-local live pilot should rename only the source-backed definition when no references exist: {rename_result}"
    );

    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("package_local_live_pilot")
    );
    assert_eq!(request_receipt.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(
        request_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(edit_count).ok()
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_falls_back_on_partial_source_backed_plan()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let uri = "file:///workspace/lib/Rename/Pilot.pm";
    let source = r#"package Rename::Pilot;
use strict;
use warnings;

sub pilot_target {
    return 1;
}

sub caller {
    return pilot_target();
}

1;
"#;
    open_document(&server, uri, source)?;

    let (target_line, target_character) = position_of(source, "pilot_target {")?;
    let rename_result = server
        .handle_rename_workspace(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": target_line, "character": target_character},
            "newName": "renamed_pilot_target"
        })))?
        .ok_or("missing package-local live rename result")?;

    let edit_count = rename_result
        .get("changes")
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .ok_or("missing fallback workspace edit changes")?;
    assert_eq!(
        edit_count, 2,
        "partial semantic package-local pilot must fall back to the existing safe workspace-index edit set: {rename_result}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("full_index_workspace_edit")
    );
    assert_eq!(
        request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("workspace_index")
    );
    assert_eq!(request_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(2));

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_blocks_real_workspace_imported_symbol_false_allow()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let rename_result = server
        .handle_rename_workspace(Some(json!({
            "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
            "position": {"line": helper_line, "character": helper_character},
            "newName": "renamed_helper"
        })))?
        .ok_or("missing package-local live rename blocker result")?;

    let edit_count = rename_result
        .get("changes")
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .ok_or("missing package-local live rename blocker changes")?;
    assert_eq!(
        edit_count, 0,
        "imported/exported real-workspace package symbol must not be falsely allowed as a package-local edit: {rename_result}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("package_local_live_pilot_blocked")
    );
    assert_eq!(request_receipt.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(request_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("helper"));
    assert!(
        request_receipt.get("claim_boundary").and_then(Value::as_str).is_some_and(|boundary| {
            boundary.contains("package-local compiler facts")
                && boundary.contains("broader compiler-backed refactor facts remain gated")
        }),
        "rename trace must preserve the package-local claim boundary: {request_receipt}"
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_real_workspace_false_allow_falls_back_with_fresh_rollback_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let app = files.get("lib/RealBaseline/App.pm").ok_or("missing RealBaseline App fixture")?;

    let (name_line, name_character) = position_of(app, "name {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_APP_URI},
                "position": {"line": name_line, "character": name_character},
                "newName": "renamed_name"
            }]
        })))?
        .ok_or("missing package rename false-allow preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        preview_result.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("eligible").and_then(Value::as_bool), Some(true));
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    let compiler_edit_count = package_pilot
        .get("edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing package pilot edit count")?;
    assert_eq!(
        compiler_edit_count, 1,
        "RealBaseline package pilot should see only the source-backed definition edit: {package_pilot}"
    );
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;

    let rollback_receipt =
        preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_receipt.get("live_package_rename_enabled").and_then(Value::as_bool),
        Some(false)
    );

    let rename_request = json!({
        "textDocument": {"uri": REAL_BASELINE_APP_URI},
        "position": {"line": name_line, "character": name_character},
        "newName": "renamed_name"
    });
    let live_result = server
        .handle_rename_workspace(Some(rename_request.clone()))?
        .ok_or("missing package-local live rename fallback result")?;
    let live_edit_count = workspace_edit_change_count(&live_result)?;
    assert!(
        live_edit_count > usize::try_from(compiler_edit_count)?,
        "workspace guard must catch the package-pilot false allow and return broader current-source fallback edits: compiler={compiler_edit_count}, live={live_edit_count}, result={live_result}"
    );
    let live_app_texts = workspace_edit_texts_for_uri(&live_result, REAL_BASELINE_APP_URI)?;
    assert!(
        live_app_texts.iter().filter(|text| **text == "renamed_name").count() >= 2,
        "workspace-index fallback must include RealBaseline::App::name references in App.pm: {live_result}"
    );

    let explanation = explain_provider_decision(&server, "rename")?;
    let live_receipt = request_receipt(&explanation)?;
    assert_eq!(live_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        live_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert_eq!(
        live_receipt.get("reason").and_then(Value::as_str),
        Some("full_index_workspace_edit")
    );
    assert_eq!(live_receipt.get("fallback_state").and_then(Value::as_str), Some("workspace_index"));
    assert_eq!(
        live_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(live_edit_count).ok()
    );

    let updated_app = app.replace("helper($self->name);", "helper($self->name);\n    $self->name;");
    change_document(&server, REAL_BASELINE_APP_URI, 2, &updated_app)?;

    let fresh_live_result = server
        .handle_rename_workspace(Some(rename_request))?
        .ok_or("missing package-local fresh fallback result")?;
    let fresh_edit_count = workspace_edit_change_count(&fresh_live_result)?;
    assert!(
        fresh_edit_count > live_edit_count,
        "current-source fallback must use fresh didChange state instead of stale package-pilot evidence: before={live_edit_count}, after={fresh_edit_count}, result={fresh_live_result}"
    );
    let fresh_app_texts = workspace_edit_texts_for_uri(&fresh_live_result, REAL_BASELINE_APP_URI)?;
    assert!(
        fresh_app_texts.iter().filter(|text| **text == "renamed_name").count()
            > live_app_texts.iter().filter(|text| **text == "renamed_name").count(),
        "post-edit fallback should include the newly added App.pm name call: before={live_app_texts:?}, after={fresh_app_texts:?}"
    );

    let fresh_explanation = explain_provider_decision(&server, "rename")?;
    let fresh_receipt = request_receipt(&fresh_explanation)?;
    assert_eq!(fresh_receipt.get("reason").and_then(Value::as_str), Some("same_file_semantic"));
    assert_eq!(fresh_receipt.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(
        fresh_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(fresh_edit_count).ok()
    );
    let fresh_copyable_receipt = copyable_request_receipt(&fresh_explanation)?;
    assert_eq!(
        fresh_copyable_receipt.get("reason").and_then(Value::as_str),
        Some("same_file_semantic")
    );
    assert_eq!(fresh_copyable_receipt.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(
        fresh_copyable_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(fresh_edit_count).ok()
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_catalyst_false_allow_blocks()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_catalyst_workspace(&server)?;
    let dispatcher =
        files.get("lib/Catalyst/Dispatcher.pm").ok_or("missing Catalyst Dispatcher fixture")?;

    let (get_action_line, get_action_character) = position_of(dispatcher, "get_action {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": CATALYST_DISPATCHER_URI},
                "position": {"line": get_action_line, "character": get_action_character},
                "newName": "renamed_get_action"
            }]
        })))?
        .ok_or("missing Catalyst package rename preview result")?;

    assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        preview_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewPackageRename")
    );
    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(
        preview_result.get("reason").and_then(Value::as_str),
        Some("compiler_preview_allowed")
    );
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
    assert_eq!(package_pilot.get("eligible").and_then(Value::as_bool), Some(true));
    assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("none"));
    let compiler_edit_count = package_pilot
        .get("edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing package pilot edit count")?;
    assert_eq!(
        compiler_edit_count, 1,
        "Catalyst package pilot should see only the source-backed definition edit: {package_pilot}"
    );
    assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
    assert!(
        preview_result
            .get("claim_boundary")
            .and_then(Value::as_str)
            .is_some_and(|boundary| boundary.contains("no package rename edits are applied")),
        "Catalyst package rename preview must remain no-edit: {preview_result}"
    );

    let rename_request = json!({
        "textDocument": {"uri": CATALYST_DISPATCHER_URI},
        "position": {"line": get_action_line, "character": get_action_character},
        "newName": "renamed_get_action"
    });
    let live_result = match server.handle_rename_workspace(Some(rename_request)) {
        Ok(Some(result)) => {
            let edit_count = workspace_edit_change_count(&result)?;
            assert_eq!(
                edit_count, 0,
                "Catalyst ambiguous package-local false allow must not return edits: {result}"
            );
            Some(result)
        }
        Ok(None) => return Err("missing Catalyst package-local live rename result".into()),
        Err(error) => {
            assert_eq!(error.code, -32602);
            assert!(
                error.message.contains("ambiguous symbol identity"),
                "Catalyst false-allow refusal should explain ambiguous project-shaped identity: {error:?}"
            );
            None
        }
    };
    let live_edit_count = live_result.as_ref().map_or(Ok(0), workspace_edit_change_count)?;

    let explanation = explain_provider_decision(&server, "rename")?;
    let live_receipt = request_receipt(&explanation)?;
    assert_eq!(live_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        live_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    assert!(
        live_receipt.get("claim_boundary").and_then(Value::as_str).is_some_and(|boundary| {
            boundary.contains("package-local compiler facts")
                && boundary.contains("broader compiler-backed refactor facts remain gated")
        }),
        "Catalyst rename trace must preserve the package-local claim boundary: {live_receipt}"
    );

    let reason = live_receipt
        .get("reason")
        .and_then(Value::as_str)
        .ok_or("missing Catalyst live rename reason")?;
    assert_eq!(
        reason, "package_local_live_pilot_ambiguous",
        "Catalyst package-local false allow must be refused as ambiguous identity: {live_receipt}"
    );
    assert_eq!(
        live_edit_count, 0,
        "ambiguous package-local pilot must not return edits: {live_result:?}"
    );
    assert_eq!(
        live_receipt.get("fallback_state").and_then(Value::as_str),
        Some("ambiguous_identity")
    );
    assert_eq!(live_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_package_local_live_pilot_receipt_preserves_cutover_guardrails()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;

    for (fixture, expected_reason, expected_fallback, expected_fact_source) in [
        ("generated_member", "generated_no_source", "no_edit", "framework_adapter"),
        ("dynamic_boundary", "dynamic_boundary", "no_edit", "dynamic_boundary"),
        ("stale_fact", "stale_fact", "refresh_workspace_facts", "compiler_fact"),
        (
            "low_confidence",
            "ambiguous_low_confidence_candidates",
            "require_confirmation",
            "semantic_fact",
        ),
    ] {
        let preview_result = server
            .handle_execute_command(Some(json!({
                "command": "perl.previewPackageRename",
                "arguments": [{
                    "textDocument": {"uri": REFACTOR_URI},
                    "position": {"line": line, "character": character},
                    "newName": "renamed_target",
                    "compilerPlanFixture": fixture
                }]
            })))?
            .ok_or("missing package rename blocker preview result")?;

        assert_eq!(preview_result.get("provider").and_then(Value::as_str), Some("rename"));
        assert_eq!(
            preview_result.get("provider_action").and_then(Value::as_str),
            Some("perl.previewPackageRename")
        );
        assert_eq!(
            preview_result.get("decision").and_then(Value::as_str),
            Some("blocked"),
            "package-local live-pilot blocker receipt must block `{fixture}`: {preview_result}"
        );
        assert_eq!(
            preview_result.get("reason").and_then(Value::as_str),
            Some(expected_reason),
            "package-local live-pilot blocker reason should identify `{fixture}`: {preview_result}"
        );
        assert_eq!(
            preview_result.get("fact_source").and_then(Value::as_str),
            Some(expected_fact_source)
        );
        assert_eq!(
            preview_result.get("fallback_state").and_then(Value::as_str),
            Some(expected_fallback)
        );
        assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
        assert_eq!(
            preview_result.get("live_package_rename_enabled").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            preview_result.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            preview_result
                .pointer("/workspace_edit/changes")
                .and_then(Value::as_object)
                .map(serde_json::Map::len),
            Some(0)
        );
        assert_eq!(
            preview_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
            Some(0)
        );

        let package_pilot = preview_result.get("package_pilot").ok_or("missing package_pilot")?;
        assert_eq!(
            package_pilot.get("eligible").and_then(Value::as_bool),
            Some(false),
            "blocked package-local live-pilot receipt must not be eligible: {package_pilot}"
        );
        assert_eq!(package_pilot.get("reason").and_then(Value::as_str), Some("blocked"));
        assert_eq!(package_pilot.get("edit_count").and_then(Value::as_u64), Some(2));
        assert_eq!(package_pilot.get("blocker_count").and_then(Value::as_u64), Some(1));
        assert_json_array_contains(package_pilot, "edit_categories", "Definition")?;
        assert_json_array_contains(package_pilot, "edit_categories", "Reference")?;
        assert_eq!(
            package_pilot.get("no_live_rename_cutover").and_then(Value::as_bool),
            Some(true)
        );

        let fallback_noise =
            preview_result.get("fallback_noise").ok_or("missing fallback_noise")?;
        assert_eq!(
            fallback_noise.get("fallback_state").and_then(Value::as_str),
            Some("compiler_blocked")
        );
        assert_eq!(fallback_noise.get("compiler_plan_edit_count").and_then(Value::as_u64), Some(2));
        assert_eq!(
            fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
            Some(true)
        );

        let rollback_receipt =
            preview_result.get("rollback_receipt").ok_or("missing rollback_receipt")?;
        assert_eq!(
            rollback_receipt.get("provider_action").and_then(Value::as_str),
            Some("perl.previewPackageRename")
        );
        assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
        assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
        assert_eq!(rollback_receipt.get("edits_applied").and_then(Value::as_bool), Some(false));
        assert_eq!(
            rollback_receipt.get("live_package_rename_enabled").and_then(Value::as_bool),
            Some(false)
        );

        let user_message = preview_result
            .get("user_message")
            .and_then(Value::as_str)
            .ok_or("missing package rename blocker user_message")?;
        assert!(
            user_message.contains("Package rename preview refused")
                && user_message.contains("renamable")
                && user_message.contains("renamed_target")
                && user_message.contains("No edits were applied"),
            "blocked preview message should explain the no-edit guardrail: {user_message}"
        );

        let explanation = explain_provider_decision(&server, "rename")?;
        let request_receipt = request_receipt(&explanation)?;
        assert_eq!(
            request_receipt.get("provider_action").and_then(Value::as_str),
            Some("perl.previewPackageRename")
        );
        assert_eq!(request_receipt.get("reason").and_then(Value::as_str), Some(expected_reason));
        assert_eq!(
            request_receipt.pointer("/package_pilot/eligible").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            request_receipt
                .pointer("/rollback_receipt/live_package_rename_enabled")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_request_receipt_preserves_package_fallback_noise()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let helper_params = json!({
        "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
        "position": {"line": helper_line, "character": helper_character},
        "newName": "renamed_helper"
    });
    let receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(helper_params))?
        .ok_or("missing real-workspace rename fallback/noise receipt")?;
    let fallback_noise = receipt.get("fallback_noise").ok_or("missing fallback_noise")?.clone();

    let explanation = explain_provider_decision_with_request_receipt(
        &server,
        "rename",
        "realbaseline-rename-fallback-noise",
        "helper-to-renamed_helper",
        fallback_noise,
    )?;

    assert_eq!(explanation.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(explanation.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(
        explanation.get("receipt_id").and_then(Value::as_str),
        Some("realbaseline-rename-fallback-noise")
    );
    assert_eq!(
        explanation.get("scenario").and_then(Value::as_str),
        Some("helper-to-renamed_helper")
    );
    let request_receipt = explanation
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing request-local rename receipt")?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_blocked")
    );
    assert_eq!(request_receipt.get("compiler_plan_edit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        request_receipt.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(request_receipt.get("live_provider_state").and_then(Value::as_str), Some("error"));
    assert!(
        request_receipt.get("live_provider_error").and_then(Value::as_str).is_some_and(|message| {
            message.contains("ambiguous symbol identity") && message.contains("helper")
        }),
        "request-local receipt must preserve live fallback/noise reason: {request_receipt:?}"
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_rename_receipt_records_imported_call_fallback_noise()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let app = files.get("lib/RealBaseline/App.pm").ok_or("missing RealBaseline App fixture")?;

    let (alias_line, alias_character) = position_of(app, "alias($self->shared)")?;
    let alias_params = json!({
        "textDocument": {"uri": REAL_BASELINE_APP_URI},
        "position": {"line": alias_line, "character": alias_character},
        "newName": "renamed_alias"
    });
    let receipt = server
        .test_rename_runtime_blocker_ux_receipt(Some(alias_params))?
        .ok_or("missing real-workspace imported-call rename fallback/noise receipt")?;
    let fallback_noise = receipt.get("fallback_noise").ok_or("missing fallback_noise")?;

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("alias"));
    assert_eq!(receipt.get("new_name").and_then(Value::as_str), Some("renamed_alias"));
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert!(
        receipt.get("compiler_receipt").is_some_and(Value::is_null),
        "imported-call receipt should record missing compiler receipt explicitly: {receipt}"
    );
    assert_eq!(fallback_noise.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(fallback_noise.get("symbol").and_then(Value::as_str), Some("alias"));
    assert_eq!(fallback_noise.get("new_name").and_then(Value::as_str), Some("renamed_alias"));
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_missing")
    );
    assert_eq!(fallback_noise.get("compiler_available").and_then(Value::as_bool), Some(false));
    assert_eq!(fallback_noise.get("compiler_requires_confirmation"), Some(&Value::Null));
    assert!(
        fallback_noise.get("compiler_plan_edit_count").is_some_and(Value::is_null),
        "imported-call rename receipt should not claim compiler edits without a compiler receipt: {fallback_noise}"
    );
    assert_eq!(
        fallback_noise.get("live_provider_edit_count").and_then(Value::as_u64),
        Some(1),
        "imported-call receipt should preserve the legacy same-file fallback edit count as noise: {fallback_noise}"
    );
    assert_eq!(
        fallback_noise.get("live_provider_state").and_then(Value::as_str),
        Some("edits"),
        "unexpected imported-call live provider state: {fallback_noise}"
    );
    assert!(
        fallback_noise.get("live_provider_error").is_some_and(Value::is_null),
        "fallback-noise state should not fabricate a provider error: {fallback_noise}"
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
    assert_safe_delete_decision_trace(&runtime_receipt, "blocked", "dynamic_boundary", "no_edit")?;
    assert_eq!(
        runtime_receipt.get("fact_source").and_then(Value::as_str),
        Some("dynamic_boundary")
    );
    assert_eq!(runtime_receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(true));
    assert_json_array_contains(&runtime_receipt, "blocker_reasons", "DynamicBoundary")?;
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
fn refactor_runtime_blocker_ux_safe_delete_fixture_persists_blocker_decision_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, REFACTOR_URI, REFACTOR_MODULE)?;
    let (line, character) = position_of(REFACTOR_MODULE, "renamable")?;
    let params = json!({
        "textDocument": {"uri": REFACTOR_URI},
        "position": {"line": line, "character": character},
        "compilerPlanFixture": "dynamic_boundary"
    });

    let receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(params))?
        .ok_or("missing safe-delete fixture receipt")?;
    assert_safe_delete_decision_trace(&receipt, "blocked", "dynamic_boundary", "no_edit")?;

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let persisted = request_receipt(&explanation)?;
    assert_safe_delete_decision_trace(persisted, "blocked", "dynamic_boundary", "no_edit")?;
    assert_eq!(persisted.get("symbol").and_then(Value::as_str), Some("renamable"));
    assert_eq!(persisted.get("dynamic_boundary").and_then(Value::as_bool), Some(true));
    assert_eq!(persisted.get("blocker_count").and_then(Value::as_u64), Some(1));
    assert_json_array_contains(persisted, "blocker_reasons", "DynamicBoundary")?;

    let live_blocker_ux = persisted.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_eq!(live_blocker_ux.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(live_blocker_ux.get("fallback").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(live_blocker_ux.get("requires_confirmation").and_then(Value::as_bool), Some(true));

    let rollback_receipt = persisted.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("blocked_before_edit").and_then(Value::as_bool), Some(true));

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

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_dancer2_stale_symbol_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let dsl = files.get("lib/Dancer2/Core/DSL.pm").ok_or("missing Dancer2 Core DSL fixture")?;

    let (compile_line, compile_character) = position_of(dsl, "_compile {")?;
    let compile_params = json!({
        "textDocument": {"uri": DANCER2_DSL_URI},
        "position": {"line": compile_line, "character": compile_character},
        "compilerPlanFixture": "stale_fact"
    });
    let compile_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(compile_params))?
        .ok_or("missing Dancer2 stale-symbol safe-delete receipt")?;
    let compile_compiler = compiler_receipt(&compile_receipt)?;

    assert_eq!(compile_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        compile_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("stale_fact")
    );
    assert_eq!(compile_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(compile_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_safe_delete_decision_trace(
        &compile_receipt,
        "blocked",
        "stale_fact",
        "refresh_workspace_facts",
    )?;
    assert_eq!(compile_receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(compile_receipt.get("freshness").and_then(Value::as_str), Some("stale"));
    assert_trace_contains(compile_compiler, "CompilerFact", "Low", "Stale")?;
    assert!(trace_count(compile_compiler)? > 0, "_compile receipt must carry fact-source traces");
    assert_note_contains(
        compile_compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=stale_fact",
            "compiler_plan_safe=false",
            "blocker_reasons=StaleFact",
            "stale_fact=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_dancer2_generated_dynamic_low_confidence()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let app = files.get("lib/Dancer2/Core/App.pm").ok_or("missing Dancer2 Core App fixture")?;
    let dsl = files.get("lib/Dancer2/Core/DSL.pm").ok_or("missing Dancer2 Core DSL fixture")?;
    let plugin = files.get("lib/Dancer2/Plugin.pm").ok_or("missing Dancer2 Plugin fixture")?;

    let (routes_line, routes_character) = position_of(app, "routes      =>")?;
    let generated_params = json!({
        "textDocument": {"uri": DANCER2_APP_URI},
        "position": {"line": routes_line, "character": routes_character},
        "compilerPlanFixture": "generated_member"
    });
    let generated_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(generated_params))?
        .ok_or("missing Dancer2 generated safe-delete receipt")?;
    let generated_compiler = compiler_receipt(&generated_receipt)?;

    assert_eq!(generated_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        generated_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("generated_member")
    );
    assert_eq!(generated_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        generated_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(generated_receipt.get("symbol").and_then(Value::as_str), Some("routes"));
    assert_safe_delete_decision_trace(
        &generated_receipt,
        "blocked",
        "generated_no_source",
        "no_edit",
    )?;
    assert_eq!(
        generated_receipt.get("fact_source").and_then(Value::as_str),
        Some("framework_adapter")
    );
    assert_trace_contains(generated_compiler, "FrameworkAdapter", "High", "Fresh")?;
    assert_note_contains(
        generated_compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=generated_member",
            "compiler_plan_safe=false",
            "blocker_reasons=GeneratedMember",
            "generated_member=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    let (plugin_keywords_line, plugin_keywords_character) = position_of(plugin, "plugin_keywords")?;
    let dynamic_params = json!({
        "textDocument": {"uri": DANCER2_PLUGIN_URI},
        "position": {"line": plugin_keywords_line, "character": plugin_keywords_character},
        "compilerPlanFixture": "dynamic_boundary"
    });
    let dynamic_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(dynamic_params))?
        .ok_or("missing Dancer2 dynamic-boundary safe-delete receipt")?;
    let dynamic_compiler = compiler_receipt(&dynamic_receipt)?;

    assert_eq!(dynamic_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        dynamic_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("dynamic_boundary")
    );
    assert_eq!(dynamic_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(dynamic_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(dynamic_receipt.get("symbol").and_then(Value::as_str), Some("plugin_keywords"));
    assert_safe_delete_decision_trace(&dynamic_receipt, "blocked", "dynamic_boundary", "no_edit")?;
    assert_eq!(
        dynamic_receipt.get("fact_source").and_then(Value::as_str),
        Some("dynamic_boundary")
    );
    assert_eq!(dynamic_receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(true));
    assert_trace_contains(dynamic_compiler, "DynamicBoundary", "High", "Fresh")?;
    assert_note_contains(
        dynamic_compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=dynamic_boundary",
            "compiler_plan_safe=false",
            "blocker_reasons=DynamicBoundary",
            "dynamic_boundary=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    let (compile_line, compile_character) = position_of(dsl, "_compile {")?;
    let low_confidence_params = json!({
        "textDocument": {"uri": DANCER2_DSL_URI},
        "position": {"line": compile_line, "character": compile_character},
        "compilerPlanFixture": "low_confidence"
    });
    let low_confidence_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(low_confidence_params))?
        .ok_or("missing Dancer2 low-confidence safe-delete receipt")?;
    let low_confidence_compiler = compiler_receipt(&low_confidence_receipt)?;

    assert_eq!(low_confidence_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        low_confidence_receipt.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("low_confidence")
    );
    assert_eq!(
        low_confidence_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        low_confidence_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(low_confidence_receipt.get("symbol").and_then(Value::as_str), Some("_compile"));
    assert_safe_delete_decision_trace(
        &low_confidence_receipt,
        "blocked",
        "ambiguous_low_confidence_candidates",
        "require_confirmation",
    )?;
    assert_eq!(low_confidence_receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_trace_contains(low_confidence_compiler, "SemanticFact", "Low", "Fresh")?;
    assert_note_contains(
        low_confidence_compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_fixture=low_confidence",
            "compiler_plan_safe=false",
            "blocker_reasons=AmbiguousReference",
            "low_confidence=true",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_blocks_real_workspace_imported_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let helper_params = json!({
        "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
        "position": {"line": helper_line, "character": helper_character}
    });
    let helper_receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(helper_params))?
        .ok_or("missing real-workspace referenced-symbol safe-delete receipt")?;
    let helper_compiler = compiler_receipt(&helper_receipt)?;

    assert_eq!(helper_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(helper_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(helper_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(helper_receipt.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(helper_compiler.get("query").and_then(Value::as_str), Some("safe_delete_plan"));
    assert_trace_contains(helper_compiler, "CompilerFact", "High", "Fresh")?;
    assert!(trace_count(helper_compiler)? > 0, "helper receipt must carry fact-source traces");
    assert_note_contains(
        helper_compiler,
        &[
            "safe-delete runtime blocker UX",
            "compiler_plan_safe=false",
            "blocker_reasons=",
            "ImportedSymbol",
            "imported by another file",
            "requires_confirmation=true",
            "no live refactor behavior change",
        ],
    )?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_records_allowed_symbol_cutover_proof()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (reset_line, reset_character) = position_of(base, "reset {")?;
    let reset_params = json!({
        "textDocument": {"uri": REAL_BASELINE_BASE_URI},
        "position": {"line": reset_line, "character": reset_character}
    });
    let receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(reset_params))?
        .ok_or("missing real-workspace safe-delete allowed-symbol receipt")?;
    let compiler = compiler_receipt(&receipt)?;

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("reset"));
    assert_eq!(receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(compiler.get("query").and_then(Value::as_str), Some("safe_delete_plan"));
    assert_trace_contains(compiler, "SemanticFact", "High", "Fresh")?;
    assert_note_contains(
        compiler,
        &[
            "safe-delete cutover receipt",
            "compiler_plan_safe=true",
            "blocker_count=0",
            "blocker_reasons=none",
            "fallback_state=allowed",
            "blocker_ux=none",
            "requires_confirmation=false",
            "no live refactor behavior change",
        ],
    )?;

    let live_blocker_ux = receipt.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_eq!(live_blocker_ux.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(live_blocker_ux.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(live_blocker_ux.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(live_blocker_ux.get("requires_confirmation").and_then(Value::as_bool), Some(false));
    assert_eq!(
        live_blocker_ux.get("blocker_reasons").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert_eq!(
        live_blocker_ux.get("blocker_messages").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );

    let rollback_receipt = receipt.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(rollback_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("blocked_before_edit").and_then(Value::as_bool), Some(false));
    assert!(
        rollback_receipt
            .get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("plan allowed")
                && reason.contains("no live symbol-level delete")),
        "rollback receipt should explain the allowed no-live-edit path: {rollback_receipt}"
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_proves_symbol_delete_edit_rollback()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (reset_line, reset_character) = position_of(base, "reset {")?;
    let reset_params = json!({
        "textDocument": {"uri": REAL_BASELINE_BASE_URI},
        "position": {"line": reset_line, "character": reset_character},
        "includeEditRollbackProof": true
    });
    let receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(reset_params))?
        .ok_or("missing safe-delete edit rollback receipt")?;

    assert_safe_delete_decision_trace(&receipt, "allowed", "compiler_allowed", "none")?;
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("reset"));
    assert_eq!(receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));

    let rollback_proof =
        receipt.get("symbol_delete_edit_rollback").ok_or("missing symbol_delete_edit_rollback")?;
    assert_eq!(rollback_proof.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        rollback_proof.get("provider_action").and_then(Value::as_str),
        Some("safeDelete/symbolDeleteEditRollbackProof")
    );
    assert_eq!(rollback_proof.get("edit_plan_state").and_then(Value::as_str), Some("planned"));
    assert_eq!(rollback_proof.get("planned_delete_edit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(rollback_proof.get("rollback_edit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(rollback_proof.get("rollback_required").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_proof.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_proof.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        rollback_proof.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(rollback_proof.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        rollback_proof.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_subroutine_range")
    );
    assert_eq!(
        rollback_proof.get("rollback_verification").and_then(Value::as_str),
        Some("restores_original")
    );
    assert_eq!(
        rollback_proof.get("claim_boundary").and_then(Value::as_str),
        Some("safe-delete edit rollback proof only; no live symbol-level delete edits are applied")
    );

    let planned_delete = rollback_proof
        .get("planned_delete_workspace_edit")
        .ok_or("missing planned_delete_workspace_edit")?;
    let planned_texts = workspace_edit_texts_for_uri(planned_delete, REAL_BASELINE_BASE_URI)?;
    assert_eq!(
        planned_texts,
        vec![""],
        "delete proof must replace the symbol range with empty text"
    );

    let rollback_edit =
        rollback_proof.get("rollback_workspace_edit").ok_or("missing rollback_workspace_edit")?;
    let rollback_texts = workspace_edit_texts_for_uri(rollback_edit, REAL_BASELINE_BASE_URI)?;
    let rollback_text = rollback_texts.first().ok_or("missing rollback insertion text")?;
    assert!(
        rollback_text.contains("sub reset") && rollback_text.contains("return 1;"),
        "rollback proof must carry the source-backed symbol text: {rollback_text}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt
            .pointer("/symbol_delete_edit_rollback/rollback_verification")
            .and_then(Value::as_str),
        Some("restores_original")
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_explain_provider_decision_replays_persisted_safe_delete_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (reset_line, reset_character) = position_of(base, "reset {")?;
    let reset_params = json!({
        "textDocument": {"uri": REAL_BASELINE_BASE_URI},
        "position": {"line": reset_line, "character": reset_character}
    });
    let receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(reset_params))?
        .ok_or("missing real-workspace safe-delete allowed-symbol receipt")?;
    assert_eq!(receipt.get("symbol").and_then(Value::as_str), Some("reset"));

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    assert_eq!(explanation.get("provider").and_then(Value::as_str), Some("safe_delete"));

    let request_receipt = request_receipt(&explanation)?;
    assert_safe_delete_decision_trace(request_receipt, "allowed", "compiler_allowed", "none")?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("reset"));
    assert_eq!(request_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(
        request_receipt
            .get("compiler_receipt")
            .and_then(|value| value.get("query"))
            .and_then(Value::as_str),
        Some("safe_delete_plan")
    );
    assert_eq!(
        request_receipt
            .get("live_blocker_ux")
            .and_then(|value| value.get("decision"))
            .and_then(Value::as_str),
        Some("allowed")
    );

    let caller_receipt = json!({
        "provider": "safe_delete",
        "reason": "caller_supplied_receipt"
    });
    let caller_explanation = explain_provider_decision_with_request_receipt(
        &server,
        "safe_delete",
        "docs/project/status/provider_confidence_matrix.md#safe-delete",
        "caller-overrides-persisted-receipt",
        caller_receipt,
    )?;
    let caller_request_receipt =
        caller_explanation.get("request_receipt").ok_or("missing caller request_receipt")?;
    assert_eq!(
        caller_request_receipt.get("reason").and_then(Value::as_str),
        Some("caller_supplied_receipt")
    );
    assert!(
        caller_request_receipt.get("symbol").is_none(),
        "caller-provided request_receipt must take precedence over persisted trace"
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_explain_provider_decision_replays_live_rename_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let uri = "file:///workspace/lib/RenameTrace.pm";
    let source = r#"package RenameTrace;
use strict;
use warnings;

sub run {
    my $value = 1;
    return $value;
}

1;
"#;
    open_document(&server, uri, source)?;
    let (line, character) = position_of(source, "$value =")?;
    let rename_result = server
        .handle_rename_workspace(Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "newName": "renamed_value"
        })))?
        .ok_or("missing live rename response")?;
    let edit_count = rename_result
        .get("changes")
        .and_then(|changes| changes.get(uri))
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or("missing live rename edits")?;
    assert!(edit_count > 0, "live rename must produce the baseline lexical edits");

    let explanation = explain_provider_decision(&server, "rename")?;
    let request_receipt =
        explanation.get("request_receipt").ok_or("missing persisted rename request_receipt")?;
    assert_eq!(request_receipt.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/rename")
    );
    let reason = request_receipt.get("reason").and_then(Value::as_str).ok_or("missing reason")?;
    assert!(
        matches!(reason, "same_file_lexical" | "same_file_semantic"),
        "expected a persisted same-file rename trace, got: {request_receipt}"
    );
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("$value"));
    assert_eq!(
        request_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(edit_count).ok()
    );
    let copyable_receipt = copyable_request_receipt(&explanation)?;
    assert_eq!(copyable_receipt.get("reason").and_then(Value::as_str), Some(reason));
    assert_eq!(
        copyable_receipt.get("fallback_state").and_then(Value::as_str),
        request_receipt.get("fallback_state").and_then(Value::as_str)
    );
    assert_eq!(
        copyable_receipt.get("live_provider_edit_count").and_then(Value::as_u64),
        u64::try_from(edit_count).ok()
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_receipt_records_live_blocker_ux_and_rollback()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let helper_params = json!({
        "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
        "position": {"line": helper_line, "character": helper_character}
    });
    let receipt = server
        .test_safe_delete_runtime_blocker_ux_receipt(Some(helper_params))?
        .ok_or("missing real-workspace safe-delete live blocker UX receipt")?;

    let live_blocker_ux = receipt.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_eq!(live_blocker_ux.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(live_blocker_ux.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(live_blocker_ux.get("fallback").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(live_blocker_ux.get("requires_confirmation").and_then(Value::as_bool), Some(true));
    assert_json_array_contains(live_blocker_ux, "blocker_reasons", "ImportedSymbol")?;
    assert_json_array_contains(live_blocker_ux, "blocker_messages", "imported by another file")?;

    let rollback_receipt = receipt.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback_receipt.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(rollback_receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(rollback_receipt.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_receipt.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_receipt.get("blocked_before_edit").and_then(Value::as_bool), Some(true));
    assert!(
        rollback_receipt
            .get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("blocker") && reason.contains("no live edits")),
        "rollback receipt should explain the no-edit blocked path: {rollback_receipt}"
    );

    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_preview_command_returns_scoped_no_edit_ux()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewSafeDelete",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
                "position": {"line": helper_line, "character": helper_character}
            }]
        })))?
        .ok_or("missing safe-delete preview blocker result")?;
    assert_eq!(blocked_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewSafeDelete")
    );
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(blocked_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    let blocked_message = blocked_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing safe-delete blocker user_message")?;
    assert!(
        blocked_message.contains("Safe delete refused")
            && blocked_message.contains("helper")
            && blocked_message.contains("No edits were applied"),
        "blocked preview message should explain the refusal without edits: {blocked_message}"
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;
    assert_eq!(
        blocked_result.get("claim_boundary").and_then(Value::as_str),
        Some("scoped safe-delete UX preview only; no live symbol-level delete edits are applied")
    );

    let blocked_explanation = explain_provider_decision(&server, "safe_delete")?;
    let blocked_request_receipt = request_receipt(&blocked_explanation)?;
    assert_eq!(
        blocked_request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.previewSafeDelete")
    );
    assert_eq!(
        blocked_request_receipt.get("user_message").and_then(Value::as_str),
        Some(blocked_message)
    );

    let (reset_line, reset_character) = position_of(base, "reset {")?;
    let allowed_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewSafeDelete",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_BASE_URI},
                "position": {"line": reset_line, "character": reset_character}
            }]
        })))?
        .ok_or("missing safe-delete preview allowed result")?;
    assert_eq!(allowed_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        allowed_result.get("provider_action").and_then(Value::as_str),
        Some("perl.previewSafeDelete")
    );
    assert_eq!(allowed_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(allowed_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(allowed_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    let allowed_message = allowed_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing safe-delete allowed user_message")?;
    assert!(
        allowed_message.contains("Safe delete preview")
            && allowed_message.contains("reset")
            && allowed_message.contains("no symbol deletion was applied"),
        "allowed preview message should describe the no-edit preview path: {allowed_message}"
    );
    assert_safe_delete_decision_trace(&allowed_result, "allowed", "compiler_allowed", "none")?;

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_returns_source_backed_edit_only()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;
    let base = files.get("lib/RealBaseline/Base.pm").ok_or("missing RealBaseline Base fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
                "position": {"line": helper_line, "character": helper_character}
            }]
        })))?
        .ok_or("missing safe-delete live pilot blocker result")?;
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        blocked_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;

    let (reset_line, reset_character) = position_of(base, "reset {")?;
    let live_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_BASE_URI},
                "position": {"line": reset_line, "character": reset_character}
            }]
        })))?
        .ok_or("missing safe-delete live pilot result")?;
    assert_eq!(live_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        live_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(live_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(live_result.get("reason").and_then(Value::as_str), Some("compiler_allowed"));
    assert_eq!(live_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(
        live_result.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(false),
        "live pilot should return an edit: {live_result}"
    );
    assert_eq!(live_result.get("no_live_behavior_change").and_then(Value::as_bool), Some(false));
    assert_eq!(live_result.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        live_result.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_subroutine_range")
    );
    assert_eq!(
        live_result.get("live_pilot_source_guard").and_then(Value::as_str),
        Some("source_backed_exact_subroutine_definition")
    );
    assert_eq!(live_result.get("live_symbol_delete_enabled").and_then(Value::as_bool), Some(true));
    assert_eq!(live_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(live_result.get("returned_workspace_edit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        live_result.get("claim_boundary").and_then(Value::as_str),
        Some(
            "narrow safe-delete live pilot only; returns a source-backed symbol-delete WorkspaceEdit when compiler proof, exact source guard, current-source/workspace reference guards, workspace identity guard, and rollback proof all pass"
        )
    );

    let workspace_edit = live_result.get("workspace_edit").ok_or("missing live workspace_edit")?;
    let live_texts = workspace_edit_texts_for_uri(workspace_edit, REAL_BASELINE_BASE_URI)?;
    assert_eq!(live_texts, vec![""], "live pilot must return one delete edit");

    let rollback_proof =
        live_result.get("symbol_delete_edit_rollback").ok_or("missing rollback proof")?;
    assert_eq!(
        rollback_proof.get("rollback_verification").and_then(Value::as_str),
        Some("restores_original")
    );
    assert_eq!(rollback_proof.get("rollback_safe").and_then(Value::as_bool), Some(true));

    let message = live_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing live pilot user_message")?;
    assert!(
        message.contains("Safe delete can remove")
            && message.contains("reset")
            && message.contains("WorkspaceEdit")
            && message.contains("no edit was applied by the server"),
        "live pilot message should explain the returned edit without server-side application: {message}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(
        request_receipt.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        request_receipt
            .pointer("/symbol_delete_edit_rollback/rollback_verification")
            .and_then(Value::as_str),
        Some("restores_original")
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_records_second_project_source_backed_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let response =
        files.get("lib/Dancer2/Core/Response.pm").ok_or("missing Dancer2 Core Response fixture")?;

    let (to_psgi_line, to_psgi_character) = position_of(response, "to_psgi {")?;
    let live_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": to_psgi_line, "character": to_psgi_character}
            }]
        })))?
        .ok_or("missing Dancer2 safe-delete live pilot result")?;

    assert_eq!(live_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        live_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(live_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(live_result.get("symbol").and_then(Value::as_str), Some("to_psgi"));
    assert_eq!(live_result.get("reason").and_then(Value::as_str), Some("compiler_allowed"));
    assert_eq!(live_result.get("fallback_state").and_then(Value::as_str), Some("none"));
    assert_eq!(live_result.get("live_symbol_delete_enabled").and_then(Value::as_bool), Some(true));
    assert_eq!(live_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(live_result.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        live_result.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_subroutine_range")
    );
    assert_eq!(
        live_result.get("live_pilot_source_guard").and_then(Value::as_str),
        Some("source_backed_exact_subroutine_definition")
    );
    assert_eq!(live_result.get("returned_workspace_edit_count").and_then(Value::as_u64), Some(1));

    let workspace_edit =
        live_result.get("workspace_edit").ok_or("missing Dancer2 live workspace_edit")?;
    let live_texts = workspace_edit_texts_for_uri(workspace_edit, DANCER2_RESPONSE_URI)?;
    assert_eq!(live_texts, vec![""], "Dancer2 live pilot must return one delete edit");

    let rollback_proof =
        live_result.get("symbol_delete_edit_rollback").ok_or("missing Dancer2 rollback proof")?;
    assert_eq!(
        rollback_proof.get("rollback_verification").and_then(Value::as_str),
        Some("restores_original")
    );
    assert_eq!(rollback_proof.get("rollback_safe").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_proof.get("source_backed").and_then(Value::as_bool), Some(true));
    let rollback_edit = rollback_proof
        .get("rollback_workspace_edit")
        .ok_or("missing Dancer2 rollback_workspace_edit")?;
    let rollback_texts = workspace_edit_texts_for_uri(rollback_edit, DANCER2_RESPONSE_URI)?;
    let rollback_text = rollback_texts.first().ok_or("missing Dancer2 rollback insertion text")?;
    assert!(
        rollback_text.contains("sub to_psgi") && rollback_text.contains("$self->status"),
        "Dancer2 rollback text must carry the source-backed symbol body: {rollback_text}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("to_psgi"));
    assert_eq!(
        request_receipt.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        request_receipt
            .pointer("/symbol_delete_edit_rollback/rollback_verification")
            .and_then(Value::as_str),
        Some("restores_original")
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_catalyst_false_allow_blocks()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_catalyst_workspace(&server)?;
    let dispatcher =
        files.get("lib/Catalyst/Dispatcher.pm").ok_or("missing Catalyst Dispatcher fixture")?;

    let (get_action_line, get_action_character) = position_of(dispatcher, "get_action {")?;
    let live_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": CATALYST_DISPATCHER_URI},
                "position": {"line": get_action_line, "character": get_action_character}
            }]
        })))?
        .ok_or("missing Catalyst safe-delete live pilot result")?;

    assert_eq!(live_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        live_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(live_result.get("symbol").and_then(Value::as_str), Some("get_action"));
    assert_eq!(live_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(
        live_result.get("reason").and_then(Value::as_str),
        Some("ambiguous_low_confidence_candidates")
    );
    assert_eq!(live_result.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(live_result.get("live_symbol_delete_enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(live_result.get("returned_workspace_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        live_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(
        &live_result,
        "blocked",
        "ambiguous_low_confidence_candidates",
        "no_edit",
    )?;

    let live_blocker_ux = live_result.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_json_array_contains(live_blocker_ux, "blocker_reasons", "AmbiguousReference")?;
    assert_eq!(
        live_result.get("live_pilot_workspace_identity_guard").and_then(Value::as_str),
        Some("ambiguous_workspace_identity")
    );
    assert!(
        live_result
            .get("claim_boundary")
            .and_then(Value::as_str)
            .is_some_and(|boundary| boundary.contains("narrow safe-delete live pilot")),
        "Catalyst safe-delete receipt must preserve the live-pilot claim boundary: {live_result}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("get_action"));
    assert_eq!(
        request_receipt.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_non_subroutine_and_package_wide()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, SAFE_DELETE_BOUNDARY_URI, SAFE_DELETE_BOUNDARY_MODULE)?;

    let (config_line, config_character) = position_of(SAFE_DELETE_BOUNDARY_MODULE, "$CONFIG =")?;
    let variable_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": SAFE_DELETE_BOUNDARY_URI},
                "position": {"line": config_line, "character": config_character}
            }]
        })))?
        .ok_or("missing non-subroutine safe-delete blocker result")?;
    assert_safe_delete_live_source_guard_blocked(&variable_result, "$CONFIG")?;

    let (package_line, package_character) = position_of(SAFE_DELETE_BOUNDARY_MODULE, "Boundary;")?;
    let package_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": SAFE_DELETE_BOUNDARY_URI},
                "position": {"line": package_line, "character": package_character}
            }]
        })))?
        .ok_or("missing package-wide safe-delete blocker result")?;
    assert_safe_delete_live_source_guard_blocked(&package_result, "Boundary")?;

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("Boundary"));
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("not_source_backed_exact_subroutine_definition")
    );
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_imported_symbol_false_allow()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_semantic_real_workspace(&server)?;
    let util = files.get("lib/RealBaseline/Util.pm").ok_or("missing RealBaseline Util fixture")?;

    let (helper_line, helper_character) = position_of(util, "helper {")?;
    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": REAL_BASELINE_UTIL_URI},
                "position": {"line": helper_line, "character": helper_character}
            }]
        })))?
        .ok_or("missing imported-symbol safe-delete live blocker result")?;

    assert_eq!(blocked_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(blocked_result.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(blocked_result.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_eq!(blocked_result.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(blocked_result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(blocked_result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(blocked_result.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(
        blocked_result.get("live_pilot_source_guard").and_then(Value::as_str),
        Some("source_backed_exact_subroutine_definition")
    );
    assert_eq!(
        blocked_result.get("live_pilot_workspace_identity_guard").and_then(Value::as_str),
        Some("not_evaluated")
    );
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(blocked_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        blocked_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;
    assert_json_array_contains(&blocked_result, "blocker_reasons", "ImportedSymbol")?;

    let live_blocker_ux = blocked_result.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_json_array_contains(live_blocker_ux, "blocker_reasons", "ImportedSymbol")?;
    assert_json_array_contains(live_blocker_ux, "blocker_messages", "imported by another file")?;

    let rollback_proof =
        blocked_result.get("symbol_delete_edit_rollback").ok_or("missing rollback proof")?;
    assert_eq!(rollback_proof.get("blocked_before_edit").and_then(Value::as_bool), Some(true));
    assert_eq!(rollback_proof.get("rollback_required").and_then(Value::as_bool), Some(false));
    assert_eq!(rollback_proof.get("rollback_safe").and_then(Value::as_bool), Some(true));

    let message = blocked_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing imported-symbol safe-delete blocked user_message")?;
    assert!(
        message.contains("Safe delete refused")
            && message.contains("helper")
            && message.contains("No edits were returned"),
        "blocked message should explain the imported-symbol no-edit path: {message}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(request_receipt.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_json_array_contains(request_receipt, "blocker_reasons", "ImportedSymbol")?;
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_generated_and_dynamic_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let app = files.get("lib/Dancer2/Core/App.pm").ok_or("missing Dancer2 Core App fixture")?;
    let plugin = files.get("lib/Dancer2/Plugin.pm").ok_or("missing Dancer2 Plugin fixture")?;

    let (routes_line, routes_character) = position_of(app, "routes      =>")?;
    let generated_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": DANCER2_APP_URI},
                "position": {"line": routes_line, "character": routes_character},
                "compilerPlanFixture": "generated_member"
            }]
        })))?
        .ok_or("missing generated-member safe-delete live blocker result")?;
    assert_eq!(generated_result.get("symbol").and_then(Value::as_str), Some("routes"));
    assert_eq!(
        generated_result.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("generated_member")
    );
    assert_safe_delete_decision_trace(
        &generated_result,
        "blocked",
        "generated_no_source",
        "no_edit",
    )?;
    assert_eq!(
        generated_result.get("fact_source").and_then(Value::as_str),
        Some("framework_adapter")
    );
    assert_eq!(
        generated_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        generated_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        generated_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert!(
        generated_result.get("current_source_delete_guard").is_none(),
        "generated/no-source blocker should remain compiler-driven, not source-guard promoted: {generated_result}"
    );
    let generated_blocker_ux =
        generated_result.get("live_blocker_ux").ok_or("missing generated live_blocker_ux")?;
    assert_json_array_contains(generated_blocker_ux, "blocker_reasons", "GeneratedMember")?;

    let generated_explanation = explain_provider_decision(&server, "safe_delete")?;
    let generated_receipt = request_receipt(&generated_explanation)?;
    assert_eq!(generated_receipt.get("symbol").and_then(Value::as_str), Some("routes"));
    assert_eq!(
        generated_receipt.get("reason").and_then(Value::as_str),
        Some("generated_no_source")
    );
    assert_eq!(
        generated_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    let (plugin_keywords_line, plugin_keywords_character) = position_of(plugin, "plugin_keywords")?;
    let dynamic_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": DANCER2_PLUGIN_URI},
                "position": {"line": plugin_keywords_line, "character": plugin_keywords_character},
                "compilerPlanFixture": "dynamic_boundary"
            }]
        })))?
        .ok_or("missing dynamic-boundary safe-delete live blocker result")?;
    assert_eq!(dynamic_result.get("symbol").and_then(Value::as_str), Some("plugin_keywords"));
    assert_eq!(
        dynamic_result.get("compiler_plan_fixture").and_then(Value::as_str),
        Some("dynamic_boundary")
    );
    assert_safe_delete_decision_trace(&dynamic_result, "blocked", "dynamic_boundary", "no_edit")?;
    assert_eq!(dynamic_result.get("fact_source").and_then(Value::as_str), Some("dynamic_boundary"));
    assert_eq!(dynamic_result.get("dynamic_boundary").and_then(Value::as_bool), Some(true));
    assert_eq!(
        dynamic_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        dynamic_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        dynamic_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert!(
        dynamic_result.get("current_source_delete_guard").is_none(),
        "dynamic blocker should remain dynamic-boundary driven, not source-guard promoted: {dynamic_result}"
    );
    let dynamic_blocker_ux =
        dynamic_result.get("live_blocker_ux").ok_or("missing dynamic live_blocker_ux")?;
    assert_json_array_contains(dynamic_blocker_ux, "blocker_reasons", "DynamicBoundary")?;

    let dynamic_explanation = explain_provider_decision(&server, "safe_delete")?;
    let dynamic_receipt = request_receipt(&dynamic_explanation)?;
    assert_eq!(dynamic_receipt.get("symbol").and_then(Value::as_str), Some("plugin_keywords"));
    assert_eq!(dynamic_receipt.get("reason").and_then(Value::as_str), Some("dynamic_boundary"));
    assert_eq!(dynamic_receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(true));
    assert_eq!(
        dynamic_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_dancer2_referenced_source_backed_method()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let response =
        files.get("lib/Dancer2/Core/Response.pm").ok_or("missing Dancer2 Core Response fixture")?;

    let (header_line, header_character) = position_of(response, "header {")?;
    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": header_line, "character": header_character}
            }]
        })))?
        .ok_or("missing Dancer2 referenced safe-delete blocker result")?;

    assert_eq!(blocked_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(blocked_result.get("symbol").and_then(Value::as_str), Some("header"));
    assert_eq!(blocked_result.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert!(
        blocked_result
            .get("fact_source")
            .and_then(Value::as_str)
            .is_some_and(|source| source == "compiler_fact" || source == "current_source"),
        "referenced Dancer2 method blocker should be compiler-backed or current-source backed: {blocked_result}"
    );
    assert_eq!(blocked_result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(blocked_result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(blocked_result.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(blocked_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        blocked_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;

    let live_blocker_ux = blocked_result.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_json_array_contains(&blocked_result, "blocker_reasons", "ReferencesExist")?;
    assert_json_array_contains(live_blocker_ux, "blocker_messages", "still has")?;

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(request_receipt.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert!(
        request_receipt
            .get("fact_source")
            .and_then(Value::as_str)
            .is_some_and(|source| source == "compiler_fact" || source == "current_source"),
        "persisted Dancer2 receipt should keep the referenced-source blocker: {request_receipt}"
    );
    assert_eq!(request_receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(request_receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_cross_file_referenced_source_backed_sub()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, CROSS_PROJECT_SOURCE_URI, CROSS_PROJECT_SOURCE_MODULE)?;
    open_document(&server, CROSS_PROJECT_CALLER_URI, CROSS_PROJECT_CALLER_MODULE)?;

    let (target_line, target_character) =
        position_of(CROSS_PROJECT_SOURCE_MODULE, "used_target {")?;
    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": CROSS_PROJECT_SOURCE_URI},
                "position": {"line": target_line, "character": target_character}
            }]
        })))?
        .ok_or("missing cross-file referenced safe-delete blocker result")?;

    assert_eq!(blocked_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(blocked_result.get("symbol").and_then(Value::as_str), Some("used_target"));
    assert_eq!(blocked_result.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_eq!(blocked_result.get("fact_source").and_then(Value::as_str), Some("workspace_index"));
    assert_eq!(blocked_result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(blocked_result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(blocked_result.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(blocked_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        blocked_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;
    assert_eq!(
        blocked_result.get("current_source_reference_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(blocked_result.get("workspace_reference_count").and_then(Value::as_u64), Some(1));
    assert!(
        blocked_result.get("current_source_delete_guard").is_none(),
        "cross-file referenced-source receipt must be workspace-reference blocked, not current-source guarded: {blocked_result}"
    );
    assert_eq!(
        blocked_result.get("workspace_reference_guard").and_then(Value::as_str),
        Some("blocked_by_workspace_reference")
    );

    let live_blocker_ux = blocked_result.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_json_array_contains(live_blocker_ux, "blocker_reasons", "ReferencesExist")?;
    assert_json_array_contains(live_blocker_ux, "blocker_messages", "still has")?;

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(request_receipt.get("symbol").and_then(Value::as_str), Some("used_target"));
    assert_eq!(request_receipt.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_eq!(request_receipt.get("fact_source").and_then(Value::as_str), Some("workspace_index"));
    assert_eq!(request_receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(request_receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}

#[test]
fn refactor_runtime_blocker_ux_safe_delete_live_pilot_blocks_dancer2_current_source_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let files = open_dancer2_workspace(&server)?;
    let response =
        files.get("lib/Dancer2/Core/Response.pm").ok_or("missing Dancer2 Core Response fixture")?;

    let (to_psgi_line, to_psgi_character) = position_of(response, "to_psgi {")?;
    let preview_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.previewSafeDelete",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": to_psgi_line, "character": to_psgi_character}
            }]
        })))?
        .ok_or("missing Dancer2 safe-delete edit-freshness preview result")?;

    assert_eq!(preview_result.get("decision").and_then(Value::as_str), Some("allowed"));
    assert_eq!(preview_result.get("symbol").and_then(Value::as_str), Some("to_psgi"));
    assert_eq!(preview_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_safe_delete_decision_trace(&preview_result, "allowed", "compiler_allowed", "none")?;

    let updated_response = response.replace(
        "sub is_forwarded",
        "sub as_array {\n    my $self = shift;\n    return $self->to_psgi;\n}\n\nsub is_forwarded",
    );
    change_document(&server, DANCER2_RESPONSE_URI, 2, &updated_response)?;

    let blocked_result = server
        .handle_execute_command(Some(json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": DANCER2_RESPONSE_URI},
                "position": {"line": to_psgi_line, "character": to_psgi_character}
            }]
        })))?
        .ok_or("missing Dancer2 safe-delete edit-freshness blocker result")?;

    assert_eq!(blocked_result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(
        blocked_result.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(blocked_result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(blocked_result.get("symbol").and_then(Value::as_str), Some("to_psgi"));
    assert_eq!(blocked_result.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_eq!(blocked_result.get("fact_source").and_then(Value::as_str), Some("current_source"));
    assert_eq!(blocked_result.get("fallback_state").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(
        blocked_result.get("current_source_reference_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        blocked_result.get("live_symbol_delete_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(blocked_result.get("edits_applied").and_then(Value::as_bool), Some(false));
    assert_eq!(
        blocked_result.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        blocked_result
            .pointer("/workspace_edit/changes")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        Some(0)
    );
    assert_safe_delete_decision_trace(&blocked_result, "blocked", "references_exist", "no_edit")?;

    let message = blocked_result
        .get("user_message")
        .and_then(Value::as_str)
        .ok_or("missing Dancer2 safe-delete blocked user_message")?;
    assert!(
        message.contains("Safe delete refused")
            && message.contains("to_psgi")
            && message.contains("No edits were returned"),
        "blocked message should explain the current-source reference without edits: {message}"
    );

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    let request_receipt = request_receipt(&explanation)?;
    assert_eq!(
        request_receipt.get("provider_action").and_then(Value::as_str),
        Some("perl.safeDeleteSymbol")
    );
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(request_receipt.get("reason").and_then(Value::as_str), Some("references_exist"));
    assert_eq!(request_receipt.get("fact_source").and_then(Value::as_str), Some("current_source"));
    assert_eq!(
        request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64),
        Some(0)
    );

    Ok(())
}
