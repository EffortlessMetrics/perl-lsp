//! Scenario 31 - Mojolicious diagnostics quality receipt.
//!
//! This receipt exercises diagnostics over the committed Mojolicious skeleton
//! workspace and records current correctness boundaries without changing
//! provider behavior.
//!
//! Receipt signals:
//! - diagnostic notification and payload shape for selected real-workspace files
//! - no false PL701 for Mojolicious modules present in the fixture workspace
//! - conservative handling for a dynamic typeglob route-method registration
//! - true-missing-module PL701 behavior from an injected probe file
//! - mixed project-shaped files keep present modules clean while reporting a
//!   genuinely missing module

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    LspEvent, ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness,
    fixture_scenario_config, load_mojolicious_fixture_files, open_all_fixture_files,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_31_mojolicious_diagnostics_quality.rs";
const MISSING_MODULE_PROBE_PATH: &str = "lib/Mojolicious/DiagnosticsProbe.pm";
const MIXED_MODULE_PROBE_PATH: &str = "lib/Mojolicious/DiagnosticsMixedProbe.pm";
const MISSING_MODULE_PROBE_SOURCE: &str = r#"package Mojolicious::DiagnosticsProbe;
use Mojo::Base -base, -signatures;
use Definitely::Missing::ForMojoliciousReceipt;

sub diagnostic_probe { return 1 }

1;
"#;
const MIXED_MODULE_PROBE_SOURCE: &str = r#"package Mojolicious::DiagnosticsMixedProbe;
use Mojo::Base -base, -signatures;
use Mojolicious::Routes;
use Mojolicious::Types;
use Definitely::Missing::MixedMojoliciousReceipt;

sub mixed_diagnostic_probe {
  return (Mojolicious::Routes->new, Mojolicious::Types->new);
}

1;
"#;

#[derive(Debug)]
struct DiagnosticProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    absent_pl701_modules: &'static [&'static str],
    required_codes: &'static [&'static str],
    required_message_substrings: &'static [&'static str],
    forbidden_message_substrings: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct DiagnosticProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    diagnostic_count: usize,
    notification_count: usize,
    invalid_shape_count: usize,
    pl701_count: usize,
    false_positive_pl701_modules: Vec<String>,
    required_code_hits: Vec<String>,
    missing_required_codes: Vec<String>,
    required_message_hits: Vec<String>,
    missing_required_message_substrings: Vec<String>,
    forbidden_message_hits: Vec<String>,
    dynamic_boundary_label_hits: Vec<String>,
    message_excerpts: Vec<String>,
    fallback_or_empty: bool,
}

fn create_mojolicious_harness(files: &[FixtureFile]) -> Result<UxHarness> {
    let config = fixture_scenario_config(files)
        .with_file(MISSING_MODULE_PROBE_PATH, MISSING_MODULE_PROBE_SOURCE)
        .with_file(MIXED_MODULE_PROBE_PATH, MIXED_MODULE_PROBE_SOURCE);

    UxHarness::new(config)
}

fn diagnostic_probes() -> Vec<DiagnosticProbe> {
    vec![
        DiagnosticProbe {
            name: "workspace_present_mojolicious_imports",
            category: "workspace_module_resolution_no_false_pl701",
            file: "lib/Mojolicious.pm",
            absent_pl701_modules: &[
                "Mojo::Base",
                "Mojolicious::Commands",
                "Mojolicious::Controller",
                "Mojolicious::Log",
                "Mojolicious::Plugins",
                "Mojolicious::Renderer",
                "Mojolicious::Routes",
                "Mojolicious::Sessions",
                "Mojolicious::Static",
                "Mojolicious::Types",
            ],
            required_codes: &[],
            required_message_substrings: &[],
            forbidden_message_substrings: &[],
        },
        DiagnosticProbe {
            name: "dynamic_route_method_registration",
            category: "dynamic_boundary_conservative",
            file: "lib/Mojolicious/Routes.pm",
            absent_pl701_modules: &[],
            required_codes: &[],
            required_message_substrings: &[],
            forbidden_message_substrings: &["Mojolicious::Routes::Route::$name"],
        },
        DiagnosticProbe {
            name: "missing_module_probe",
            category: "missing_module_true_unknown",
            file: MISSING_MODULE_PROBE_PATH,
            absent_pl701_modules: &[],
            required_codes: &["PL701"],
            required_message_substrings: &["Definitely::Missing::ForMojoliciousReceipt"],
            forbidden_message_substrings: &[],
        },
        DiagnosticProbe {
            name: "mixed_present_and_missing_modules",
            category: "mixed_false_positive_false_negative_boundary",
            file: MIXED_MODULE_PROBE_PATH,
            absent_pl701_modules: &["Mojo::Base", "Mojolicious::Routes", "Mojolicious::Types"],
            required_codes: &["PL701"],
            required_message_substrings: &["Definitely::Missing::MixedMojoliciousReceipt"],
            forbidden_message_substrings: &[],
        },
    ]
}

fn diagnostic_code(diagnostic: &Value) -> Option<String> {
    diagnostic.get("code").and_then(|code| {
        code.as_str().map(str::to_string).or_else(|| code.as_u64().map(|n| n.to_string()))
    })
}

fn diagnostic_message(diagnostic: &Value) -> &str {
    diagnostic.get("message").and_then(Value::as_str).unwrap_or_default()
}

fn has_diagnostic_code(diagnostic: &Value, code: &str) -> bool {
    diagnostic_code(diagnostic).is_some_and(|actual| actual == code)
}

fn is_valid_position(position: &Value) -> bool {
    position.get("line").and_then(Value::as_u64).is_some()
        && position.get("character").and_then(Value::as_u64).is_some()
}

fn is_valid_diagnostic_shape(diagnostic: &Value) -> bool {
    let Some(range) = diagnostic.get("range") else {
        return false;
    };
    let has_range = range.get("start").is_some_and(is_valid_position)
        && range.get("end").is_some_and(is_valid_position);
    let has_message = diagnostic.get("message").and_then(Value::as_str).is_some();
    let severity_ok = diagnostic
        .get("severity")
        .is_none_or(|severity| severity.as_u64().is_some_and(|n| (1..=4).contains(&n)));
    has_range && has_message && severity_ok
}

fn message_excerpt(message: &str) -> String {
    const MAX_CHARS: usize = 180;
    let mut excerpt = message.chars().take(MAX_CHARS).collect::<String>();
    if message.chars().count() > MAX_CHARS {
        excerpt.push_str("...");
    }
    excerpt
}

fn uri_matches_relative_path(uri: &str, relative_path: &str) -> bool {
    let normalized_uri = uri.replace('\\', "/");
    let normalized_path = relative_path.replace('\\', "/");
    normalized_uri.ends_with(&normalized_path)
}

fn diagnostic_notification_count(harness: &UxHarness, relative_path: &str) -> usize {
    harness
        .peek_notifications()
        .iter()
        .filter(|event| {
            matches!(
                event,
                LspEvent::Diagnostics { uri, .. } if uri_matches_relative_path(uri, relative_path)
            )
        })
        .count()
}

fn matching_pl701_modules(diagnostics: &[Value], modules: &[&str]) -> Vec<String> {
    modules
        .iter()
        .copied()
        .filter(|module| {
            diagnostics.iter().any(|diagnostic| {
                has_diagnostic_code(diagnostic, "PL701")
                    && diagnostic_message(diagnostic).contains(module)
            })
        })
        .map(str::to_string)
        .collect()
}

fn required_code_hits(diagnostics: &[Value], codes: &[&str]) -> Vec<String> {
    codes
        .iter()
        .copied()
        .filter(|code| diagnostics.iter().any(|diagnostic| has_diagnostic_code(diagnostic, code)))
        .map(str::to_string)
        .collect()
}

fn missing_required_codes(diagnostics: &[Value], codes: &[&str]) -> Vec<String> {
    codes
        .iter()
        .copied()
        .filter(|code| !diagnostics.iter().any(|diagnostic| has_diagnostic_code(diagnostic, code)))
        .map(str::to_string)
        .collect()
}

fn required_message_hits(diagnostics: &[Value], substrings: &[&str]) -> Vec<String> {
    substrings
        .iter()
        .copied()
        .filter(|needle| {
            diagnostics.iter().any(|diagnostic| diagnostic_message(diagnostic).contains(needle))
        })
        .map(str::to_string)
        .collect()
}

fn missing_required_message_substrings(diagnostics: &[Value], substrings: &[&str]) -> Vec<String> {
    substrings
        .iter()
        .copied()
        .filter(|needle| {
            !diagnostics.iter().any(|diagnostic| diagnostic_message(diagnostic).contains(needle))
        })
        .map(str::to_string)
        .collect()
}

fn forbidden_message_hits(diagnostics: &[Value], substrings: &[&str]) -> Vec<String> {
    substrings
        .iter()
        .copied()
        .filter(|needle| {
            diagnostics.iter().any(|diagnostic| diagnostic_message(diagnostic).contains(needle))
        })
        .map(str::to_string)
        .collect()
}

fn dynamic_boundary_label_hits(diagnostics: &[Value]) -> Vec<String> {
    ["dynamic", "boundary", "fallback", "low confidence", "unknown"]
        .iter()
        .copied()
        .filter(|needle| {
            diagnostics.iter().any(|diagnostic| {
                diagnostic_message(diagnostic).to_ascii_lowercase().contains(needle)
            })
        })
        .map(str::to_string)
        .collect()
}

fn run_probe(harness: &UxHarness, probe: &DiagnosticProbe) -> DiagnosticProbeReport {
    let diagnostics = harness.wait_for_latest_diagnostics(probe.file, Duration::from_secs(6));
    let notification_count = diagnostic_notification_count(harness, probe.file);
    let invalid_shape_count =
        diagnostics.iter().filter(|diagnostic| !is_valid_diagnostic_shape(diagnostic)).count();
    let pl701_count =
        diagnostics.iter().filter(|diagnostic| has_diagnostic_code(diagnostic, "PL701")).count();
    let false_positive_pl701_modules =
        matching_pl701_modules(&diagnostics, probe.absent_pl701_modules);
    let required_hits = required_code_hits(&diagnostics, probe.required_codes);
    let missing_codes = missing_required_codes(&diagnostics, probe.required_codes);
    let message_hits = required_message_hits(&diagnostics, probe.required_message_substrings);
    let missing_messages =
        missing_required_message_substrings(&diagnostics, probe.required_message_substrings);
    let forbidden_hits = forbidden_message_hits(&diagnostics, probe.forbidden_message_substrings);
    let dynamic_hits = dynamic_boundary_label_hits(&diagnostics);
    let message_excerpts = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.get("message").and_then(Value::as_str))
        .map(message_excerpt)
        .collect::<Vec<_>>();

    DiagnosticProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        diagnostic_count: diagnostics.len(),
        notification_count,
        invalid_shape_count,
        pl701_count,
        false_positive_pl701_modules,
        required_code_hits: required_hits,
        missing_required_codes: missing_codes,
        required_message_hits: message_hits,
        missing_required_message_substrings: missing_messages,
        forbidden_message_hits: forbidden_hits,
        dynamic_boundary_label_hits: dynamic_hits,
        message_excerpts,
        fallback_or_empty: diagnostics.is_empty(),
    }
}

#[test]
fn scenario_31_mojolicious_diagnostics_quality_receipt() {
    run_ux_scenario(
        "mojolicious_diagnostics_quality",
        SCENARIO_FILE,
        "scenario_31_mojolicious_diagnostics_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Diagnostics),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_mojolicious_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            harness.open_file(MISSING_MODULE_PROBE_PATH, MISSING_MODULE_PROBE_SOURCE)?;
            harness.open_file(MIXED_MODULE_PROBE_PATH, MIXED_MODULE_PROBE_SOURCE)?;
            std::thread::sleep(Duration::from_millis(800));

            let probes = diagnostic_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe);
                if report.notification_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "diagnostic_probe={} category={} diagnostics={} pl701={} false_pl701={:?} missing_codes={:?}",
                    report.name,
                    report.category,
                    report.diagnostic_count,
                    report.pl701_count,
                    report.false_positive_pl701_modules,
                    report.missing_required_codes
                );
                reports.push(report);
            }

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let notification_total: usize =
                reports.iter().map(|report| report.notification_count).sum();
            let false_positive_pl701_total: usize =
                reports.iter().map(|report| report.false_positive_pl701_modules.len()).sum();
            let missing_required_code_total: usize =
                reports.iter().map(|report| report.missing_required_codes.len()).sum();
            let missing_required_message_total: usize =
                reports.iter().map(|report| report.missing_required_message_substrings.len()).sum();
            let forbidden_message_total: usize =
                reports.iter().map(|report| report.forbidden_message_hits.len()).sum();
            let fallback_or_empty_count =
                reports.iter().filter(|report| report.fallback_or_empty).count();
            let missing_module_pl701_count = reports
                .iter()
                .find(|report| report.name == "missing_module_probe")
                .map(|report| report.required_code_hits.len())
                .unwrap_or_default();
            let mixed_probe_pl701_count = reports
                .iter()
                .find(|report| report.name == "mixed_present_and_missing_modules")
                .map(|report| report.required_code_hits.len())
                .unwrap_or_default();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "diagnostics",
                "claim_boundary": "real-workspace diagnostics quality receipt only; no provider behavior changed or promoted",
                "fixture_file_count": fixture_files.len(),
                "injected_probe_file_count": 2,
                "probe_count": reports.len(),
                "notification_total": notification_total,
                "invalid_shape_total": invalid_shape_total,
                "false_positive_pl701_total": false_positive_pl701_total,
                "missing_required_code_total": missing_required_code_total,
                "missing_required_message_total": missing_required_message_total,
                "forbidden_message_total": forbidden_message_total,
                "fallback_or_empty_count": fallback_or_empty_count,
                "missing_module_pl701_count": missing_module_pl701_count,
                "mixed_probe_pl701_count": mixed_probe_pl701_count,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_diagnostics_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("all diagnostics probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "diagnostics probes covered all intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_boundary_conservative",
                        "missing_module_true_unknown",
                        "mixed_false_positive_false_negative_boundary",
                        "workspace_module_resolution_no_false_pl701",
                    ]),
            )?;
            recorder.check(
                "diagnostic notifications arrived for selected probes",
                notification_total >= probes.len(),
            )?;
            recorder
                .check("all diagnostics used valid LSP payload shape", invalid_shape_total == 0)?;
            recorder.check(
                "workspace-present Mojolicious imports did not emit PL701",
                false_positive_pl701_total == 0,
            )?;
            recorder.check(
                "dynamic typeglob route registration stayed conservative",
                forbidden_message_total == 0,
            )?;
            recorder.check(
                "missing-module probe emitted PL701",
                missing_module_pl701_count > 0 && missing_required_code_total == 0,
            )?;
            recorder.check(
                "diagnostic PL701 messages identify the required missing modules",
                missing_required_message_total == 0,
            )?;
            recorder.check(
                "mixed present/missing module probe emitted only the missing-module PL701 boundary",
                mixed_probe_pl701_count > 0 && false_positive_pl701_total == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
