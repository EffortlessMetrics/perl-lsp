//! Scenario 67 - checked-in golden daily-driver workload baseline.
//!
//! The manifest is deliberately broader than the current exact-support proof.
//! This runner records the observed result class and provider receipt for each
//! journey without promoting a support tier or turning a fallback into an
//! exactness claim.

use anyhow::{Context, Result, anyhow, ensure};
use perl_lsp_ux_tests::{
    ProjectFixtureFile, ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available,
    fixture_content, fixture_scenario_config, load_catalyst_fixture_files,
    load_dancer2_fixture_files, load_mojolicious_fixture_files, missing_binary_skip,
    open_all_fixture_files, run_ux_scenario,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use url::Url;

const SCENARIO_FILE: &str = "ux_scenario_67_golden_editor_workload.rs";
const MANIFEST: &str = include_str!("../fixtures/golden_editor_workload.json");
const PLAIN_ACTIVE_FILE: &str = "lib/Plain/App.pm";
const PLAIN_SOURCE: &str = r#"package Plain::App;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub run {
    my ($self) = @_;
    my $value = $self->value;
    return $value;
}

sub value {
    return 1;
}

1;
"#;

#[test]
fn golden_manifest_has_exact_after_ready_shape() -> Result<()> {
    let manifest: WorkloadManifest = serde_json::from_str(MANIFEST)?;
    validate_manifest(&manifest)
}

#[test]
fn waiver_expiry_rejects_malformed_and_expired_dates() -> Result<()> {
    ensure!(validate_waiver_expiry("9999-12-31").is_ok());
    ensure!(validate_waiver_expiry("2026-02-30").is_err());
    ensure!(validate_waiver_expiry("2020-01-01").is_err());
    Ok(())
}

#[test]
fn cursor_positions_use_utf16_code_units() -> Result<()> {
    let cursor = position_from_offset("x😀", "x😀".len())?;
    ensure!(cursor.line == 0 && cursor.character == 3, "unexpected UTF-16 cursor: {cursor:?}");
    Ok(())
}

#[test]
fn resource_operations_are_checked_for_external_targets() -> Result<()> {
    let safe = json!({
        "documentChanges": [{ "oldUri": "file:///workspace/lib/App.pm", "newUri": "file:///workspace/lib/App.pm" }]
    });
    let unsafe_edit = json!({
        "documentChanges": [{ "kind": "create", "uri": "file:///workspace/other.pm" }]
    });
    ensure!(!rename_edit_targets_other_file(&safe, "file:///workspace/lib/App.pm"));
    ensure!(rename_edit_targets_other_file(&unsafe_edit, "file:///workspace/lib/App.pm"));
    let suffix_collision = json!({
        "changes": { "file:///workspace/other/lib/App.pm": [] }
    });
    ensure!(rename_edit_targets_other_file(&suffix_collision, "file:///workspace/lib/App.pm"));
    Ok(())
}

#[test]
fn unexplained_empty_results_count_against_zero_budget() -> Result<()> {
    let rollup =
        build_rollup(&[test_workload_row("empty", "empty_result_requires_class_proof")], &[], 0)?;
    ensure!(rollup.unexplained_empty_count == 1);
    Ok(())
}

#[test]
fn protocol_crashes_are_preserved_in_the_rollup() -> Result<()> {
    let rollup = build_rollup(&[], &[], 2)?;
    ensure!(rollup.protocol_crash_count == 2);
    Ok(())
}

#[cfg(test)]
fn test_workload_row(actual_result_class: &str, fallback_or_blocker: &str) -> WorkloadRow {
    WorkloadRow {
        project: "test".to_owned(),
        journey: "test".to_owned(),
        provider: "test".to_owned(),
        source_snapshot: "checked_in_fixture",
        file: "lib/App.pm".to_owned(),
        cursor: CursorReceipt { line: 0, character: 0 },
        request_shape: "test".to_owned(),
        expected_result_class: "test".to_owned(),
        actual_result_class: actual_result_class.to_owned(),
        error_class: "not_applicable".to_owned(),
        actual_locations: Value::Null,
        actual_edits: Value::Null,
        missing: Vec::new(),
        extra: Vec::new(),
        comparison: "not_scored_until_class_promotion",
        answering_tier: "not_observed".to_owned(),
        fact_producer: "not_observed".to_owned(),
        proof_class: "baseline_only".to_owned(),
        confidence: "not_observed".to_owned(),
        freshness: "not_observed".to_owned(),
        fallback_or_blocker: fallback_or_blocker.to_owned(),
        readiness_state: "active_document_ready".to_owned(),
        latency_ms: 1.0,
        unsafe_edit: false,
    }
}

#[derive(Debug, Deserialize)]
struct WorkloadManifest {
    kind: String,
    schema_version: u32,
    manifest_version: String,
    claim_boundary: String,
    projects: Vec<ProjectSpec>,
    journeys: Vec<JourneySpec>,
    zero_budget_metrics: Vec<String>,
    error_waivers: Vec<ErrorWaiver>,
}

#[derive(Debug, Deserialize)]
struct ProjectSpec {
    name: String,
    fixture: String,
    active_file: String,
    completion_needle: String,
    definition_needle: String,
    reference_needle: String,
    safe_rename_needle: String,
}

#[derive(Debug, Deserialize, Clone)]
struct JourneySpec {
    id: String,
    provider: String,
    request_shape: String,
    expected_result_class: String,
}

#[derive(Debug, Deserialize)]
struct ErrorWaiver {
    project: String,
    journey: String,
    expected_error_class: String,
    issue: u64,
    expires_after: String,
}

#[derive(Debug, Serialize)]
struct WorkloadReceipt {
    kind: &'static str,
    schema_version: u32,
    measured_at_unix_ms: u128,
    manifest_version: String,
    claim_boundary: String,
    run_identity: RunIdentity,
    projects: Vec<ProjectReceipt>,
    rows: Vec<WorkloadRow>,
    rollup: Rollup,
}

#[derive(Debug, Serialize)]
struct RunIdentity {
    commit: Option<String>,
    run_id: Option<String>,
    ci: bool,
}

#[derive(Debug, Serialize)]
struct ProjectReceipt {
    project: String,
    fixture: String,
    source_snapshot: &'static str,
    file_count: usize,
    active_file: String,
    active_document_ready: bool,
}

#[derive(Debug, Serialize)]
struct WorkloadRow {
    project: String,
    journey: String,
    provider: String,
    source_snapshot: &'static str,
    file: String,
    cursor: CursorReceipt,
    request_shape: String,
    expected_result_class: String,
    actual_result_class: String,
    error_class: String,
    actual_locations: Value,
    actual_edits: Value,
    missing: Vec<String>,
    extra: Vec<String>,
    comparison: &'static str,
    answering_tier: String,
    fact_producer: String,
    proof_class: String,
    confidence: String,
    freshness: String,
    fallback_or_blocker: String,
    readiness_state: String,
    latency_ms: f64,
    unsafe_edit: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CursorReceipt {
    line: u32,
    character: u32,
}

#[derive(Debug, Default, Serialize)]
struct Rollup {
    exactness_state: &'static str,
    false_exact_count: &'static str,
    stale_exact_count: &'static str,
    unsafe_external_edit_count: usize,
    bounded_fallback_count: usize,
    unexplained_empty_count: usize,
    unwaived_error_count: usize,
    protocol_crash_count: usize,
    measured_debt_count: usize,
    p50_latency_ms: Option<f64>,
    p95_latency_ms: Option<f64>,
}

#[test]
fn scenario_67_golden_editor_workload_receipt() {
    run_ux_scenario(
        "golden_daily_driver_workload",
        SCENARIO_FILE,
        "scenario_67_golden_editor_workload_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Infra),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let manifest: WorkloadManifest =
                serde_json::from_str(MANIFEST).context("golden workload manifest must parse")?;
            validate_manifest(&manifest)?;

            let mut rows = Vec::new();
            let mut project_receipts = Vec::new();
            let mut protocol_crash_count = 0;
            for project in &manifest.projects {
                let files = project_files(project)?;
                let harness = create_harness(project, &files)?;
                let source = if project.fixture == "inline_plain_modern_oo" {
                    PLAIN_SOURCE.to_owned()
                } else {
                    fixture_content(&files, &project.active_file)?.to_owned()
                };
                if files.is_empty() {
                    harness.open_file(&project.active_file, &source)?;
                } else {
                    open_all_fixture_files(&harness, &files)?;
                }
                let active_uri = harness.workspace.uri(&project.active_file);
                let active_document_ready =
                    harness.wait_for_active_document_ready(&active_uri, Duration::from_secs(30));
                ensure!(
                    active_document_ready,
                    "after-ready workload project {} did not reach active-document readiness",
                    project.name
                );
                project_receipts.push(ProjectReceipt {
                    project: project.name.clone(),
                    fixture: project.fixture.clone(),
                    source_snapshot: "checked_in_fixture",
                    file_count: files.len()
                        + usize::from(project.fixture == "inline_plain_modern_oo"),
                    active_file: project.active_file.clone(),
                    active_document_ready,
                });

                run_project_workload(
                    recorder,
                    &manifest.journeys,
                    project,
                    &source,
                    &harness,
                    active_document_ready,
                    &mut rows,
                )?;
                protocol_crash_count += count_protocol_crash_events(&harness);
            }

            let rollup = build_rollup(&rows, &manifest.error_waivers, protocol_crash_count)?;
            let receipt = WorkloadReceipt {
                kind: "golden_editor_workload",
                schema_version: 2,
                measured_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
                manifest_version: manifest.manifest_version,
                claim_boundary: manifest.claim_boundary,
                run_identity: RunIdentity {
                    commit: std::env::var("GITHUB_SHA").ok(),
                    run_id: std::env::var("GITHUB_RUN_ID").ok(),
                    ci: std::env::var("CI").is_ok(),
                },
                projects: project_receipts,
                rows,
                rollup,
            };
            write_workload_receipt(&receipt)?;

            recorder.check("golden workload produced rows", !receipt.rows.is_empty())?;
            recorder.check(
                "exactness remains explicitly unscored",
                receipt.rollup.exactness_state == "not_scored"
                    && receipt.rollup.false_exact_count == "not_measured"
                    && receipt.rollup.stale_exact_count == "not_measured",
            )?;
            recorder.check(
                "unsafe external edit count remains zero",
                receipt.rollup.unsafe_external_edit_count == 0,
            )?;
            recorder.check(
                "unwaived request errors remain zero",
                receipt.rollup.unwaived_error_count == 0,
            )?;
            recorder.check(
                "unexplained empty count remains zero",
                receipt.rollup.unexplained_empty_count == 0,
            )?;
            recorder.check(
                "protocol crash count remains zero",
                receipt.rollup.protocol_crash_count == 0,
            )?;
            Ok(())
        },
    );
}

fn validate_manifest(manifest: &WorkloadManifest) -> Result<()> {
    ensure!(manifest.kind == "golden_editor_workload", "unexpected manifest kind");
    ensure!(manifest.schema_version == 2, "unsupported manifest schema");
    let expected_projects =
        BTreeSet::from(["catalyst", "dancer2", "mojolicious", "plain_modern_oo"]);
    let actual_projects =
        manifest.projects.iter().map(|project| project.name.as_str()).collect::<BTreeSet<_>>();
    ensure!(
        actual_projects == expected_projects,
        "workload must cover the exact four projects once"
    );
    ensure!(
        manifest.projects.len() == expected_projects.len(),
        "workload contains duplicate projects"
    );
    let expected_journeys = BTreeSet::from([
        "close_reopen_pending",
        "completion_after_ready",
        "definition_local_or_imported",
        "diagnostics_present_import",
        "edit_burst_completion",
        "edit_burst_hover",
        "hover_after_ready",
        "references_lexical",
        "rename_safe_lexical",
        "workspace_symbols_after_ready",
    ]);
    let actual_journeys =
        manifest.journeys.iter().map(|journey| journey.id.as_str()).collect::<BTreeSet<_>>();
    ensure!(
        actual_journeys == expected_journeys,
        "workload must cover the exact ten journeys once"
    );
    ensure!(
        manifest.journeys.len() == expected_journeys.len(),
        "workload contains duplicate journeys"
    );
    for metric in [
        "exactness_state",
        "unwaived_error_count",
        "unsafe_external_edit_count",
        "protocol_crash_count",
        "unexplained_empty_count",
    ] {
        ensure!(
            manifest.zero_budget_metrics.iter().any(|candidate| candidate == metric),
            "manifest is missing zero-budget metric {metric}"
        );
    }
    for waiver in &manifest.error_waivers {
        ensure!(waiver.issue > 0, "error waiver must name a tracking issue");
        validate_waiver_expiry(&waiver.expires_after)?;
    }
    Ok(())
}

fn validate_waiver_expiry(expires_after: &str) -> Result<()> {
    let mut parts = expires_after.split('-');
    let year = parts.next().context("waiver expiry is missing a year")?.parse::<i64>()?;
    let month = parts.next().context("waiver expiry is missing a month")?.parse::<u32>()?;
    let day = parts.next().context("waiver expiry is missing a day")?.parse::<u32>()?;
    ensure!(parts.next().is_none(), "waiver expiry must use YYYY-MM-DD: {expires_after}");
    ensure!(expires_after.len() == 10, "waiver expiry must use YYYY-MM-DD: {expires_after}");
    ensure!((1..=12).contains(&month), "waiver expiry month is invalid: {expires_after}");
    let days_in_month = match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    ensure!((1..=days_in_month).contains(&day), "waiver expiry day is invalid: {expires_after}");

    let expiry_days = days_from_civil(year, month, day);
    let today_days =
        i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() / 86_400)?;
    ensure!(
        expiry_days >= today_days,
        "error waiver expired on {expires_after}; refresh or remove the waiver"
    );
    Ok(())
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Return days since 1970-01-01 for a proleptic Gregorian calendar date.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn project_files(project: &ProjectSpec) -> Result<Vec<ProjectFixtureFile>> {
    match project.fixture.as_str() {
        "mojolicious" => load_mojolicious_fixture_files(),
        "dancer2" => load_dancer2_fixture_files(),
        "catalyst" => load_catalyst_fixture_files(),
        "inline_plain_modern_oo" => Ok(Vec::new()),
        other => Err(anyhow!("unknown workload fixture {other}")),
    }
}

fn create_harness(project: &ProjectSpec, files: &[ProjectFixtureFile]) -> Result<UxHarness> {
    if project.fixture == "inline_plain_modern_oo" {
        return UxHarness::new(
            ScenarioConfig { timeout: Duration::from_secs(30), ..Default::default() }
                .env("PERL_LSP_WORKSPACE", "1")
                .env("PERL_LSP_E2E", "1")
                .with_file(PLAIN_ACTIVE_FILE, PLAIN_SOURCE),
        );
    }
    UxHarness::new(fixture_scenario_config(files).env("PERL_LSP_E2E", "1"))
}

fn run_project_workload(
    recorder: &mut perl_lsp_ux_tests::UxRunRecorder,
    journeys: &[JourneySpec],
    project: &ProjectSpec,
    source: &str,
    harness: &UxHarness,
    active_document_ready: bool,
    rows: &mut Vec<WorkloadRow>,
) -> Result<()> {
    let completion_cursor = position_after(source, &project.completion_needle)?;
    let definition_cursor = position_at(source, &project.definition_needle)?;
    let reference_cursor = position_at(source, &project.reference_needle)?;
    let rename_cursor = position_at(source, &project.safe_rename_needle)?;

    for journey in journeys {
        let operation = operation_name(project, journey);
        recorder.mark_request_start(&operation);
        let request_started = Instant::now();
        let (cursor, response, actual_edits) = match journey.id.as_str() {
            "completion_after_ready" => (
                completion_cursor,
                capture(harness.completion(
                    &project.active_file,
                    completion_cursor.line,
                    completion_cursor.character,
                )),
                Value::Null,
            ),
            "hover_after_ready" => (
                reference_cursor,
                capture(
                    harness
                        .hover(
                            &project.active_file,
                            reference_cursor.line,
                            reference_cursor.character,
                        )
                        .map(|value| value.unwrap_or(Value::Null)),
                ),
                Value::Null,
            ),
            "definition_local_or_imported" => (
                definition_cursor,
                capture(harness.definition(
                    &project.active_file,
                    definition_cursor.line,
                    definition_cursor.character,
                )),
                Value::Null,
            ),
            "references_lexical" => (
                reference_cursor,
                capture(harness.references(
                    &project.active_file,
                    reference_cursor.line,
                    reference_cursor.character,
                    true,
                )),
                Value::Null,
            ),
            "rename_safe_lexical" => {
                let result = capture(request_rename(harness, &project.active_file, rename_cursor));
                let edits = result.clone();
                (rename_cursor, result, edits)
            }
            "diagnostics_present_import" => {
                let diagnostics =
                    harness.wait_for_diagnostics(&project.active_file, Duration::from_secs(5));
                (CursorReceipt { line: 0, character: 0 }, json!(diagnostics), Value::Null)
            }
            "workspace_symbols_after_ready" => (
                CursorReceipt { line: 0, character: 0 },
                capture(harness.workspace_symbols("new")),
                Value::Null,
            ),
            "edit_burst_completion" => (
                completion_cursor,
                run_edit_burst_completion(
                    harness,
                    &project.active_file,
                    source,
                    completion_cursor,
                )?,
                Value::Null,
            ),
            "edit_burst_hover" => (
                reference_cursor,
                run_edit_burst_hover(harness, &project.active_file, source, reference_cursor)?,
                Value::Null,
            ),
            "close_reopen_pending" => {
                let uri = harness.workspace.uri(&project.active_file);
                harness
                    .client
                    .notify("textDocument/didClose", json!({"textDocument": {"uri": uri}}))?;
                harness.open_file(&project.active_file, source)?;
                (CursorReceipt { line: 0, character: 0 }, json!({"reopened": true}), Value::Null)
            }
            other => return Err(anyhow!("unhandled golden workload journey {other}")),
        };
        let request_latency_ms = request_started.elapsed().as_secs_f64() * 1000.0;

        let row = response_row(
            project,
            journey,
            cursor,
            response,
            actual_edits,
            if active_document_ready { "active_document_ready" } else { "active_document_pending" },
            request_latency_ms,
            harness,
        )?;
        let row = if journey.provider == "lifecycle" {
            apply_lifecycle_receipt(row)
        } else {
            let receipt_id = format!(
                "golden_{project_name}_{journey_id}",
                project_name = project.name,
                journey_id = journey.id,
            );
            let receipt_provider = receipt_provider_name(&journey.provider);
            let provider_receipt = explain_provider(
                harness,
                receipt_provider,
                &receipt_id,
                journey.id.as_str(),
                cursor,
            )?;
            apply_provider_receipt(row, Ok(provider_receipt))
        };
        if row.actual_result_class != "empty" && row.actual_result_class != "error" {
            recorder.mark_first_useful_result(&operation);
        }
        rows.push(row);
    }
    Ok(())
}

fn receipt_provider_name(provider: &str) -> &str {
    match provider {
        "definition" => "goto_definition",
        other => other,
    }
}

fn run_edit_burst_completion(
    harness: &UxHarness,
    file: &str,
    source: &str,
    cursor: CursorReceipt,
) -> Result<Value> {
    for edit in 0..20 {
        harness.change_file_full(file, &format!("{source}\n# golden editor burst {edit}"))?;
    }
    // The initial active-document-ready gate is already established. The
    // burst intentionally measures provider behavior while edits are pending;
    // it does not claim post-edit workspace readiness.
    let response = capture(harness.completion(file, cursor.line, cursor.character));
    harness.change_file_full(file, source)?;
    Ok(response)
}

fn run_edit_burst_hover(
    harness: &UxHarness,
    file: &str,
    source: &str,
    cursor: CursorReceipt,
) -> Result<Value> {
    for edit in 0..20 {
        harness.change_file_full(file, &format!("{source}\n# golden editor burst {edit}"))?;
    }
    let response = capture(
        harness
            .hover(file, cursor.line, cursor.character)
            .map(|value| value.unwrap_or(Value::Null)),
    );
    harness.change_file_full(file, source)?;
    Ok(response)
}

fn request_rename(harness: &UxHarness, file: &str, cursor: CursorReceipt) -> Result<Value> {
    let response = harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": harness.workspace.uri(file) },
            "position": { "line": cursor.line, "character": cursor.character },
            "newName": "golden_renamed_value"
        }),
        Duration::from_secs(5),
    )?;
    ensure!(response.get("error").is_none(), "rename returned a protocol error: {response}");
    Ok(response.get("result").cloned().unwrap_or(Value::Null))
}

fn response_row(
    project: &ProjectSpec,
    journey: &JourneySpec,
    cursor: CursorReceipt,
    response: Value,
    actual_edits: Value,
    readiness_state: &str,
    latency_ms: f64,
    harness: &UxHarness,
) -> Result<WorkloadRow> {
    let actual_result_class = classify_result(&response, journey.provider.as_str());
    let error_class = if actual_result_class == "error" {
        classify_error(&response)
    } else {
        "not_applicable".to_owned()
    };
    let is_empty = actual_result_class == "empty";
    let fallback_or_blocker =
        response.get("_golden_error").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(
            || {
                if is_empty {
                    "empty_result_requires_class_proof".to_owned()
                } else {
                    "not_observed".to_owned()
                }
            },
        );
    let actual_locations = if matches!(journey.provider.as_str(), "definition" | "references") {
        harness.normalize_response(&response)
    } else {
        Value::Array(Vec::new())
    };
    let normalized_edits = harness.normalize_response(&actual_edits);
    let active_uri = harness.workspace.uri(&project.active_file);
    let unsafe_edit = journey.provider == "rename"
        && rename_edit_targets_other_file(&normalized_edits, &active_uri);
    Ok(WorkloadRow {
        project: project.name.clone(),
        journey: journey.id.clone(),
        provider: journey.provider.clone(),
        source_snapshot: "checked_in_fixture",
        file: project.active_file.clone(),
        cursor,
        request_shape: journey.request_shape.clone(),
        expected_result_class: journey.expected_result_class.clone(),
        actual_result_class,
        error_class,
        actual_locations,
        actual_edits: normalized_edits,
        missing: Vec::new(),
        extra: Vec::new(),
        comparison: "not_scored_until_class_promotion",
        answering_tier: "not_observed".to_owned(),
        fact_producer: "not_observed".to_owned(),
        proof_class: "baseline_only".to_owned(),
        confidence: "not_observed".to_owned(),
        freshness: "not_observed".to_owned(),
        fallback_or_blocker,
        readiness_state: readiness_state.to_owned(),
        latency_ms,
        unsafe_edit,
    })
}

fn classify_error(response: &Value) -> String {
    let message = response.get("_golden_error").and_then(Value::as_str).unwrap_or_default();
    let message_lower = message.to_ascii_lowercase();
    if message_lower.contains("request superseded") {
        "request_superseded".to_owned()
    } else if message_lower.contains("timeout") || message_lower.contains("timed out") {
        "timeout".to_owned()
    } else {
        "request_error".to_owned()
    }
}

fn apply_lifecycle_receipt(mut row: WorkloadRow) -> WorkloadRow {
    row.answering_tier = "lifecycle".to_owned();
    row.fact_producer = "transport".to_owned();
    row.proof_class = "lifecycle".to_owned();
    row.confidence = "not_applicable".to_owned();
    row.freshness = "current".to_owned();
    row.fallback_or_blocker = "lifecycle_evidence".to_owned();
    row
}

fn apply_provider_receipt(mut row: WorkloadRow, receipt: Result<Value>) -> WorkloadRow {
    let Ok(receipt) = receipt else {
        return row;
    };
    row.answering_tier = string_field(&receipt, "answering_tier");
    row.fact_producer = string_field(&receipt, "fact_source");
    row.confidence = string_field(&receipt, "confidence");
    row.freshness = string_field(&receipt, "freshness");
    row.proof_class = string_field(&receipt, "proof_class");
    if row.actual_result_class != "error" {
        row.fallback_or_blocker = receipt
            .get("fallback")
            .or_else(|| receipt.get("fallback_state"))
            .and_then(Value::as_str)
            .unwrap_or(&row.fallback_or_blocker)
            .to_owned();
    }
    row
}

fn explain_provider(
    harness: &UxHarness,
    provider: &str,
    receipt_id: &str,
    journey: &str,
    cursor: CursorReceipt,
) -> Result<Value> {
    let response = harness.client.request(
        "workspace/executeCommand",
        json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": provider,
                "receipt_id": receipt_id,
                "scenario": "golden_daily_driver_workload",
                "request_position": {
                    "uri_scheme": "file",
                    "line": cursor.line,
                    "character": cursor.character
                }
            }]
        }),
        Duration::from_secs(20),
    )?;
    ensure!(response.get("error").is_none(), "provider explanation returned an error: {response}");
    let result = response.get("result").cloned().unwrap_or(Value::Null);
    ensure!(
        result.get("provider").and_then(Value::as_str) == Some(provider),
        "provider receipt for {journey} returned the wrong provider"
    );
    ensure!(
        result.get("receipt_id").and_then(Value::as_str) == Some(receipt_id),
        "provider receipt for {journey} was not bound to request {receipt_id}"
    );
    ensure!(
        result.get("scenario").and_then(Value::as_str) == Some("golden_daily_driver_workload"),
        "provider receipt for {journey} was not bound to the workload scenario"
    );
    Ok(result)
}

fn classify_result(response: &Value, provider: &str) -> String {
    if response.is_null() {
        return if provider == "rename" { "refused" } else { "empty" }.to_owned();
    }
    if response.get("error").is_some() || response.get("_golden_error").is_some() {
        return "error".to_owned();
    }
    if response.as_array().is_some_and(Vec::is_empty) {
        return "empty".to_owned();
    }
    if provider == "rename" && response.as_object().is_some_and(|object| object.is_empty()) {
        return "refused".to_owned();
    }
    "partial".to_owned()
}

fn capture<T: serde::Serialize>(result: Result<T>) -> Value {
    match result {
        Ok(value) => json!(value),
        Err(error) => json!({ "_golden_error": error.to_string() }),
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value.get(field).and_then(Value::as_str).unwrap_or("not_observed").to_owned()
}

fn operation_name(project: &ProjectSpec, journey: &JourneySpec) -> String {
    format!("golden_{}_{}", project.name, journey.id)
}

fn build_rollup(
    rows: &[WorkloadRow],
    error_waivers: &[ErrorWaiver],
    protocol_crash_count: usize,
) -> Result<Rollup> {
    let mut rollup = Rollup {
        exactness_state: "not_scored",
        false_exact_count: "not_measured",
        stale_exact_count: "not_measured",
        protocol_crash_count,
        ..Rollup::default()
    };
    let mut latencies = Vec::with_capacity(rows.len());
    for row in rows {
        latencies.push(row.latency_ms);
        match row.actual_result_class.as_str() {
            "error" => {
                if error_waivers.iter().any(|waiver| {
                    waiver.project == row.project
                        && waiver.journey == row.journey
                        && waiver.expected_error_class == row.error_class
                }) {
                    rollup.measured_debt_count += 1;
                } else {
                    rollup.unwaived_error_count += 1;
                }
            }
            "empty" => {
                rollup.unexplained_empty_count += usize::from(matches!(
                    row.fallback_or_blocker.as_str(),
                    "not_observed" | "empty_result_requires_class_proof"
                ));
                rollup.measured_debt_count += 1;
            }
            "partial" => rollup.bounded_fallback_count += 1,
            "refused" => rollup.measured_debt_count += 1,
            _ => {}
        }
        rollup.unsafe_external_edit_count += usize::from(row.unsafe_edit);
    }
    latencies.sort_by(f64::total_cmp);
    rollup.p50_latency_ms = percentile(&latencies, 0.50);
    rollup.p95_latency_ms = percentile(&latencies, 0.95);
    Ok(rollup)
}

fn rename_edit_targets_other_file(edit: &Value, active_uri: &str) -> bool {
    let mut unsafe_edit = false;
    if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
        unsafe_edit |= changes.keys().any(|uri| !same_document_uri(uri, active_uri));
    }
    if let Some(changes) = edit.get("documentChanges").and_then(Value::as_array) {
        unsafe_edit |= changes.iter().any(|change| {
            [
                change.pointer("/textDocument/uri"),
                change.get("uri"),
                change.get("oldUri"),
                change.get("newUri"),
            ]
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|uri| !same_document_uri(uri, active_uri))
        });
    }
    unsafe_edit
}

fn same_document_uri(left: &str, right: &str) -> bool {
    match (Url::parse(left), Url::parse(right)) {
        (Ok(left), Ok(right)) if left.scheme() == "file" && right.scheme() == "file" => {
            left.host_str() == right.host_str() && left.path() == right.path()
        }
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn count_protocol_crash_events(harness: &UxHarness) -> usize {
    harness
        .peek_notifications()
        .iter()
        .filter(|event| {
            let message = format!("{event:?}");
            message.contains("panicked")
                || message.contains("SIGABRT")
                || message.contains("stack overflow")
        })
        .count()
}

fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let index = ((values.len() - 1) as f64 * fraction).round() as usize;
    values.get(index).copied()
}

fn write_workload_receipt(receipt: &WorkloadReceipt) -> Result<PathBuf> {
    let directory = std::env::var_os("PERL_LSP_UX_RECEIPT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/receipts/editor-ux"));
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating receipt directory {}", directory.display()))?;
    let path = directory.join("golden-editor-workload-v1.json");
    let content = serde_json::to_string_pretty(receipt).context("serializing workload receipt")?;
    fs::write(&path, format!("{content}\n"))
        .with_context(|| format!("writing workload receipt {}", path.display()))?;
    Ok(path)
}

fn position_at(source: &str, needle: &str) -> Result<CursorReceipt> {
    let offset = source.find(needle).with_context(|| format!("missing `{needle}`"))?;
    position_from_offset(source, offset)
}

fn position_after(source: &str, needle: &str) -> Result<CursorReceipt> {
    let offset = source
        .find(needle)
        .with_context(|| format!("missing `{needle}`"))?
        .checked_add(needle.len())
        .context("cursor offset overflow")?;
    position_from_offset(source, offset)
}

fn position_from_offset(source: &str, offset: usize) -> Result<CursorReceipt> {
    let prefix = source.get(..offset).context("cursor is not a UTF-8 boundary")?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character =
        prefix.rsplit('\n').next().map(|line| line.chars().map(char::len_utf16).sum()).unwrap_or(0);
    Ok(CursorReceipt { line: u32::try_from(line)?, character: u32::try_from(character)? })
}
