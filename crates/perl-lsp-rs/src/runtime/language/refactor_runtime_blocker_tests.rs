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

const DANCER2_DSL_URI: &str = "file:///workspace/lib/Dancer2/Core/DSL.pm";
const DANCER2_APP_URI: &str = "file:///workspace/lib/Dancer2/Core/App.pm";
const DANCER2_PLUGIN_URI: &str = "file:///workspace/lib/Dancer2/Plugin.pm";
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
    assert_eq!(fallback_noise.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(fallback_noise.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_eq!(fallback_noise.get("new_name").and_then(Value::as_str), Some("renamed_helper"));
    assert_eq!(
        fallback_noise.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        fallback_noise.get("fallback_state").and_then(Value::as_str),
        Some("compiler_empty")
    );
    let live_edit_count = fallback_noise
        .get("live_provider_edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing live_provider_edit_count")?;
    let compiler_edit_count = fallback_noise
        .get("compiler_plan_edit_count")
        .and_then(Value::as_u64)
        .ok_or("missing compiler_plan_edit_count")?;
    assert_eq!(
        compiler_edit_count, 0,
        "package/compiler-backed rename receipt should not promote an empty compiler plan: {fallback_noise}"
    );
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
    assert_trace_contains(compiler, "Fallback", "Low", "NotApplicable")?;
    assert_note_contains(
        compiler,
        &[
            "rename runtime blocker UX",
            "compiler_plan_edits=",
            "blocker_count=0",
            "requires_confirmation=false",
            "no live refactor behavior change",
        ],
    )?;

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

    let persisted = explain_provider_decision(&server, "rename")?;
    assert_eq!(persisted.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(persisted.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(
        persisted.get("receipt_id").and_then(Value::as_str),
        Some("runtime-rename-fallback-noise")
    );
    let persisted_request_receipt = persisted
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing persisted provider-local rename receipt")?;
    assert_eq!(
        persisted_request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_empty")
    );
    assert_eq!(
        persisted_request_receipt.get("live_provider_state").and_then(Value::as_str),
        Some("error")
    );

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
        Some("compiler_empty")
    );
    assert_eq!(request_receipt.get("compiler_plan_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        request_receipt.get("compiler_requires_confirmation").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(request_receipt.get("live_provider_state").and_then(Value::as_str), Some("error"));
    assert!(
        request_receipt.get("live_provider_error").and_then(Value::as_str).is_some_and(|message| {
            message.contains("ambiguous symbol identity") && message.contains("helper")
        }),
        "request-local receipt must preserve live fallback/noise reason: {request_receipt:?}"
    );

    let replayed = explain_provider_decision(&server, "rename")?;
    assert_eq!(replayed.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(replayed.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(
        replayed.get("receipt_id").and_then(Value::as_str),
        Some("realbaseline-rename-fallback-noise")
    );
    let replayed_request_receipt = replayed
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing persisted request-local rename receipt")?;
    assert_eq!(
        replayed_request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_empty")
    );
    assert_eq!(
        replayed_request_receipt.get("live_provider_state").and_then(Value::as_str),
        Some("error")
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
        "imported-call live provider noise should stay visible before promotion: {fallback_noise}"
    );
    assert_eq!(
        fallback_noise.get("live_provider_state").and_then(Value::as_str),
        Some("edits"),
        "unexpected imported-call live provider state: {fallback_noise}"
    );
    assert!(
        fallback_noise.get("live_provider_error").is_some_and(Value::is_null),
        "live edit-noise state should not fabricate a provider error: {fallback_noise}"
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

    let explanation = explain_provider_decision(&server, "safe_delete")?;
    assert_eq!(explanation.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(explanation.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(
        explanation.get("receipt_id").and_then(Value::as_str),
        Some("runtime-safe-delete-live-blocker-ux")
    );
    let request_receipt = explanation
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing persisted safe-delete blocker receipt")?;
    assert_eq!(request_receipt.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_json_array_contains(
        &Value::Object(request_receipt.clone()),
        "blocker_reasons",
        "ImportedSymbol",
    )?;

    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    Ok(())
}
