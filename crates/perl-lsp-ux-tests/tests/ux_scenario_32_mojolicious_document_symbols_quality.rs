//! Scenario 32 - Mojolicious document-symbol quality receipt.
//!
//! This receipt exercises `textDocument/documentSymbol` over the committed
//! Mojolicious skeleton workspace and records real-workspace quality signals
//! for the source-backed partial-live document-symbol slice without changing
//! provider behavior.
//!
//! Receipt signals:
//! - live document-symbol counts and valid LSP DocumentSymbol shapes
//! - expected source-backed package/sub hits for representative files
//! - generated `has` candidate counts while those labels remain gated
//! - dynamic-boundary-shaped names observed separately from exact symbols
//! - freshness after editing a document so stale symbol names disappear

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness, create_fixture_harness,
    fixture_content, load_mojolicious_fixture_files, open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_32_mojolicious_document_symbols_quality.rs";
const FRESHNESS_FILE: &str = "lib/Mojolicious/Static.pm";

#[derive(Debug)]
struct DocumentSymbolProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    expected_names: &'static [&'static str],
    generated_candidate_names: &'static [&'static str],
    dynamic_boundary_names: &'static [&'static str],
}

#[derive(Debug)]
struct SymbolSummary {
    names: Vec<String>,
    total_symbol_count: usize,
    valid_shape_count: usize,
    invalid_shape_count: usize,
    source_backed_range_count: usize,
}

#[derive(Debug, Serialize)]
struct DocumentSymbolProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    live_symbol_count: usize,
    valid_shape_count: usize,
    invalid_shape_count: usize,
    source_backed_range_count: usize,
    expected_name_hits: Vec<String>,
    missing_expected_names: Vec<String>,
    generated_candidate_count: usize,
    generated_label_hits: Vec<String>,
    dynamic_boundary_candidate_count: usize,
    dynamic_boundary_live_hits: Vec<String>,
    fallback_or_empty: bool,
    symbol_name_sample: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FreshnessReport {
    file: &'static str,
    before_symbol_count: usize,
    after_symbol_count: usize,
    stale_symbol_absent: bool,
    fresh_symbol_present: bool,
    before_hits: Vec<String>,
    after_hits: Vec<String>,
    after_invalid_shape_count: usize,
}

fn is_valid_position(position: &Value) -> bool {
    position.get("line").and_then(Value::as_u64).is_some()
        && position.get("character").and_then(Value::as_u64).is_some()
}

fn is_valid_range(range: &Value) -> bool {
    let Some(start) = range.get("start") else {
        return false;
    };
    let Some(end) = range.get("end") else {
        return false;
    };
    is_valid_position(start) && is_valid_position(end)
}

fn is_valid_document_symbol_shape(symbol: &Value) -> bool {
    let has_name = symbol.get("name").and_then(Value::as_str).is_some();
    let has_kind =
        symbol.get("kind").and_then(Value::as_u64).is_some_and(|kind| (1..=26).contains(&kind));
    let has_range = symbol.get("range").is_some_and(is_valid_range);
    let has_selection_range = symbol.get("selectionRange").is_some_and(is_valid_range);
    has_name && has_kind && has_range && has_selection_range
}

fn collect_symbol_summary(symbols: &[Value]) -> SymbolSummary {
    let mut summary = SymbolSummary {
        names: Vec::new(),
        total_symbol_count: 0,
        valid_shape_count: 0,
        invalid_shape_count: 0,
        source_backed_range_count: 0,
    };
    for symbol in symbols {
        collect_symbol(&mut summary, symbol);
    }
    summary.names.sort();
    summary.names.dedup();
    summary
}

fn collect_symbol(summary: &mut SymbolSummary, symbol: &Value) {
    summary.total_symbol_count += 1;
    if let Some(name) = symbol.get("name").and_then(Value::as_str) {
        summary.names.push(name.to_string());
    }

    if is_valid_document_symbol_shape(symbol) {
        summary.valid_shape_count += 1;
        summary.source_backed_range_count += 1;
    } else {
        summary.invalid_shape_count += 1;
    }

    if let Some(children) = symbol.get("children").and_then(Value::as_array) {
        for child in children {
            collect_symbol(summary, child);
        }
    }
}

fn matching_names(names: &[String], expected: &[&str]) -> Vec<String> {
    expected
        .iter()
        .copied()
        .filter(|name| names.iter().any(|actual| actual == name))
        .map(str::to_string)
        .collect()
}

fn missing_names(names: &[String], expected: &[&str]) -> Vec<String> {
    expected
        .iter()
        .copied()
        .filter(|name| !names.iter().any(|actual| actual == name))
        .map(str::to_string)
        .collect()
}

fn symbol_name_sample(names: &[String]) -> Vec<String> {
    const MAX_NAMES: usize = 16;
    names.iter().take(MAX_NAMES).cloned().collect()
}

fn run_probe(
    harness: &UxHarness,
    probe: &DocumentSymbolProbe,
) -> Result<DocumentSymbolProbeReport> {
    let symbols = harness.document_symbols(probe.file)?;
    let summary = collect_symbol_summary(&symbols);
    let expected_name_hits = matching_names(&summary.names, probe.expected_names);
    let missing_expected_names = missing_names(&summary.names, probe.expected_names);
    let generated_label_hits = matching_names(&summary.names, probe.generated_candidate_names);
    let dynamic_boundary_live_hits = matching_names(&summary.names, probe.dynamic_boundary_names);

    Ok(DocumentSymbolProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        live_symbol_count: summary.total_symbol_count,
        valid_shape_count: summary.valid_shape_count,
        invalid_shape_count: summary.invalid_shape_count,
        source_backed_range_count: summary.source_backed_range_count,
        expected_name_hits,
        missing_expected_names,
        generated_candidate_count: probe.generated_candidate_names.len(),
        generated_label_hits,
        dynamic_boundary_candidate_count: probe.dynamic_boundary_names.len(),
        dynamic_boundary_live_hits,
        fallback_or_empty: summary.total_symbol_count == 0,
        symbol_name_sample: symbol_name_sample(&summary.names),
    })
}

fn freshness_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FreshnessReport> {
    let before_symbols = harness.document_symbols(FRESHNESS_FILE)?;
    let before_summary = collect_symbol_summary(&before_symbols);
    let original = fixture_content(files, FRESHNESS_FILE)?;
    let updated = original.replace("sub serve_asset", "sub serve_blob");
    anyhow::ensure!(updated != original, "freshness fixture must rename serve_asset to serve_blob");

    harness.change_file_full(FRESHNESS_FILE, &updated)?;
    std::thread::sleep(Duration::from_millis(300));
    let after_symbols = harness.document_symbols(FRESHNESS_FILE)?;
    let after_summary = collect_symbol_summary(&after_symbols);

    let before_hits = matching_names(&before_summary.names, &["serve_asset"]);
    let after_hits = matching_names(&after_summary.names, &["serve_asset", "serve_blob"]);
    let stale_symbol_absent = !after_summary.names.iter().any(|name| name == "serve_asset");
    let fresh_symbol_present = after_summary.names.iter().any(|name| name == "serve_blob");

    Ok(FreshnessReport {
        file: FRESHNESS_FILE,
        before_symbol_count: before_summary.total_symbol_count,
        after_symbol_count: after_summary.total_symbol_count,
        stale_symbol_absent,
        fresh_symbol_present,
        before_hits,
        after_hits,
        after_invalid_shape_count: after_summary.invalid_shape_count,
    })
}

fn document_symbol_probes() -> Vec<DocumentSymbolProbe> {
    vec![
        DocumentSymbolProbe {
            name: "app_source_backed_symbols",
            category: "source_backed_explicit_and_generated",
            file: "lib/Mojolicious.pm",
            expected_names: &["Mojolicious", "new", "dispatch", "helper", "start", "startup"],
            generated_candidate_names: &[
                "commands",
                "controller_class",
                "log",
                "plugins",
                "renderer",
                "routes",
                "sessions",
                "static",
                "types",
                "mode",
            ],
            dynamic_boundary_names: &[],
        },
        DocumentSymbolProbe {
            name: "routes_source_backed_symbols",
            category: "source_backed_routes_and_generated",
            file: "lib/Mojolicious/Routes.pm",
            expected_names: &[
                "Mojolicious::Routes",
                "add_condition",
                "add_shortcut",
                "any",
                "get",
                "post",
                "websocket",
                "_dispatch",
            ],
            generated_candidate_names: &[
                "base_classes",
                "cache",
                "conditions",
                "hidden",
                "namespaces",
            ],
            dynamic_boundary_names: &["Mojolicious::Routes::Route::$name", "$name"],
        },
        DocumentSymbolProbe {
            name: "mojo_base_dynamic_boundary_shape",
            category: "dynamic_boundary_shape",
            file: "lib/Mojo/Base.pm",
            expected_names: &[
                "Mojo::Base",
                "import",
                "new",
                "attr",
                "tap",
                "with_roles",
                "_attr",
                "_has",
                "_import_strict",
            ],
            generated_candidate_names: &[],
            dynamic_boundary_names: &[
                "${caller}::has",
                "${caller}::strict",
                "Mojo::Base::_RoleBase",
            ],
        },
    ]
}

#[test]
fn scenario_32_mojolicious_document_symbols_quality_receipt() {
    run_ux_scenario(
        "mojolicious_document_symbols_quality",
        SCENARIO_FILE,
        "scenario_32_mojolicious_document_symbols_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_fixture_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = document_symbol_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                if report.live_symbol_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "document_symbol_probe={} category={} count={} expected_hits={:?} generated_hits={:?} dynamic_hits={:?}",
                    report.name,
                    report.category,
                    report.live_symbol_count,
                    report.expected_name_hits,
                    report.generated_label_hits,
                    report.dynamic_boundary_live_hits
                );
                reports.push(report);
            }

            recorder.mark_request_start("freshness_after_edit");
            let freshness = freshness_report(&harness, &fixture_files)?;
            if freshness.stale_symbol_absent && freshness.fresh_symbol_present {
                recorder.mark_first_useful_result("freshness_after_edit");
            }
            eprintln!(
                "document_symbol_freshness file={} stale_absent={} fresh_present={} before_hits={:?} after_hits={:?}",
                freshness.file,
                freshness.stale_symbol_absent,
                freshness.fresh_symbol_present,
                freshness.before_hits,
                freshness.after_hits
            );

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let live_symbol_total: usize =
                reports.iter().map(|report| report.live_symbol_count).sum();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let missing_expected_symbol_total: usize =
                reports.iter().map(|report| report.missing_expected_names.len()).sum();
            let generated_candidate_total: usize =
                reports.iter().map(|report| report.generated_candidate_count).sum();
            let generated_label_total: usize =
                reports.iter().map(|report| report.generated_label_hits.len()).sum();
            let dynamic_boundary_candidate_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_candidate_count).sum();
            let dynamic_boundary_live_symbol_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_live_hits.len()).sum();
            let fallback_or_empty_count =
                reports.iter().filter(|report| report.fallback_or_empty).count();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "document_symbols",
                "claim_boundary": "real-workspace document-symbol quality receipt only; no provider behavior changed or promoted",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "live_symbol_total": live_symbol_total,
                "invalid_shape_total": invalid_shape_total,
                "missing_expected_symbol_total": missing_expected_symbol_total,
                "generated_candidate_total": generated_candidate_total,
                "generated_label_total": generated_label_total,
                "dynamic_boundary_candidate_total": dynamic_boundary_candidate_total,
                "dynamic_boundary_live_symbol_total": dynamic_boundary_live_symbol_total,
                "fallback_or_empty_count": fallback_or_empty_count,
                "freshness": freshness,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_document_symbols_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all document-symbol probes produced reports",
                reports.len() == probes.len(),
            )?;
            recorder.check(
                "document-symbol probes covered intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_boundary_shape",
                        "source_backed_explicit_and_generated",
                        "source_backed_routes_and_generated",
                    ]),
            )?;
            recorder.check(
                "document-symbol probes returned live symbols",
                live_symbol_total > 0 && fallback_or_empty_count == 0,
            )?;
            recorder.check(
                "all document-symbol entries used valid source-backed LSP shapes",
                invalid_shape_total == 0,
            )?;
            recorder.check(
                "expected source-backed package and sub symbols were present",
                missing_expected_symbol_total == 0,
            )?;
            recorder.check(
                "receipt recorded generated candidates as still gated",
                generated_candidate_total > 0 && generated_label_total == 0,
            )?;
            recorder.check(
                "receipt covered dynamic-boundary-shaped names without requiring promotion",
                dynamic_boundary_candidate_total > 0,
            )?;
            recorder.check(
                "document-symbol freshness after edit removed stale name and surfaced fresh name",
                freshness.stale_symbol_absent
                    && freshness.fresh_symbol_present
                    && freshness.after_invalid_shape_count == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
