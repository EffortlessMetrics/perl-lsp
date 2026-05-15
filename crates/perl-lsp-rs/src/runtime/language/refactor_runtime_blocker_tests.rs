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
const REAL_BASELINE_UTIL_URI: &str = "file:///workspace/lib/RealBaseline/Util.pm";

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

fn assert_safe_delete_no_live_edit(receipt: &Value) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(receipt.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    let changes = receipt
        .get("live_provider_result")
        .and_then(|value| value.get("changes"))
        .and_then(Value::as_object)
        .ok_or("safe-delete receipt must expose live provider changes")?;
    assert!(changes.is_empty(), "safe-delete live provider must return no symbol edits");

    let rollback = receipt.get("rollback_receipt").ok_or("missing rollback_receipt")?;
    assert_eq!(rollback.get("live_provider_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(rollback.get("inverse_edit_count").and_then(Value::as_u64), Some(0));
    assert_eq!(rollback.get("restored_original").and_then(Value::as_bool), Some(true));
    let claim_boundary = rollback
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing rollback claim_boundary")?;
    assert!(
        claim_boundary.contains("symbol-level safe-delete remains blocked"),
        "rollback claim boundary must preserve safe-delete cutover boundary: {claim_boundary}"
    );
    Ok(())
}

fn assert_safe_delete_live_blocker_ux(
    receipt: &Value,
    expected_reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_safe_delete_no_live_edit(receipt)?;

    let live_blocker = receipt.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_eq!(live_blocker.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(live_blocker.get("requires_confirmation").and_then(Value::as_bool), Some(true));
    let blocker_reasons = live_blocker
        .get("blocker_reasons")
        .and_then(Value::as_array)
        .ok_or("missing blocker_reasons")?;
    assert!(
        blocker_reasons.iter().any(|reason| reason.as_str() == Some(expected_reason)),
        "expected live blocker reason `{expected_reason}`, got {blocker_reasons:?}"
    );
    let message = live_blocker.get("message").and_then(Value::as_str).ok_or("missing message")?;
    assert!(
        message.contains("Safe delete is blocked") && message.contains("no live symbol-level edit"),
        "live blocker message must explain the no-edit blocker boundary: {message}"
    );
    let claim_boundary = live_blocker
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing live blocker claim_boundary")?;
    assert!(
        claim_boundary.contains("test-only receipt")
            && claim_boundary.contains("no live symbol-level safe-delete provider cutover"),
        "live blocker claim boundary must keep safe-delete shadowed: {claim_boundary}"
    );
    Ok(())
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
    assert_safe_delete_live_blocker_ux(&runtime_receipt, "ambiguous_reference")?;
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
    assert_safe_delete_live_blocker_ux(&runtime_receipt, "stale_fact")?;
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
    assert_safe_delete_no_live_edit(&runtime_receipt)?;
    let live_blocker =
        runtime_receipt.get("live_blocker_ux").ok_or("missing live_blocker_ux")?;
    assert_eq!(
        live_blocker.get("decision").and_then(Value::as_str),
        Some("shadow_only_no_live_edit")
    );
    assert_eq!(live_blocker.get("requires_confirmation").and_then(Value::as_bool), Some(false));
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
    assert_safe_delete_live_blocker_ux(&runtime_receipt, "dynamic_boundary")?;
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
    assert_safe_delete_live_blocker_ux(&runtime_receipt, "generated_member")?;
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
    assert_safe_delete_live_blocker_ux(&compile_receipt, "stale_fact")?;
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
    assert_eq!(
        generated_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(generated_receipt.get("symbol").and_then(Value::as_str), Some("routes"));
    assert_safe_delete_live_blocker_ux(&generated_receipt, "generated_member")?;
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
    assert_eq!(dynamic_receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
    assert_eq!(dynamic_receipt.get("symbol").and_then(Value::as_str), Some("plugin_keywords"));
    assert_safe_delete_live_blocker_ux(&dynamic_receipt, "dynamic_boundary")?;
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
        low_confidence_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(low_confidence_receipt.get("symbol").and_then(Value::as_str), Some("_compile"));
    assert_safe_delete_live_blocker_ux(&low_confidence_receipt, "ambiguous_reference")?;
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
    assert_eq!(helper_receipt.get("symbol").and_then(Value::as_str), Some("helper"));
    assert_safe_delete_live_blocker_ux(&helper_receipt, "imported_symbol")?;
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
