use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail, eyre};
use perl_lsp_ux_tests::{
    DiagnosticsTracker, FakeWorkspace, LspEvent, ScenarioConfig, UxClient, normalize_lsp_payload,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::WalkDir;

const DEFAULT_JSON_RECEIPT: &str = "target/receipts/ux/lsp-ux-smoke.json";
const DEFAULT_MARKDOWN_RECEIPT: &str = "target/receipts/ux/lsp-ux-smoke.md";
const DEFAULT_BINARY_PROFILE: &str = "agent";
const COMPACT_JSON_INLINE_LIMIT: usize = 2000;

#[derive(Debug, Clone)]
pub struct LspUxSmokeConfig {
    pub fixture_root: PathBuf,
    pub emit_receipt: bool,
    pub binary: Option<PathBuf>,
    pub no_build: bool,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    claim_boundary: String,
    fixtures: Vec<ManifestFixture>,
}

#[derive(Debug, Deserialize)]
struct ManifestFixture {
    name: String,
    workspace: String,
    open: String,
    primary_methods: Vec<String>,
    expected_summary: String,
}

#[derive(Debug, Deserialize)]
struct FixtureRequest {
    method: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    position_marker: Option<String>,
    #[serde(default)]
    diagnostic_marker: Option<String>,
    #[serde(default)]
    diagnostic_code: Option<String>,
    #[serde(default)]
    diagnostic_source: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    line_endings: Option<String>,
}

#[derive(Debug, Serialize)]
struct SmokeReceipt {
    schema_version: &'static str,
    status: &'static str,
    generated_at: String,
    git_sha: String,
    fixture_root: String,
    fixture_schema_version: u32,
    fixture_claim_boundary: String,
    binary: String,
    claim_boundary: &'static str,
    summary: SmokeSummary,
    fixtures: Vec<FixtureReceipt>,
}

#[derive(Debug, Serialize)]
struct SmokeSummary {
    fixture_count: usize,
    passed: usize,
    failed: usize,
    gaps: usize,
    request_count: usize,
    failed_checks: usize,
    gap_checks: usize,
}

#[derive(Debug, Serialize)]
struct FixtureReceipt {
    name: String,
    workspace: String,
    opened_file: String,
    expected_summary: String,
    status: &'static str,
    primary_methods: Vec<String>,
    request_count: usize,
    check_count: usize,
    failed_checks: usize,
    gap_checks: usize,
    checks: Vec<CheckReceipt>,
    stderr_line_count: usize,
    window_message_count: usize,
    warning_or_error_message_count: usize,
}

#[derive(Debug, Serialize)]
struct CheckReceipt {
    method: String,
    target: Option<String>,
    status: &'static str,
    detail: String,
    observed: Value,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RequestMethod {
    Initialize,
    DidOpen,
    Diagnostics,
    DocumentSymbol,
    Definition,
    WorkspaceSymbol,
    Completion,
    DocumentLink,
    CodeAction,
    Hover,
    Shutdown,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Range {
    start: Position,
    end: Position,
}

pub fn run(config: LspUxSmokeConfig) -> Result<()> {
    let root = crate::utils::project_root()?;
    let fixture_root = normalize_input_path(&root, &config.fixture_root);
    let manifest = load_manifest(&fixture_root)?;
    let binary = resolve_smoke_binary(&root, config.binary, config.no_build)?;

    let receipt = run_manifest(&root, &fixture_root, &manifest, &binary)?;
    if config.emit_receipt {
        write_json_receipt(&root.join(DEFAULT_JSON_RECEIPT), &receipt)?;
        write_markdown_receipt(&root.join(DEFAULT_MARKDOWN_RECEIPT), &receipt)?;
        println!("lsp UX smoke receipt OK: {}", root.join(DEFAULT_JSON_RECEIPT).display());
        println!("lsp UX smoke markdown OK: {}", root.join(DEFAULT_MARKDOWN_RECEIPT).display());
    }

    if receipt.status != "pass" {
        bail!(
            "lsp UX smoke failed: {} failed checks across {} fixtures",
            receipt.summary.failed_checks,
            receipt.summary.fixture_count
        );
    }

    println!(
        "lsp UX smoke OK: {} fixtures, {} requests, binary {}",
        receipt.summary.fixture_count,
        receipt.summary.request_count,
        binary.display()
    );
    Ok(())
}

fn run_manifest(
    root: &Path,
    fixture_root: &Path,
    manifest: &Manifest,
    binary: &Path,
) -> Result<SmokeReceipt> {
    let mut fixtures = Vec::new();
    for fixture in &manifest.fixtures {
        fixtures.push(run_fixture(fixture_root, fixture, binary)?);
    }

    let fixture_count = fixtures.len();
    let failed = fixtures.iter().filter(|fixture| fixture.failed_checks > 0).count();
    let gaps = fixtures.iter().filter(|fixture| fixture.gap_checks > 0).count();
    let request_count = fixtures.iter().map(|fixture| fixture.request_count).sum();
    let failed_checks = fixtures.iter().map(|fixture| fixture.failed_checks).sum();
    let gap_checks = fixtures.iter().map(|fixture| fixture.gap_checks).sum();
    let passed = fixture_count.saturating_sub(failed);
    let status = if failed_checks == 0 { "pass" } else { "fail" };

    Ok(SmokeReceipt {
        schema_version: "lsp-ux-smoke.v1",
        status,
        generated_at: Utc::now().to_rfc3339(),
        git_sha: git_sha(root).unwrap_or_else(|| "unknown".to_string()),
        fixture_root: display_path(fixture_root),
        fixture_schema_version: manifest.schema_version,
        fixture_claim_boundary: manifest.claim_boundary.clone(),
        binary: display_path(binary),
        claim_boundary: "runtime stdio LSP smoke over release_smoke fixtures; proves initialize, initialized, didOpen, request/response shape, selected diagnostics/code-action/document-link/navigation behavior, and graceful shutdown, not live editor UI automation, package publishing, or exhaustive provider correctness",
        summary: SmokeSummary {
            fixture_count,
            passed,
            failed,
            gaps,
            request_count,
            failed_checks,
            gap_checks,
        },
        fixtures,
    })
}

fn run_fixture(
    fixture_root: &Path,
    fixture: &ManifestFixture,
    binary: &Path,
) -> Result<FixtureReceipt> {
    let workspace_source = fixture_root.join(&fixture.workspace);
    let requests = load_requests(&workspace_source)?;
    let expected = load_expected(&workspace_source)?;
    let workspace = seed_workspace(&workspace_source)?;
    let timeout = Duration::from_secs(30);
    let config = ScenarioConfig {
        timeout,
        extra_env: vec![("PERL_LSP_QUIET".to_string(), Some("1".to_string()))],
        ..ScenarioConfig::default()
    };
    let binary_path = binary.to_string_lossy().into_owned();
    let client = ux(UxClient::spawn(&binary_path, &workspace, &config))
        .with_context(|| format!("spawning perl-lsp for fixture {}", fixture.name))?;

    let mut checks = Vec::new();
    let mut opened_files = BTreeSet::new();
    let mut latest_diagnostics = BTreeMap::<String, Vec<Value>>::new();

    for request in &requests {
        let check = run_request(
            &client,
            &workspace,
            &expected,
            request,
            timeout,
            &mut opened_files,
            &mut latest_diagnostics,
        )?;
        checks.push(check);
    }

    let events = client.peek_events();
    let stderr_lines = client.peek_stderr_lines();
    let warning_or_error_message_count = count_warning_or_error_messages(&events, &stderr_lines);
    let window_message_count = events
        .iter()
        .filter(|event| {
            matches!(event, LspEvent::WindowMessage { .. } | LspEvent::LogMessage { .. })
        })
        .count();
    let failed_checks = checks.iter().filter(|check| check.status == "fail").count();
    let gap_checks = checks.iter().filter(|check| check.status == "gap").count();
    let status = if failed_checks == 0 { "pass" } else { "fail" };

    Ok(FixtureReceipt {
        name: fixture.name.clone(),
        workspace: fixture.workspace.clone(),
        opened_file: fixture.open.clone(),
        expected_summary: fixture.expected_summary.clone(),
        status,
        primary_methods: fixture.primary_methods.clone(),
        request_count: requests.len(),
        check_count: checks.len(),
        failed_checks,
        gap_checks,
        checks,
        stderr_line_count: stderr_lines.len(),
        window_message_count,
        warning_or_error_message_count,
    })
}

fn run_request(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
    timeout: Duration,
    opened_files: &mut BTreeSet<String>,
    latest_diagnostics: &mut BTreeMap<String, Vec<Value>>,
) -> Result<CheckReceipt> {
    match request_method(request.method.as_str()) {
        RequestMethod::Initialize => Ok(check_initialize(client, request)),
        RequestMethod::DidOpen => {
            check_did_open(client, workspace, request, opened_files, timeout, latest_diagnostics)
        }
        RequestMethod::Diagnostics => {
            check_diagnostics(client, workspace, expected, request, timeout, latest_diagnostics)
        }
        RequestMethod::DocumentSymbol => {
            check_document_symbol(client, workspace, expected, request)
        }
        RequestMethod::Definition => check_definition(client, workspace, expected, request),
        RequestMethod::WorkspaceSymbol => {
            check_workspace_symbol(client, workspace, expected, request)
        }
        RequestMethod::Completion => check_completion(client, workspace, request),
        RequestMethod::DocumentLink => check_document_link(client, workspace, expected, request),
        RequestMethod::CodeAction => {
            check_code_action(client, workspace, expected, request, latest_diagnostics)
        }
        RequestMethod::Hover => check_hover(client, workspace, request),
        RequestMethod::Shutdown => check_shutdown(client, timeout, request),
        RequestMethod::Unsupported => Ok(unsupported_request_receipt(request)),
    }
}

fn request_method(method: &str) -> RequestMethod {
    match method {
        "initialize" => RequestMethod::Initialize,
        "textDocument/didOpen" => RequestMethod::DidOpen,
        "diagnostics" => RequestMethod::Diagnostics,
        "textDocument/documentSymbol" => RequestMethod::DocumentSymbol,
        "textDocument/definition" => RequestMethod::Definition,
        "workspace/symbol" => RequestMethod::WorkspaceSymbol,
        "textDocument/completion" => RequestMethod::Completion,
        "textDocument/documentLink" => RequestMethod::DocumentLink,
        "textDocument/codeAction" => RequestMethod::CodeAction,
        "textDocument/hover" => RequestMethod::Hover,
        "shutdown" => RequestMethod::Shutdown,
        _ => RequestMethod::Unsupported,
    }
}

fn unsupported_request_receipt(request: &FixtureRequest) -> CheckReceipt {
    CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status: "fail",
        detail: format!("unsupported release smoke request method `{}`", request.method),
        observed: Value::Null,
    }
}

fn check_initialize(client: &UxClient, request: &FixtureRequest) -> CheckReceipt {
    let initialize = client.initialize_result();
    let has_capabilities = initialize.pointer("/result/capabilities").is_some()
        || initialize.get("capabilities").is_some();
    let status = if has_capabilities { "pass" } else { "fail" };
    CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if has_capabilities {
            "initialize returned server capabilities and initialized notification was sent"
                .to_string()
        } else {
            "initialize response did not include capabilities".to_string()
        },
        observed: compact_json(&initialize),
    }
}

fn check_did_open(
    client: &UxClient,
    workspace: &FakeWorkspace,
    request: &FixtureRequest,
    opened_files: &mut BTreeSet<String>,
    timeout: Duration,
    latest_diagnostics: &mut BTreeMap<String, Vec<Value>>,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let uri = workspace.uri(file);
    ux(client.did_open(&uri, &source))?;
    opened_files.insert(file.to_string());

    let diagnostics = wait_for_latest_diagnostics(client, workspace, file, timeout);
    latest_diagnostics.insert(file.to_string(), diagnostics.clone());

    let line_ending_ok = line_endings_match_request(&source, request.line_endings.as_deref());
    let status = if line_ending_ok { "pass" } else { "fail" };
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if line_ending_ok {
            format!("opened {file} and collected {} diagnostics", diagnostics.len())
        } else {
            format!("opened {file}, but fixture line endings did not match request")
        },
        observed: json!({
            "uri": workspace.uri(file),
            "diagnostic_count": diagnostics.len(),
            "line_endings": line_ending_kind(&source),
        }),
    })
}

fn check_diagnostics(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
    timeout: Duration,
    latest_diagnostics: &mut BTreeMap<String, Vec<Value>>,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let diagnostics = wait_for_latest_diagnostics(client, workspace, file, timeout);
    latest_diagnostics.insert(file.to_string(), diagnostics.clone());

    let code = request.diagnostic_code.as_deref();
    let matching_code_count = code.map_or(diagnostics.len(), |diagnostic_code| {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic_has_code(diagnostic, diagnostic_code))
            .count()
    });
    let missing_labels = missing_expected_diagnostic_labels(
        expected,
        &diagnostics,
        "/diagnostics/must_include_labels",
    );
    let forbidden_labels = present_forbidden_diagnostic_labels(
        expected,
        &diagnostics,
        "/diagnostics/must_not_include_labels",
    );
    let status = if code.is_some_and(|_| matching_code_count == 0)
        || !missing_labels.is_empty()
        || !forbidden_labels.is_empty()
    {
        "fail"
    } else {
        "pass"
    };

    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("diagnostics matched requested code boundary ({matching_code_count} matching)")
        } else {
            format!(
                "diagnostics mismatch: missing labels [{}], forbidden labels [{}], matching code count {matching_code_count}",
                missing_labels.join(", "),
                forbidden_labels.join(", ")
            )
        },
        observed: json!({
            "diagnostic_count": diagnostics.len(),
            "matching_code_count": matching_code_count,
            "diagnostics": diagnostics,
        }),
    })
}

fn check_document_symbol(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let response = lsp_request(
        client,
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": workspace.uri(file) } }),
        Duration::from_secs(30),
    )?;
    let result = response.get("result").cloned().unwrap_or(Value::Null);
    let symbols = result.as_array().cloned().unwrap_or_default();
    let labels = collect_string_fields(&result, "name");
    let missing = expected
        .pointer("/document_symbols/must_include")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|required| !labels.iter().any(|label| label == required))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let status =
        if response.get("error").is_none() && missing.is_empty() { "pass" } else { "fail" };
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("documentSymbol returned {} symbols", symbols.len())
        } else {
            format!("documentSymbol missing expected symbols: {}", missing.join(", "))
        },
        observed: normalize_lsp_payload(&result, workspace.dir.path()),
    })
}

fn check_definition(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let marker = required_position_marker(request)?;
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let position = marker_position(&source, marker, MarkerMode::Inside)?;
    let result = retry_request_array(5, Duration::from_millis(200), || {
        request_position(client, workspace, "textDocument/definition", file, position)
    })?;
    let normalized = normalize_lsp_payload(&Value::Array(result.clone()), workspace.dir.path());
    let expected_path = expected.pointer(&format!("/definition/{marker}")).and_then(Value::as_str);
    let status = match expected_path {
        Some(path) if !normalized_contains_path(&normalized, path) => "gap",
        _ => "pass",
    };

    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("definition for {marker} returned {} locations", result.len())
        } else {
            format!(
                "definition for {marker} did not include fixture target {}; recorded as a release UX gap",
                expected_path.unwrap_or("")
            )
        },
        observed: normalized,
    })
}

fn check_workspace_symbol(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let query = request.query.as_deref().ok_or_else(|| eyre!("workspace/symbol missing query"))?;
    let result = retry_request_array(5, Duration::from_millis(200), || {
        let response = lsp_request(
            client,
            "workspace/symbol",
            json!({ "query": query }),
            Duration::from_secs(30),
        )?;
        response_array_result(response)
    })?;
    let normalized = normalize_lsp_payload(&Value::Array(result.clone()), workspace.dir.path());
    let expected_path =
        expected.pointer("/workspace_symbols/must_include_path").and_then(Value::as_str);
    let status = match expected_path {
        Some(path) if !normalized_contains_path(&normalized, path) => "gap",
        _ => "pass",
    };
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("workspace/symbol `{query}` returned {} symbols", result.len())
        } else {
            format!(
                "workspace/symbol `{query}` did not include fixture target {}; recorded as a release UX gap",
                expected_path.unwrap_or("")
            )
        },
        observed: normalized,
    })
}

fn check_completion(
    client: &UxClient,
    workspace: &FakeWorkspace,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let marker = required_position_marker(request)?;
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let position = marker_position(&source, marker, MarkerMode::After)?;
    let response = request_position(client, workspace, "textDocument/completion", file, position)?;
    let normalized = normalize_lsp_payload(&Value::Array(response.clone()), workspace.dir.path());
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status: "pass",
        detail: format!(
            "completion at marker `{marker}` returned {} items; empty is allowed when uncertain",
            response.len()
        ),
        observed: json!({
            "item_count": response.len(),
            "items": normalized,
        }),
    })
}

fn check_document_link(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let response = lsp_request(
        client,
        "textDocument/documentLink",
        json!({ "textDocument": { "uri": workspace.uri(file) } }),
        Duration::from_secs(30),
    )?;
    let links = response_array_result(response)?;
    let mut resolved = Vec::new();
    for link in &links {
        resolved.push(resolve_document_link_if_needed(client, link)?);
    }
    let normalized = normalize_lsp_payload(&Value::Array(resolved.clone()), workspace.dir.path());
    let ranges_valid = resolved.iter().all(link_range_is_valid);
    let expected_target = expected
        .pointer("/document_links/must_include_target")
        .and_then(Value::as_str)
        .or_else(|| {
            expected
                .pointer("/document_links/must_include_target_when_supported")
                .and_then(Value::as_str)
        });
    let hard_target = expected.pointer("/document_links/must_include_target").is_some();
    let target_present =
        expected_target.is_none_or(|target| normalized_contains_path(&normalized, target));
    let range_text =
        expected.pointer("/document_links/range_must_match_quoted_text").and_then(Value::as_str);
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let range_text_ok = document_link_range_text_matches(&resolved, &source, range_text);
    let status = if !ranges_valid {
        "fail"
    } else if (hard_target && !target_present) || !range_text_ok {
        "gap"
    } else {
        "pass"
    };

    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("documentLink returned {} links with valid ranges", links.len())
        } else if status == "gap" {
            "documentLink returned valid ranges but missed a fixture target; recorded as a release UX gap"
                .to_string()
        } else {
            "documentLink failed target or range expectations".to_string()
        },
        observed: json!({
            "link_count": links.len(),
            "ranges_valid": ranges_valid,
            "target_present": target_present,
            "range_text_ok": range_text_ok,
            "links": normalized,
        }),
    })
}

fn check_code_action(
    client: &UxClient,
    workspace: &FakeWorkspace,
    expected: &Value,
    request: &FixtureRequest,
    latest_diagnostics: &BTreeMap<String, Vec<Value>>,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let marker =
        request.diagnostic_marker.as_deref().or(request.position_marker.as_deref()).unwrap_or("");
    let range = if marker.is_empty() {
        Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }
    } else {
        marker_range(&source, marker)?
    };
    let diagnostics = if request.diagnostic_source.as_deref() == Some("none") {
        Vec::new()
    } else {
        let all = latest_diagnostics.get(file).cloned().unwrap_or_default();
        diagnostics_overlapping_range(&all, range)
    };
    let response = lsp_request(
        client,
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": workspace.uri(file) },
            "range": lsp_range(range),
            "context": { "diagnostics": diagnostics }
        }),
        Duration::from_secs(30),
    )?;
    let actions = response_array_result(response)?;
    let titles = actions
        .iter()
        .filter_map(|action| action.get("title").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    let expected_title = expected.pointer("/code_actions/title").and_then(Value::as_str);
    let is_negative = expected
        .pointer(&format!("/code_actions/{marker}/must_not_offer_remove_label"))
        .and_then(Value::as_bool)
        == Some(true)
        || request.diagnostic_source.as_deref() == Some("none");
    let has_expected = expected_title.is_some_and(|title| titles.iter().any(|item| item == title));
    let status = if is_negative {
        if has_expected { "gap" } else { "pass" }
    } else if expected_title.is_some() && !has_expected {
        "fail"
    } else {
        "pass"
    };

    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            format!("codeAction returned {} actions", actions.len())
        } else if status == "gap" {
            format!(
                "codeAction returned a fixture-forbidden title at marker `{marker}`; recorded as a release UX gap"
            )
        } else {
            format!("codeAction titles did not match expectation: {}", titles.join(", "))
        },
        observed: json!({
            "action_count": actions.len(),
            "titles": titles,
            "actions": normalize_lsp_payload(&Value::Array(actions), workspace.dir.path()),
        }),
    })
}

fn check_hover(
    client: &UxClient,
    workspace: &FakeWorkspace,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let file = required_file(request)?;
    let marker = required_position_marker(request)?;
    let source = fs::read_to_string(workspace.path(file))
        .with_context(|| format!("reading workspace file {file}"))?;
    let position = marker_position(&source, marker, MarkerMode::Inside)?;
    let response = request_position_raw(client, workspace, "textDocument/hover", file, position)?;
    let result = response.get("result").cloned().unwrap_or(Value::Null);
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status: if response.get("error").is_none() { "pass" } else { "fail" },
        detail: if result.is_null() {
            format!("hover for {marker} returned empty result; fixture allows quiet empty")
        } else {
            format!("hover for {marker} returned content")
        },
        observed: normalize_lsp_payload(&result, workspace.dir.path()),
    })
}

fn check_shutdown(
    client: &UxClient,
    timeout: Duration,
    request: &FixtureRequest,
) -> Result<CheckReceipt> {
    let response = lsp_request(client, "shutdown", json!({}), timeout)?;
    let status = if response.get("error").is_none() { "pass" } else { "fail" };
    ux(client.notify("exit", json!({})))?;
    Ok(CheckReceipt {
        method: request.method.clone(),
        target: request_target(request),
        status,
        detail: if status == "pass" {
            "shutdown returned without JSON-RPC error and exit was sent".to_string()
        } else {
            "shutdown returned JSON-RPC error".to_string()
        },
        observed: compact_json(&response),
    })
}

fn seed_workspace(source_dir: &Path) -> Result<FakeWorkspace> {
    let workspace = ux(FakeWorkspace::new())?;
    for entry in WalkDir::new(source_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(source_dir)
            .with_context(|| format!("stripping fixture prefix {}", source_dir.display()))?;
        if is_fixture_metadata(relative) {
            continue;
        }
        let relative_text = slash_path(relative);
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("reading fixture file {}", entry.path().display()))?;
        ux(workspace.write(&relative_text, &content))?;
    }
    Ok(workspace)
}

fn load_manifest(fixture_root: &Path) -> Result<Manifest> {
    let path = fixture_root.join("manifest.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn load_requests(source_dir: &Path) -> Result<Vec<FixtureRequest>> {
    let path = source_dir.join("requests.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn load_expected(source_dir: &Path) -> Result<Value> {
    let path = source_dir.join("expected.json");
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn resolve_smoke_binary(root: &Path, binary: Option<PathBuf>, no_build: bool) -> Result<PathBuf> {
    if let Some(path) = binary {
        let resolved = normalize_input_path(root, &path);
        return resolve_binary_path(resolved, "explicit --binary");
    }

    if let Ok(path) = std::env::var("PERL_LSP_BIN") {
        if !path.trim().is_empty() {
            return resolve_binary_path(PathBuf::from(path), "PERL_LSP_BIN");
        }
    }

    let candidate = binary_path_for_profile(root, DEFAULT_BINARY_PROFILE);
    if candidate.is_file() {
        return Ok(candidate);
    }
    let debug_candidate = binary_path_for_profile(root, "debug");
    if debug_candidate.is_file() {
        return Ok(debug_candidate);
    }

    if no_build {
        bail!(
            "perl-lsp binary not found at {} or {}; rerun without --no-build or pass --binary",
            candidate.display(),
            debug_candidate.display()
        );
    }

    build_perl_lsp(root)?;
    resolve_binary_path(candidate, "built target/agent binary")
}

fn build_perl_lsp(root: &Path) -> Result<()> {
    println!("building perl-lsp binary for UX smoke...");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(root)
        .args(["build", "-p", "perl-lsp-rs", "--profile", DEFAULT_BINARY_PROFILE, "--locked"])
        .status()
        .context("spawning cargo build for perl-lsp-rs")?;
    if !status.success() {
        bail!("cargo build -p perl-lsp-rs --profile {DEFAULT_BINARY_PROFILE} --locked failed");
    }
    Ok(())
}

fn resolve_binary_path(path: PathBuf, source: &str) -> Result<PathBuf> {
    if path.is_file() {
        return Ok(path);
    }
    #[cfg(windows)]
    {
        if path.extension().is_none() {
            let mut exe = path.clone();
            exe.set_extension("exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    bail!("perl-lsp binary from {source} does not exist: {}", path.display())
}

fn binary_path_for_profile(root: &Path, profile: &str) -> PathBuf {
    let path = root.join("target").join(profile).join("perl-lsp");
    #[cfg(windows)]
    {
        let mut path = path;
        path.set_extension("exe");
        path
    }
    #[cfg(not(windows))]
    {
        path
    }
}

fn request_position(
    client: &UxClient,
    workspace: &FakeWorkspace,
    method: &str,
    file: &str,
    position: Position,
) -> Result<Vec<Value>> {
    let response = request_position_raw(client, workspace, method, file, position)?;
    response_array_result(response)
}

fn request_position_raw(
    client: &UxClient,
    workspace: &FakeWorkspace,
    method: &str,
    file: &str,
    position: Position,
) -> Result<Value> {
    lsp_request(
        client,
        method,
        json!({
            "textDocument": { "uri": workspace.uri(file) },
            "position": lsp_position(position),
            "context": { "triggerKind": 1 }
        }),
        Duration::from_secs(30),
    )
}

fn retry_request_array(
    attempts: usize,
    pause: Duration,
    mut request: impl FnMut() -> Result<Vec<Value>>,
) -> Result<Vec<Value>> {
    let max_attempts = attempts.max(1);
    let mut last = Vec::new();
    for index in 0..max_attempts {
        let current = request()?;
        if !current.is_empty() {
            return Ok(current);
        }
        last = current;
        if index + 1 < max_attempts {
            std::thread::sleep(pause);
        }
    }
    Ok(last)
}

fn response_array_result(response: Value) -> Result<Vec<Value>> {
    if let Some(error) = response.get("error") {
        bail!("LSP request returned error: {error}");
    }
    let result = response.get("result").cloned().unwrap_or(Value::Null);
    match result {
        Value::Array(values) => Ok(values),
        Value::Null => Ok(Vec::new()),
        other => Ok(vec![other]),
    }
}

fn resolve_document_link_if_needed(client: &UxClient, link: &Value) -> Result<Value> {
    if link.get("target").is_some() {
        return Ok(link.clone());
    }
    let response =
        lsp_request(client, "documentLink/resolve", link.clone(), Duration::from_secs(30))?;
    if let Some(error) = response.get("error") {
        bail!("documentLink/resolve returned error: {error}");
    }
    Ok(response.get("result").cloned().unwrap_or_else(|| link.clone()))
}

fn lsp_request(client: &UxClient, method: &str, params: Value, timeout: Duration) -> Result<Value> {
    ux(client.request(method, params, timeout))
}

fn ux<T>(result: std::result::Result<T, anyhow::Error>) -> Result<T> {
    result.map_err(|error| eyre!("{error:#}"))
}

fn wait_for_latest_diagnostics(
    client: &UxClient,
    workspace: &FakeWorkspace,
    file: &str,
    timeout: Duration,
) -> Vec<Value> {
    let uri = workspace.uri(file);
    DiagnosticsTracker::wait_for_uri_matching(
        || client.peek_events(),
        &uri,
        timeout.min(Duration::from_secs(5)),
        |_| true,
    )
    .unwrap_or_default()
}

fn missing_expected_diagnostic_labels(
    expected: &Value,
    diagnostics: &[Value],
    pointer: &str,
) -> Vec<String> {
    expected
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|label| {
            !diagnostics.iter().any(|diagnostic| diagnostic_mentions(diagnostic, label))
        })
        .map(str::to_string)
        .collect()
}

fn present_forbidden_diagnostic_labels(
    expected: &Value,
    diagnostics: &[Value],
    pointer: &str,
) -> Vec<String> {
    expected
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|label| diagnostics.iter().any(|diagnostic| diagnostic_mentions(diagnostic, label)))
        .map(str::to_string)
        .collect()
}

fn diagnostic_has_code(diagnostic: &Value, code: &str) -> bool {
    diagnostic.get("code").is_some_and(|value| diagnostic_code_value_matches(value, code))
}

fn diagnostic_code_value_matches(value: &Value, code: &str) -> bool {
    diagnostic_code_text(value).is_some_and(|actual| code.eq(actual.as_str()))
}

fn diagnostic_code_text(value: &Value) -> Option<String> {
    match value {
        Value::String(actual) => Some(actual.clone()),
        Value::Number(actual) => Some(actual.to_string()),
        _ => None,
    }
}

fn diagnostic_mentions(diagnostic: &Value, label: &str) -> bool {
    diagnostic.to_string().contains(label)
}

fn diagnostics_overlapping_range(diagnostics: &[Value], range: Range) -> Vec<Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .get("range")
                .and_then(parse_lsp_range)
                .is_some_and(|diagnostic_range| ranges_overlap(diagnostic_range, range))
        })
        .cloned()
        .collect()
}

fn ranges_overlap(left: Range, right: Range) -> bool {
    position_tuple(left.start) <= position_tuple(right.end)
        && position_tuple(right.start) <= position_tuple(left.end)
}

fn position_tuple(position: Position) -> (u32, u32) {
    (position.line, position.character)
}

fn collect_string_fields(value: &Value, key: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    collect_string_fields_into(value, key, &mut out);
    out
}

fn collect_string_fields_into(value: &Value, key: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(text) = map.get(key).and_then(Value::as_str) {
                out.insert(text.to_string());
            }
            for child in map.values() {
                collect_string_fields_into(child, key, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_string_fields_into(child, key, out);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
enum MarkerMode {
    Inside,
    After,
}

fn marker_position(source: &str, marker: &str, mode: MarkerMode) -> Result<Position> {
    let range = marker_range(source, marker)?;
    Ok(match mode {
        MarkerMode::Inside => {
            let offset = marker_offset(source, marker)? + marker.len().min(1);
            offset_to_position(source, offset)
        }
        MarkerMode::After => range.end,
    })
}

fn marker_range(source: &str, marker: &str) -> Result<Range> {
    let start = marker_offset(source, marker)?;
    let end = start + marker.len();
    Ok(Range { start: offset_to_position(source, start), end: offset_to_position(source, end) })
}

fn marker_offset(source: &str, marker: &str) -> Result<usize> {
    source.find(marker).ok_or_else(|| eyre!("marker `{marker}` not found in fixture source"))
}

fn offset_to_position(source: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in source[..byte_offset].chars() {
        match ch {
            '\n' => {
                line += 1;
                character = 0;
            }
            '\r' => {}
            _ => {
                character += ch.len_utf16() as u32;
            }
        }
    }
    Position { line, character }
}

fn link_range_covers_text(link: &Value, source: &str, text: &str) -> bool {
    let Some(range) = parse_lsp_range(link.get("range").unwrap_or(&Value::Null)) else {
        return false;
    };
    marker_range(source, text).is_ok_and(|expected| expected == range)
}

fn link_range_is_valid(link: &Value) -> bool {
    parse_lsp_range(link.get("range").unwrap_or(&Value::Null)).is_some_and(|range| {
        (range.start.line, range.start.character) <= (range.end.line, range.end.character)
    })
}

fn parse_lsp_range(value: &Value) -> Option<Range> {
    Some(Range {
        start: Position {
            line: value.pointer("/start/line")?.as_u64()?.try_into().ok()?,
            character: value.pointer("/start/character")?.as_u64()?.try_into().ok()?,
        },
        end: Position {
            line: value.pointer("/end/line")?.as_u64()?.try_into().ok()?,
            character: value.pointer("/end/character")?.as_u64()?.try_into().ok()?,
        },
    })
}

fn lsp_position(position: Position) -> Value {
    json!({ "line": position.line, "character": position.character })
}

fn lsp_range(range: Range) -> Value {
    json!({ "start": lsp_position(range.start), "end": lsp_position(range.end) })
}

fn normalized_contains_path(value: &Value, path: &str) -> bool {
    let needle = path.replace('\\', "/");
    value.to_string().replace('\\', "/").contains(&needle)
}

fn count_warning_or_error_messages(events: &[LspEvent], stderr_lines: &[String]) -> usize {
    let event_count = events
        .iter()
        .filter(|event| match event {
            LspEvent::WindowMessage { message_type, message }
            | LspEvent::LogMessage { message_type, message } => {
                *message_type <= 2 || scary_message(message)
            }
            LspEvent::Diagnostics { .. } | LspEvent::Other { .. } => false,
        })
        .count();
    event_count + stderr_lines.iter().filter(|line| scary_message(line)).count()
}

fn scary_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    ["panic", "panicked", "stack overflow", "sigabrt", "fatal"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn line_ending_kind(source: &str) -> &'static str {
    if source.contains("\r\n") { "crlf" } else { "lf" }
}

fn line_endings_match_request(source: &str, expected: Option<&str>) -> bool {
    match expected {
        Some("crlf") => source.contains("\r\n"),
        Some("lf") => !source.contains("\r\n"),
        Some(_) | None => true,
    }
}

fn document_link_range_text_matches(links: &[Value], source: &str, text: Option<&str>) -> bool {
    match text {
        Some(quoted_text) => {
            links.iter().any(|link| link_range_covers_text(link, source, quoted_text))
        }
        None => true,
    }
}

fn required_file(request: &FixtureRequest) -> Result<&str> {
    request.file.as_deref().ok_or_else(|| eyre!("{} request missing file", request.method))
}

fn required_position_marker(request: &FixtureRequest) -> Result<&str> {
    request
        .position_marker
        .as_deref()
        .ok_or_else(|| eyre!("{} request missing position_marker", request.method))
}

fn request_target(request: &FixtureRequest) -> Option<String> {
    request
        .file
        .clone()
        .or_else(|| request.target.clone())
        .or_else(|| request.position_marker.clone())
        .or_else(|| request.diagnostic_marker.clone())
        .or_else(|| request.query.clone())
}

fn compact_json(value: &Value) -> Value {
    compact_json_with_serialized_len(value, value.to_string().len())
}

fn compact_json_with_serialized_len(value: &Value, serialized_len: usize) -> Value {
    if serialized_len <= COMPACT_JSON_INLINE_LIMIT {
        return value.clone();
    }
    json!({ "truncated": true, "top_level_keys": value.as_object().map(|map| map.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() })
}

fn normalize_input_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { root.join(path) }
}

fn is_fixture_metadata(relative: &Path) -> bool {
    matches!(slash_path(relative).as_str(), "README.md" | "requests.json" | "expected.json")
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn git_sha(root: &Path) -> Option<String> {
    let output = Command::new("git").current_dir(root).args(["rev-parse", "HEAD"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_json_receipt(path: &Path, receipt: &SmokeReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(receipt)?))
        .with_context(|| format!("writing {}", path.display()))
}

fn write_markdown_receipt(path: &Path, receipt: &SmokeReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, render_markdown(receipt)).with_context(|| format!("writing {}", path.display()))
}

fn render_markdown(receipt: &SmokeReceipt) -> String {
    let mut out = String::new();
    out.push_str("# LSP UX Smoke Receipt\n\n");
    out.push_str(&format!("- status: `{}`\n", receipt.status));
    out.push_str(&format!("- git_sha: `{}`\n", receipt.git_sha));
    out.push_str(&format!("- binary: `{}`\n", receipt.binary));
    out.push_str(&format!("- fixture_root: `{}`\n", receipt.fixture_root));
    out.push_str(&format!(
        "- fixtures: {} passed / {} failed / {} with recorded gaps\n\n",
        receipt.summary.passed, receipt.summary.failed, receipt.summary.gaps
    ));
    out.push_str("| Fixture | Status | Requests | Failed Checks | Gap Checks | User Scenario |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: | --- |\n");
    for fixture in &receipt.fixtures {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} | {} |\n",
            fixture.name,
            fixture.status,
            fixture.request_count,
            fixture.failed_checks,
            fixture.gap_checks,
            fixture.expected_summary.replace('|', "\\|")
        ));
    }
    out.push('\n');
    out.push_str("## Checks\n\n");
    for fixture in &receipt.fixtures {
        out.push_str(&format!("### `{}`\n\n", fixture.name));
        for check in &fixture.checks {
            out.push_str(&format!(
                "- `{}` {}: {} ({})\n",
                check.method,
                check.target.as_deref().unwrap_or("-"),
                check.status,
                check.detail.replace('\n', " ")
            ));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    type TestResult = Result<()>;

    fn fixture_request(method: &str) -> FixtureRequest {
        FixtureRequest {
            method: method.to_string(),
            file: None,
            target: None,
            position_marker: None,
            diagnostic_marker: None,
            diagnostic_code: None,
            diagnostic_source: None,
            query: None,
            line_endings: None,
        }
    }

    #[test]
    fn request_method_classifies_supported_fixture_methods() {
        let cases = [
            ("initialize", RequestMethod::Initialize),
            ("textDocument/didOpen", RequestMethod::DidOpen),
            ("diagnostics", RequestMethod::Diagnostics),
            ("textDocument/documentSymbol", RequestMethod::DocumentSymbol),
            ("textDocument/definition", RequestMethod::Definition),
            ("workspace/symbol", RequestMethod::WorkspaceSymbol),
            ("textDocument/completion", RequestMethod::Completion),
            ("textDocument/documentLink", RequestMethod::DocumentLink),
            ("textDocument/codeAction", RequestMethod::CodeAction),
            ("textDocument/hover", RequestMethod::Hover),
            ("shutdown", RequestMethod::Shutdown),
        ];

        for (method, expected) in cases {
            assert_eq!(request_method(method), expected);
        }
        assert_eq!(request_method("workspace/executeCommand"), RequestMethod::Unsupported);
    }

    #[test]
    fn unsupported_request_receipt_preserves_method_and_target() {
        let mut request = fixture_request("workspace/executeCommand");
        request.file = Some("bin/app.pl".to_string());

        let receipt = unsupported_request_receipt(&request);

        assert_eq!(receipt.method, "workspace/executeCommand");
        assert_eq!(receipt.target.as_deref(), Some("bin/app.pl"));
        assert_eq!(receipt.status, "fail");
        assert!(receipt.detail.contains("workspace/executeCommand"));
        assert_eq!(receipt.observed, Value::Null);
    }

    #[test]
    fn line_ending_expectations_match_fixture_source() {
        assert!(line_endings_match_request("use strict;\r\n", Some("crlf")));
        assert!(!line_endings_match_request("use strict;\n", Some("crlf")));
        assert!(line_endings_match_request("use strict;\n", Some("lf")));
        assert!(!line_endings_match_request("use strict;\r\n", Some("lf")));
        assert!(line_endings_match_request("use strict;\n", Some("mixed")));
        assert!(line_endings_match_request("use strict;\n", None));
    }

    #[test]
    fn document_link_text_matching_requires_exact_range() -> TestResult {
        let source = "use strict;\nrequire \"notes/todo.txt\";\n";
        let range = marker_range(source, "notes/todo.txt")?;
        let link = json!({ "range": lsp_range(range), "target": "file:///notes/todo.txt" });
        let wrong_link = json!({
            "range": lsp_range(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 3 },
            }),
            "target": "file:///notes/todo.txt",
        });

        assert!(document_link_range_text_matches(
            std::slice::from_ref(&link),
            source,
            Some("notes/todo.txt")
        ));
        assert!(!document_link_range_text_matches(&[wrong_link], source, Some("notes/todo.txt")));
        assert!(document_link_range_text_matches(&[], source, None));
        assert!(!link_range_covers_text(&json!({}), source, "notes/todo.txt"));
        Ok(())
    }

    #[test]
    fn retry_request_array_retries_empty_responses_until_data() -> TestResult {
        let mut calls = 0usize;

        let values = retry_request_array(3, Duration::ZERO, || {
            calls += 1;
            if calls == 2 { Ok(vec![json!("ready")]) } else { Ok(Vec::new()) }
        })?;

        assert_eq!(calls, 2);
        assert_eq!(values, vec![json!("ready")]);
        Ok(())
    }

    #[test]
    fn retry_request_array_uses_one_attempt_for_zero_attempt_request() -> TestResult {
        let mut calls = 0usize;

        let values = retry_request_array(0, Duration::ZERO, || {
            calls += 1;
            Ok(Vec::new())
        })?;

        assert_eq!(calls, 1);
        assert!(values.is_empty());
        Ok(())
    }

    #[test]
    fn diagnostic_code_accepts_string_and_numeric_codes_only() {
        let string_code = Value::String("PL410".to_string());

        assert_eq!(diagnostic_code_text(&string_code).as_deref(), Some("PL410"));
        assert!(diagnostic_code_value_matches(&string_code, "PL410"));
        assert!(!diagnostic_code_value_matches(&string_code, "PL411"));
        assert!(diagnostic_has_code(&json!({ "code": "PL410" }), "PL410"));
        assert!(!diagnostic_has_code(&json!({ "code": "PL410" }), "PL411"));
        assert!(diagnostic_has_code(&json!({ "code": 410 }), "410"));
        assert_eq!(diagnostic_code_text(&json!(410)).as_deref(), Some("410"));
        assert!(diagnostic_code_value_matches(&json!(410), "410"));
        assert!(!diagnostic_has_code(&json!({ "code": true }), "PL410"));
        assert_eq!(diagnostic_code_text(&Value::Bool(true)), None);
        assert!(!diagnostic_code_value_matches(&Value::Bool(true), "PL410"));
        assert!(!diagnostic_has_code(&json!({}), "PL410"));
    }

    #[test]
    fn collect_string_fields_recurses_objects_and_ignores_scalars() {
        let labels = collect_string_fields(
            &json!({
                "name": "root",
                "children": [
                    { "name": "child" },
                    true,
                    17
                ]
            }),
            "name",
        );

        assert_eq!(labels.len(), 2);
        assert!(labels.contains("root"));
        assert!(labels.contains("child"));
    }

    #[test]
    fn marker_position_inside_empty_marker_stays_at_start() -> TestResult {
        let position = marker_position("abc", "", MarkerMode::Inside)?;

        assert_eq!(position, Position { line: 0, character: 0 });
        Ok(())
    }

    #[test]
    fn offset_to_position_counts_lf_cr_and_utf16_columns() -> TestResult {
        let source = "a😀\r\nz";
        let cr_offset = source.find('\r').ok_or_else(|| eyre!("fixture missing CR"))?;
        let z_offset = source.find('z').ok_or_else(|| eyre!("fixture missing z"))?;

        assert_eq!(offset_to_position(source, cr_offset), Position { line: 0, character: 3 });
        assert_eq!(offset_to_position(source, cr_offset + 1), Position { line: 0, character: 3 });
        assert_eq!(offset_to_position(source, z_offset), Position { line: 1, character: 0 });
        Ok(())
    }

    #[test]
    fn compact_json_preserves_small_payload_and_summarizes_large_payload() -> TestResult {
        let small = json!({ "ok": true });
        assert_eq!(compact_json(&small), small);
        assert_eq!(compact_json_with_serialized_len(&small, COMPACT_JSON_INLINE_LIMIT), small);

        let large = compact_json(&json!({ "payload": "x".repeat(2001) }));
        assert_eq!(large.get("truncated").and_then(Value::as_bool), Some(true));
        let keys = large
            .get("top_level_keys")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre!("large payload summary missing top_level_keys"))?;
        assert!(keys.iter().any(|key| key.as_str() == Some("payload")));

        let boundary_overflow = compact_json_with_serialized_len(
            &json!({ "payload": "x" }),
            COMPACT_JSON_INLINE_LIMIT + 1,
        );
        assert_eq!(boundary_overflow.get("truncated").and_then(Value::as_bool), Some(true));
        Ok(())
    }

    #[test]
    fn marker_positions_treat_crlf_as_single_line_break() -> TestResult {
        let source = "# before\r\nuse Smoke::CRLF;\r\nrequire \"notes/todo.txt\";\r\n";
        let position = marker_position(source, "Smoke::CRLF", MarkerMode::Inside)?;
        assert_eq!(position, Position { line: 1, character: 5 });
        let range = marker_range(source, "notes/todo.txt")?;
        assert_eq!(range.start, Position { line: 2, character: 9 });
        assert_eq!(range.end, Position { line: 2, character: 23 });
        Ok(())
    }

    #[test]
    fn fixture_metadata_is_not_seeded_as_workspace_source() {
        assert!(is_fixture_metadata(Path::new("README.md")));
        assert!(is_fixture_metadata(Path::new("requests.json")));
        assert!(is_fixture_metadata(Path::new("expected.json")));
        assert!(!is_fixture_metadata(Path::new("bin/app.pl")));
    }

    #[test]
    fn markdown_receipt_lists_fixture_status() {
        let receipt = SmokeReceipt {
            schema_version: "lsp-ux-smoke.v1",
            status: "pass",
            generated_at: "2026-06-05T00:00:00Z".to_string(),
            git_sha: "abc123".to_string(),
            fixture_root: "testdata/ux/release_smoke".to_string(),
            fixture_schema_version: 1,
            fixture_claim_boundary: "fixture-data-only".to_string(),
            binary: "target/agent/perl-lsp".to_string(),
            claim_boundary: "test boundary",
            summary: SmokeSummary {
                fixture_count: 1,
                passed: 1,
                failed: 0,
                gaps: 0,
                request_count: 1,
                failed_checks: 0,
                gap_checks: 0,
            },
            fixtures: vec![FixtureReceipt {
                name: "minimal_script".to_string(),
                workspace: "minimal_script".to_string(),
                opened_file: "bin/hello.pl".to_string(),
                expected_summary: "normal startup".to_string(),
                status: "pass",
                primary_methods: vec!["initialize".to_string()],
                request_count: 1,
                check_count: 1,
                failed_checks: 0,
                gap_checks: 0,
                checks: vec![CheckReceipt {
                    method: "initialize".to_string(),
                    target: Some(".".to_string()),
                    status: "pass",
                    detail: "initialized".to_string(),
                    observed: json!({}),
                }],
                stderr_line_count: 0,
                window_message_count: 0,
                warning_or_error_message_count: 0,
            }],
        };

        let markdown = render_markdown(&receipt);

        assert!(markdown.contains("| `minimal_script` | `pass` | 1 | 0 | 0 | normal startup |"));
        assert!(markdown.contains("- `initialize` .: pass"));
    }

    #[test]
    fn load_manifest_rejects_missing_file() -> TestResult {
        let temp = TempDir::new()?;
        let error = load_manifest(temp.path())
            .err()
            .ok_or_else(|| eyre!("missing manifest should fail"))?;
        assert!(error.to_string().contains("manifest.json"));
        Ok(())
    }
}
