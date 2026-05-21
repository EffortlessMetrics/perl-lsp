//! Scenario 44 - Real Perl Editor Trust smoke receipt.
//!
//! This receipt exercises a small CPAN-style workspace across query, setup, and
//! preview-first edit surfaces. It records whether providers acted, fell back,
//! or refused unsafe edits without changing provider behavior.

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_44_real_editor_trust_smoke_receipt.rs";
const APP_PATH: &str = "lib/RealBaseline/App.pm";
const BASE_PATH: &str = "lib/RealBaseline/Base.pm";
const UTIL_PATH: &str = "lib/RealBaseline/Util.pm";
const SCRIPT_PATH: &str = "script/real-baseline.pl";

const APP_PM: &str = r#"package RealBaseline::App;
use strict;
use warnings;
use parent 'RealBaseline::Base';
use RealBaseline::Util qw(helper alias);

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub run {
    my ($self) = @_;
    helper($self->name);
    alias($self->shared);
    return $self->shared;
}

sub name {
    return $_[0]->{name};
}

1;
"#;

const BASE_PM: &str = r#"package RealBaseline::Base;
use strict;
use warnings;

sub shared {
    return 'shared';
}

sub reset {
    return 1;
}

1;
"#;

const UTIL_PM: &str = r#"package RealBaseline::Util;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(helper alias);

sub helper {
    return shift;
}

*alias = \&helper;

sub bounce {
    goto &helper;
}

1;
"#;

const SCRIPT_PL: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealBaseline::App;

my $app = RealBaseline::App->new(name => 'demo');
$app->run;
"#;

#[derive(Debug, Serialize)]
struct TrustSurfaceReport {
    surface: &'static str,
    decision: String,
    reason: String,
    fact_source: String,
    confidence: String,
    freshness: String,
    fallback: String,
    dynamic_boundary: String,
    source_backed: bool,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(APP_PATH, APP_PM)
            .with_file(BASE_PATH, BASE_PM)
            .with_file(UTIL_PATH, UTIL_PM)
            .with_file(SCRIPT_PATH, SCRIPT_PL),
    )
}

fn position_after(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset =
        source.find(needle).with_context(|| format!("missing `{needle}`"))? + needle.len();
    position_from_byte_offset(source, byte_offset)
}

fn position_at(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset = source.find(needle).with_context(|| format!("missing `{needle}`"))?;
    position_from_byte_offset(source, byte_offset)
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn has_pl701(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic.get("code").and_then(Value::as_str).is_some_and(|code| code == "PL701")
            || diagnostic.get("code").and_then(Value::as_u64) == Some(701)
    })
}

fn completion_item_has_shape(item: &Value) -> bool {
    item.get("label").and_then(Value::as_str).is_some()
        || item.get("insertText").and_then(Value::as_str).is_some()
        || item.get("filterText").and_then(Value::as_str).is_some()
}

fn lsp_location_has_shape(entry: &Value) -> bool {
    let location = entry.get("uri").is_some() && entry.get("range").is_some();
    let location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    location || location_link
}

fn location_points_to(entry: &Value, suffix: &str) -> bool {
    entry
        .get("uri")
        .or_else(|| entry.get("targetUri"))
        .and_then(Value::as_str)
        .is_some_and(|uri| uri.ends_with(suffix))
}

fn text(value: &str) -> String {
    value.to_string()
}

fn receipt_text(receipt: &Value, key: &str, fallback: &str) -> String {
    receipt.get(key).and_then(Value::as_str).unwrap_or(fallback).to_string()
}

fn execute_command(harness: &UxHarness, command: &str, arguments: Value) -> Result<Value> {
    let response = harness.client.request(
        "workspace/executeCommand",
        json!({
            "command": command,
            "arguments": arguments,
        }),
        Duration::from_secs(20),
    )?;
    if response.get("error").is_some() {
        return Err(anyhow!("{command} returned error: {}", response["error"]));
    }
    Ok(response["result"].clone())
}

fn workspace_trust_report(harness: &UxHarness) -> Result<Value> {
    execute_command(
        harness,
        "perl.workspaceTrustReport",
        json!([{
            "client_runtime_state": {
                "source": "ux-smoke",
                "perldoc": {
                    "status": "client_surface_not_reported"
                },
                "dap": {
                    "status": "client_state_not_reported",
                    "adapter_registered": false,
                    "active_perl_debug_session": false,
                    "managed_adapter_exists": false,
                    "launch_json_workspace_count": 0,
                    "workspace_folder_count": 1
                }
            }
        }]),
    )
}

fn safe_delete_preview_for_helper(harness: &UxHarness) -> Result<Value> {
    let (line, character) = position_at(UTIL_PM, "helper {")?;
    execute_command(
        harness,
        "perl.previewSafeDelete",
        json!([{
            "textDocument": {"uri": harness.workspace.uri(UTIL_PATH)},
            "position": {"line": line, "character": character}
        }]),
    )
}

fn explain_safe_delete_decision(harness: &UxHarness) -> Result<Value> {
    execute_command(
        harness,
        "perl.explainProviderDecision",
        json!([{
            "provider": "safe_delete"
        }]),
    )
}

#[test]
fn scenario_44_real_editor_trust_smoke_receipt() {
    run_ux_scenario(
        "real_editor_trust_smoke",
        SCENARIO_FILE,
        "scenario_44_real_editor_trust_smoke_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Infra),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(APP_PATH, APP_PM)?;
            harness.open_file(BASE_PATH, BASE_PM)?;
            harness.open_file(UTIL_PATH, UTIL_PM)?;
            harness.open_file(SCRIPT_PATH, SCRIPT_PL)?;
            std::thread::sleep(Duration::from_millis(500));

            let mut reports = Vec::new();

            recorder.mark_request_start("completion_receiver");
            let (completion_line, completion_character) = position_after(APP_PM, "$self->")?;
            let completion_items =
                harness.completion(APP_PATH, completion_line, completion_character)?;
            for item in &completion_items {
                anyhow::ensure!(
                    completion_item_has_shape(item),
                    "completion item must include label, insertText, or filterText: {item:?}",
                );
            }
            if !completion_items.is_empty() {
                recorder.mark_first_useful_result("completion_receiver");
            }
            reports.push(TrustSurfaceReport {
                surface: "completion",
                decision: if completion_items.is_empty() { "fallback" } else { "acted" }
                    .to_string(),
                reason: format!("candidate_count={}", completion_items.len()),
                fact_source: text("completion_provider"),
                confidence: text("provider_local"),
                freshness: text("fresh_open_document"),
                fallback: text("legacy_completion_preserved"),
                dynamic_boundary: text("not_applicable"),
                source_backed: true,
            });

            recorder.mark_request_start("definition_imported_helper");
            let (helper_line, helper_character) = position_at(APP_PM, "helper($self")?;
            let definitions = harness.definition_with_retry(
                APP_PATH,
                helper_line,
                helper_character,
                5,
                Duration::from_millis(200),
            )?;
            for entry in &definitions {
                anyhow::ensure!(
                    lsp_location_has_shape(entry),
                    "definition entry must be a Location or LocationLink: {entry:?}",
                );
            }
            let helper_source_backed =
                definitions.iter().any(|entry| location_points_to(entry, "Util.pm"));
            if helper_source_backed {
                recorder.mark_first_useful_result("definition_imported_helper");
            }
            reports.push(TrustSurfaceReport {
                surface: "definition",
                decision: if helper_source_backed { "acted" } else { "fallback" }.to_string(),
                reason: format!("location_count={}", definitions.len()),
                fact_source: text("definition_provider"),
                confidence: text(if helper_source_backed { "high" } else { "low" }),
                freshness: text("fresh_open_document"),
                fallback: text("empty_or_legacy_definition_allowed"),
                dynamic_boundary: text("not_applicable"),
                source_backed: helper_source_backed,
            });

            recorder.mark_request_start("diagnostics_workspace_present_imports");
            let diagnostics = harness.wait_for_diagnostics(APP_PATH, Duration::from_secs(5));
            let pl701_present = has_pl701(&diagnostics);
            reports.push(TrustSurfaceReport {
                surface: "diagnostics",
                decision: if pl701_present { "fallback" } else { "acted" }.to_string(),
                reason: format!("workspace_present_import_pl701={pl701_present}"),
                fact_source: text("diagnostics_provider"),
                confidence: text(if pl701_present { "low" } else { "high" }),
                freshness: text("fresh_open_document"),
                fallback: text("conservative_diagnostics_preserved"),
                dynamic_boundary: text("not_applicable"),
                source_backed: !pl701_present,
            });

            recorder.mark_request_start("workspace_trust_report");
            let trust_report = workspace_trust_report(&harness)?;
            recorder.mark_first_useful_result("workspace_trust_report");
            let trust_schema = trust_report.get("schema_version").and_then(Value::as_str);
            let trust_boundary_mentions_no_scan = trust_report
                .get("claim_boundary")
                .and_then(Value::as_str)
                .is_some_and(|claim| claim.contains("does not scan files"));
            reports.push(TrustSurfaceReport {
                surface: "workspace_trust_report",
                decision: "acted".to_string(),
                reason: format!("schema_version={}", trust_schema.unwrap_or("missing")),
                fact_source: text("existing_server_state"),
                confidence: text("bounded_report"),
                freshness: text("current_runtime_state"),
                fallback: text("report_only_no_probe"),
                dynamic_boundary: text("not_applicable"),
                source_backed: false,
            });

            recorder.mark_request_start("safe_delete_preview_helper");
            let safe_delete = safe_delete_preview_for_helper(&harness)?;
            recorder.mark_first_useful_result("safe_delete_preview_helper");
            let safe_delete_decision =
                safe_delete.get("decision").and_then(Value::as_str).unwrap_or("unknown");
            let safe_delete_edits_applied =
                safe_delete.get("edits_applied").and_then(Value::as_bool).unwrap_or(false);
            reports.push(TrustSurfaceReport {
                surface: "safe_delete_preview",
                decision: safe_delete_decision.to_string(),
                reason: safe_delete
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("reason_not_reported")
                    .to_string(),
                fact_source: text("safe_delete_preview_receipt"),
                confidence: receipt_text(&safe_delete, "confidence", "bounded_preview"),
                freshness: receipt_text(&safe_delete, "freshness", "fresh_open_document"),
                fallback: receipt_text(&safe_delete, "fallback_state", "no_edit"),
                dynamic_boundary: receipt_text(&safe_delete, "dynamic_boundary", "not_applicable"),
                source_backed: safe_delete
                    .get("source_backed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });

            recorder.mark_request_start("explain_safe_delete_decision");
            let safe_delete_explanation = explain_safe_delete_decision(&harness)?;
            recorder.mark_first_useful_result("explain_safe_delete_decision");
            let explanation_schema =
                safe_delete_explanation.get("schema_version").and_then(Value::as_str);
            let copyable_payload_present =
                safe_delete_explanation.get("copyable_payload").is_some();
            reports.push(TrustSurfaceReport {
                surface: "explain_provider_decision",
                decision: safe_delete_explanation
                    .get("decision")
                    .and_then(Value::as_str)
                    .unwrap_or("fallback")
                    .to_string(),
                reason: format!(
                    "schema_version={}; copyable_payload={copyable_payload_present}",
                    explanation_schema.unwrap_or("missing")
                ),
                fact_source: text("provider_decision_receipt"),
                confidence: receipt_text(
                    &safe_delete_explanation,
                    "confidence",
                    "bounded_explanation",
                ),
                freshness: receipt_text(&safe_delete_explanation, "freshness", "current_request"),
                fallback: receipt_text(&safe_delete_explanation, "fallback", "explanation_only"),
                dynamic_boundary: receipt_text(
                    &safe_delete_explanation,
                    "dynamic_boundary",
                    "not_applicable",
                ),
                source_backed: safe_delete_explanation
                    .get("source_backed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });

            let acted_count = reports.iter().filter(|report| report.decision == "acted").count();
            let fallback_count = reports
                .iter()
                .filter(|report| report.decision == "fallback" || report.fallback != "none")
                .count();
            let refused_no_edit_count = reports
                .iter()
                .filter(|report| {
                    report.surface == "safe_delete_preview"
                        && report.decision == "blocked"
                        && !safe_delete_edits_applied
                })
                .count();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "real_perl_editor_trust_smoke",
                "workspace_fixture": "RealBaseline four-file CPAN-style workspace",
                "claim_boundary": "product-level provider smoke receipt only; no provider behavior broadening, support-tier promotion, or release claim",
                "surface_count": reports.len(),
                "acted_count": acted_count,
                "fallback_or_bounded_count": fallback_count,
                "refused_no_edit_count": refused_no_edit_count,
                "workspace_trust_report_schema": trust_schema,
                "workspace_trust_report_no_scan_boundary": trust_boundary_mentions_no_scan,
                "safe_delete_preview_edits_applied": safe_delete_edits_applied,
                "explain_provider_decision_copyable_payload": copyable_payload_present,
                "surfaces": reports,
            });
            eprintln!(
                "real_perl_editor_trust_smoke_receipt={}",
                serde_json::to_string_pretty(&receipt)?,
            );

            recorder.check("smoke receipt covered six trust surfaces", reports.len() == 6)?;
            recorder.check("at least one provider acted", acted_count > 0)?;
            recorder.check(
                "workspace trust report kept no-scan boundary visible",
                trust_boundary_mentions_no_scan,
            )?;
            recorder.check(
                "safe-delete preview refused helper without applying edits",
                refused_no_edit_count == 1,
            )?;
            recorder.check(
                "explain-provider-decision returned a copyable payload",
                copyable_payload_present,
            )?;
            recorder.check("workspace-present imports did not emit PL701", !pl701_present)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
