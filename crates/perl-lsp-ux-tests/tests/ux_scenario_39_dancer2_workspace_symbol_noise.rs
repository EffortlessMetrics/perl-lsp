//! Scenario 39 - Dancer2 workspace-symbol noise receipt.
//!
//! This receipt exercises `workspace/symbol` over the committed Dancer2
//! skeleton workspace and records a second real-project quality signal for the
//! workspace-symbol provider without changing provider behavior.
//!
//! Receipt signals:
//! - query latency and repeated-request candidate-count delta
//! - useful hits versus unrelated/noisy hits for representative queries
//! - generated/typeglob candidate names while source-backed generated labels stay bounded
//! - dynamic-boundary-shaped names observed separately from exact symbols
//! - stale/fresh query behavior after editing an open document

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness, create_fixture_harness,
    fixture_content, load_dancer2_fixture_files, open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_39_dancer2_workspace_symbol_noise.rs";
const TOP_N: usize = 12;
const FRESHNESS_FILE: &str = "lib/Dancer2/Plugin.pm";

#[derive(Debug)]
struct WorkspaceSymbolProbe {
    name: &'static str,
    category: &'static str,
    query: &'static str,
    useful_substrings: &'static [&'static str],
    generated_candidate_names: &'static [&'static str],
    dynamic_boundary_names: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct WorkspaceSymbolProbeReport {
    name: &'static str,
    category: &'static str,
    query: &'static str,
    first_count: usize,
    second_count: usize,
    candidate_count_delta: usize,
    first_latency_ms: u128,
    second_latency_ms: u128,
    valid_shape_count: usize,
    invalid_shape_count: usize,
    useful_hits: Vec<String>,
    useful_hit_count: usize,
    top_n_noise_count: usize,
    unrelated_hit_count: usize,
    generated_candidate_count: usize,
    generated_candidate_live_hits: Vec<String>,
    generated_label_hits: Vec<String>,
    dynamic_boundary_candidate_count: usize,
    dynamic_boundary_live_hits: Vec<String>,
    stale_or_fallback_label_hits: Vec<String>,
    symbol_name_sample: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FreshnessReport {
    file: &'static str,
    query: &'static str,
    before_count: usize,
    after_count: usize,
    stale_hits_after_edit: Vec<String>,
    fresh_hits_after_edit: Vec<String>,
    stale_symbol_absent: bool,
    fresh_symbol_present: bool,
    before_latency_ms: u128,
    after_latency_ms: u128,
}

struct TimedSymbols {
    symbols: Vec<Value>,
    latency_ms: u128,
}

fn timed_workspace_symbols(harness: &UxHarness, query: &str) -> Result<TimedSymbols> {
    let started = Instant::now();
    let symbols = harness.workspace_symbols(query)?;
    Ok(TimedSymbols { symbols, latency_ms: started.elapsed().as_millis() })
}

fn workspace_symbol_name(symbol: &Value) -> Option<String> {
    symbol.get("name").and_then(Value::as_str).map(ToOwned::to_owned)
}

fn workspace_symbol_text(symbol: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["name", "detail", "containerName"] {
        if let Some(value) = symbol.get(key).and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    parts.join("\n")
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

fn is_valid_workspace_symbol_shape(symbol: &Value) -> bool {
    let has_name = symbol.get("name").and_then(Value::as_str).is_some();
    let has_kind =
        symbol.get("kind").and_then(Value::as_u64).is_some_and(|kind| (1..=26).contains(&kind));
    let Some(location) = symbol.get("location") else {
        return false;
    };
    let has_uri =
        location.get("uri").or_else(|| location.get("targetUri")).and_then(Value::as_str).is_some();
    let has_range = location
        .get("range")
        .or_else(|| location.get("targetSelectionRange"))
        .is_some_and(is_valid_range);
    has_name && has_kind && has_uri && has_range
}

fn symbol_names(symbols: &[Value]) -> Vec<String> {
    let mut names = symbols.iter().filter_map(workspace_symbol_name).collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn symbol_name_sample(names: &[String]) -> Vec<String> {
    names.iter().take(TOP_N).cloned().collect()
}

fn matching_symbol_names(symbols: &[Value], substrings: &[&str]) -> Vec<String> {
    let mut names = symbols
        .iter()
        .filter(|symbol| {
            let haystack = workspace_symbol_text(symbol);
            substrings.iter().any(|needle| haystack.contains(needle))
        })
        .filter_map(workspace_symbol_name)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn exact_unproven_generated_name_hits(
    symbols: &[Value],
    candidate_names: &[&str],
    useful_substrings: &[&str],
) -> Vec<String> {
    let mut names = symbols
        .iter()
        .filter_map(workspace_symbol_name)
        .filter(|name| candidate_names.contains(&name.as_str()))
        .filter(|name| !useful_substrings.iter().any(|useful| name.eq_ignore_ascii_case(useful)))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn count_unrelated_hits(symbols: &[Value], query: &str, useful_substrings: &[&str]) -> usize {
    symbols
        .iter()
        .filter(|symbol| {
            let haystack = workspace_symbol_text(symbol);
            !haystack.contains(query)
                && !useful_substrings.iter().any(|needle| haystack.contains(needle))
        })
        .count()
}

fn count_top_n_noise(names: &[String], useful_substrings: &[&str]) -> usize {
    names
        .iter()
        .take(TOP_N)
        .filter(|name| !useful_substrings.iter().any(|needle| name.contains(needle)))
        .count()
}

fn run_probe(
    harness: &UxHarness,
    probe: &WorkspaceSymbolProbe,
) -> Result<WorkspaceSymbolProbeReport> {
    let first = timed_workspace_symbols(harness, probe.query)?;
    let second = timed_workspace_symbols(harness, probe.query)?;
    let first_names = symbol_names(&first.symbols);

    let valid_shape_count =
        first.symbols.iter().filter(|symbol| is_valid_workspace_symbol_shape(symbol)).count();
    let invalid_shape_count = first.symbols.len().saturating_sub(valid_shape_count);
    let useful_hits = matching_symbol_names(&first.symbols, probe.useful_substrings);
    let generated_candidate_live_hits = exact_unproven_generated_name_hits(
        &first.symbols,
        probe.generated_candidate_names,
        probe.useful_substrings,
    );
    let generated_label_hits = matching_symbol_names(
        &first.symbols,
        &["generated", "framework", "virtual", "FrameworkAdapter"],
    );
    let dynamic_boundary_live_hits =
        matching_symbol_names(&first.symbols, probe.dynamic_boundary_names);
    let stale_or_fallback_label_hits = matching_symbol_names(
        &first.symbols,
        &["stale", "fallback", "low confidence", "dynamic boundary"],
    );

    Ok(WorkspaceSymbolProbeReport {
        name: probe.name,
        category: probe.category,
        query: probe.query,
        first_count: first.symbols.len(),
        second_count: second.symbols.len(),
        candidate_count_delta: first.symbols.len().abs_diff(second.symbols.len()),
        first_latency_ms: first.latency_ms,
        second_latency_ms: second.latency_ms,
        valid_shape_count,
        invalid_shape_count,
        useful_hit_count: useful_hits.len(),
        useful_hits,
        top_n_noise_count: count_top_n_noise(&first_names, probe.useful_substrings),
        unrelated_hit_count: count_unrelated_hits(
            &first.symbols,
            probe.query,
            probe.useful_substrings,
        ),
        generated_candidate_count: probe.generated_candidate_names.len(),
        generated_candidate_live_hits,
        generated_label_hits,
        dynamic_boundary_candidate_count: probe.dynamic_boundary_names.len(),
        dynamic_boundary_live_hits,
        stale_or_fallback_label_hits,
        symbol_name_sample: symbol_name_sample(&first_names),
    })
}

fn freshness_report(harness: &UxHarness, files: &[FixtureFile]) -> Result<FreshnessReport> {
    let before = timed_workspace_symbols(harness, "execute_plugin_")?;
    let original = fixture_content(files, FRESHNESS_FILE)?;
    let updated = original.replace("sub execute_plugin_hook", "sub execute_plugin_signal");
    anyhow::ensure!(
        updated != original,
        "freshness fixture must rename execute_plugin_hook to execute_plugin_signal"
    );

    harness.change_file_full(FRESHNESS_FILE, &updated)?;
    std::thread::sleep(Duration::from_millis(500));

    let after = timed_workspace_symbols(harness, "execute_plugin_")?;
    let stale_hits_after_edit = matching_symbol_names(&after.symbols, &["execute_plugin_hook"]);
    let fresh_hits_after_edit = matching_symbol_names(&after.symbols, &["execute_plugin_signal"]);

    Ok(FreshnessReport {
        file: FRESHNESS_FILE,
        query: "execute_plugin_",
        before_count: before.symbols.len(),
        after_count: after.symbols.len(),
        stale_symbol_absent: stale_hits_after_edit.is_empty(),
        fresh_symbol_present: !fresh_hits_after_edit.is_empty(),
        stale_hits_after_edit,
        fresh_hits_after_edit,
        before_latency_ms: before.latency_ms,
        after_latency_ms: after.latency_ms,
    })
}

fn workspace_symbol_probes() -> Vec<WorkspaceSymbolProbe> {
    vec![
        WorkspaceSymbolProbe {
            name: "package_prefix_dancer2",
            category: "package_prefix_quality",
            query: "Dancer2",
            useful_substrings: &[
                "Dancer2",
                "Dancer2::Core::App",
                "Dancer2::Core::DSL",
                "Dancer2::Core::Runner",
                "Dancer2::Plugin",
            ],
            generated_candidate_names: &["register", "plugin_keywords", "config", "app"],
            dynamic_boundary_names: &[],
        },
        WorkspaceSymbolProbe {
            name: "dsl_symbol_quality",
            category: "dsl_symbol_quality",
            query: "get",
            useful_substrings: &["get", "dsl_keywords", "Dancer2::Core::DSL"],
            generated_candidate_names: &["get", "post", "template", "send_file", "encode_json"],
            dynamic_boundary_names: &["${caller}::${name}", "${plugin_class}::${kw}"],
        },
        WorkspaceSymbolProbe {
            name: "app_dispatch_quality",
            category: "app_dispatch_quality",
            query: "dispatch",
            useful_substrings: &["dispatch", "add_route", "Dancer2::Core::App"],
            generated_candidate_names: &["routes", "hooks", "config"],
            dynamic_boundary_names: &[],
        },
        WorkspaceSymbolProbe {
            name: "plugin_dynamic_boundary_quality",
            category: "plugin_dynamic_boundary_shape",
            query: "plugin",
            useful_substrings: &[
                "Dancer2::Plugin",
                "plugin_keywords",
                "register",
                "execute_plugin_hook",
            ],
            generated_candidate_names: &["register", "plugin_keywords", "plugin_hook"],
            dynamic_boundary_names: &[
                "${caller}::ISA",
                "${caller}::plugin_keywords",
                "${plugin_class}::${kw}",
                "${caller}::register",
            ],
        },
    ]
}

#[test]
fn scenario_39_dancer2_workspace_symbol_noise_receipt() {
    run_ux_scenario(
        "dancer2_workspace_symbol_noise",
        SCENARIO_FILE,
        "scenario_39_dancer2_workspace_symbol_noise_receipt",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_dancer2_fixture_files()?;
            recorder
                .check("dancer2 fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_fixture_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = workspace_symbol_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                if report.first_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "workspace_symbol_probe={} query={} count={} useful_hits={:?} top_noise={} unrelated={} generated_live_hits={:?} generated_labels={:?} dynamic_hits={:?}",
                    report.name,
                    report.query,
                    report.first_count,
                    report.useful_hits,
                    report.top_n_noise_count,
                    report.unrelated_hit_count,
                    report.generated_candidate_live_hits,
                    report.generated_label_hits,
                    report.dynamic_boundary_live_hits
                );
                reports.push(report);
            }

            recorder.mark_request_start("freshness_after_edit");
            let freshness = freshness_report(&harness, &fixture_files)?;
            recorder.mark_first_useful_result("freshness_after_edit");
            eprintln!(
                "workspace_symbol_freshness file={} query={} stale_absent={} fresh_present={} stale_hits={:?} fresh_hits={:?}",
                freshness.file,
                freshness.query,
                freshness.stale_symbol_absent,
                freshness.fresh_symbol_present,
                freshness.stale_hits_after_edit,
                freshness.fresh_hits_after_edit
            );

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let live_symbol_total: usize = reports.iter().map(|report| report.first_count).sum();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let useful_hit_total: usize =
                reports.iter().map(|report| report.useful_hit_count).sum();
            let top_n_noise_total: usize =
                reports.iter().map(|report| report.top_n_noise_count).sum();
            let unrelated_hit_total: usize =
                reports.iter().map(|report| report.unrelated_hit_count).sum();
            let candidate_count_delta_total: usize =
                reports.iter().map(|report| report.candidate_count_delta).sum();
            let generated_candidate_total: usize =
                reports.iter().map(|report| report.generated_candidate_count).sum();
            let generated_live_symbol_total: usize =
                reports.iter().map(|report| report.generated_candidate_live_hits.len()).sum();
            let generated_label_total: usize =
                reports.iter().map(|report| report.generated_label_hits.len()).sum();
            let generated_label_names_are_labeled = reports.iter().all(|report| {
                report
                    .generated_label_hits
                    .iter()
                    .all(|name| name.contains("[generated/framework]"))
            });
            let dynamic_boundary_candidate_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_candidate_count).sum();
            let dynamic_boundary_live_symbol_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_live_hits.len()).sum();
            let stale_or_fallback_label_total: usize =
                reports.iter().map(|report| report.stale_or_fallback_label_hits.len()).sum();
            let max_latency_ms = reports
                .iter()
                .flat_map(|report| [report.first_latency_ms, report.second_latency_ms])
                .chain([freshness.before_latency_ms, freshness.after_latency_ms])
                .max()
                .unwrap_or(0);

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "dancer2",
                "surface": "workspace_symbols",
                "claim_boundary": "second-project workspace-symbol noise receipt only; generated labels may appear only for source-backed pilot symbols and do not promote broad generated/dynamic behavior",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "live_symbol_total": live_symbol_total,
                "useful_hit_total": useful_hit_total,
                "top_n_noise_total": top_n_noise_total,
                "unrelated_hit_total": unrelated_hit_total,
                "candidate_count_delta_total": candidate_count_delta_total,
                "invalid_shape_total": invalid_shape_total,
                "generated_candidate_total": generated_candidate_total,
                "generated_live_symbol_total": generated_live_symbol_total,
                "generated_label_total": generated_label_total,
                "dynamic_boundary_candidate_total": dynamic_boundary_candidate_total,
                "dynamic_boundary_live_symbol_total": dynamic_boundary_live_symbol_total,
                "stale_or_fallback_label_total": stale_or_fallback_label_total,
                "max_latency_ms": max_latency_ms,
                "freshness": freshness,
                "reports": reports,
            });
            eprintln!(
                "dancer2_workspace_symbol_noise_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all workspace-symbol probes produced reports",
                reports.len() == probes.len(),
            )?;
            recorder.check(
                "workspace-symbol probes covered intended receipt categories",
                categories
                    == BTreeSet::from([
                        "app_dispatch_quality",
                        "dsl_symbol_quality",
                        "package_prefix_quality",
                        "plugin_dynamic_boundary_shape",
                    ]),
            )?;
            recorder.check(
                "workspace-symbol probes returned useful project hits",
                useful_hit_total > 0,
            )?;
            recorder.check(
                "workspace-symbol entries used valid LSP shapes",
                invalid_shape_total == 0,
            )?;
            recorder.check(
                "receipt recorded repeated-query candidate count stability",
                candidate_count_delta_total == 0,
            )?;
            recorder.check(
                "receipt recorded generated candidates without exact generated-name promotion",
                generated_candidate_total > 0 && generated_live_symbol_total == 0,
            )?;
            recorder.check(
                "generated-label pilot symbols stayed explicitly labeled",
                generated_label_names_are_labeled,
            )?;
            recorder.check(
                "receipt covered dynamic-boundary-shaped names without requiring promotion",
                dynamic_boundary_candidate_total > 0,
            )?;
            recorder.check(
                "workspace-symbol freshness after edit removed stale name and surfaced fresh name",
                freshness.stale_symbol_absent && freshness.fresh_symbol_present,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
