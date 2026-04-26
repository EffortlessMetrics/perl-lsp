use crate::tasks::metrics::ratchet::{self, SubsystemBaseline};
use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use perl_lsp_ux_tests::{ScenarioScore, aggregate_editor_ux_scorecard};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str =
    "crates/perl-lsp-ux-tests/fixtures/editor_ux_scorecard_measurements.json";
const DEFAULT_OUTPUT: &str = "target/receipts/metrics/editor_ux_scorecard.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/editor_ux.md";

#[derive(Debug, Clone, Copy)]
pub enum UxScorecardFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct ScenarioMeasurement {
    scenario_id: String,
    hover_correct: Option<bool>,
    completion_top1_correct: Option<bool>,
    completion_top5_correct: Option<bool>,
    definition_exact_hit: Option<bool>,
    symbol_correct: Option<bool>,
    cross_file_success: Option<bool>,
    rename_success: Option<bool>,
    diagnostics_correct: Option<bool>,
    latency_ms_by_request: BTreeMap<String, Vec<u64>>,
}

#[derive(Debug, Serialize)]
struct PercentMetric {
    value: Option<f64>,
}

#[derive(Debug, Serialize)]
struct LatencyPercentiles {
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
struct UxScorecardArtifact {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    scenario_count: usize,
    workflow_pass_rate: Option<f64>,
    rows: BTreeMap<String, PercentMetric>,
    latency_by_request_class: BTreeMap<String, LatencyPercentiles>,
    provenance: serde_json::Value,
}

pub fn run(
    format: UxScorecardFormat,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    status_md: Option<PathBuf>,
    ratchet_check: bool,
) -> Result<()> {
    let root = project_root()?;
    let input_path = root.join(input.unwrap_or_else(|| PathBuf::from(DEFAULT_INPUT)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let raw_measurements = load_measurements_raw(&input_path)?;
    let scenarios = load_measurements(&raw_measurements);
    let scorecard = aggregate_editor_ux_scorecard(&scenarios);
    let symbol_correctness_pct =
        percent_true(raw_measurements.iter().filter_map(|s| s.symbol_correct));
    let latencies = compute_latency_percentiles(&raw_measurements);
    let workflow_pass_rate = compute_workflow_pass_rate(&raw_measurements);

    let mut rows = BTreeMap::new();
    rows.insert(
        "hover_correctness_pct".to_string(),
        PercentMetric { value: scorecard.hover_correctness_pct },
    );
    rows.insert(
        "completion_top1_pct".to_string(),
        PercentMetric { value: scorecard.completion_top1_pct },
    );
    rows.insert(
        "completion_top5_pct".to_string(),
        PercentMetric { value: scorecard.completion_top5_pct },
    );
    rows.insert(
        "definition_exact_hit_pct".to_string(),
        PercentMetric { value: scorecard.definition_exact_hit_pct },
    );
    rows.insert(
        "symbol_correctness_pct".to_string(),
        PercentMetric { value: symbol_correctness_pct },
    );
    rows.insert(
        "cross_file_success_pct".to_string(),
        PercentMetric { value: scorecard.cross_file_success_pct },
    );
    rows.insert(
        "rename_success_pct".to_string(),
        PercentMetric { value: scorecard.rename_success_pct },
    );
    rows.insert(
        "diagnostics_correctness_pct".to_string(),
        PercentMetric { value: scorecard.diagnostics_correctness_pct },
    );

    let artifact = UxScorecardArtifact {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "editor_ux",
        scenario_count: scorecard.scenario_count,
        workflow_pass_rate,
        rows,
        latency_by_request_class: latencies,
        provenance: json!({
            "input": path_relative_to_root(&root, &input_path),
            "generator": "cargo xtask ux-scorecard --format json",
            "ratchet_policy": "regression_only"
        }),
    };

    write_json(&output_path, &artifact)?;
    fs::write(&status_path, render_status_markdown(&artifact, &raw_measurements))
        .with_context(|| format!("writing {}", status_path.display()))?;
    maybe_embed_receipt_block(&root, &artifact)?;

    if ratchet_check {
        enforce_ratchet(&root, &artifact)?;
    }

    match format {
        UxScorecardFormat::Human => {
            println!("UX scorecard updated: {}", output_path.display());
            println!("Status page updated: {}", status_path.display());
        }
        UxScorecardFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&artifact)?);
        }
    }

    Ok(())
}

fn load_measurements(raw: &[ScenarioMeasurement]) -> Vec<ScenarioScore> {
    raw.iter()
        .map(|m| ScenarioScore {
            scenario_id: m.scenario_id.clone(),
            hover_correct: m.hover_correct,
            completion_top1_correct: m.completion_top1_correct,
            completion_top5_correct: m.completion_top5_correct,
            definition_exact_hit: m.definition_exact_hit,
            cross_file_success: m.cross_file_success,
            rename_success: m.rename_success,
            diagnostics_correct: m.diagnostics_correct,
            mean_latency_ms: BTreeMap::new(),
        })
        .collect()
}

fn load_measurements_raw(path: &Path) -> Result<Vec<ScenarioMeasurement>> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let rows = serde_json::from_str::<Vec<ScenarioMeasurement>>(&raw)
        .with_context(|| format!("parsing {}", path.display()))?;
    if rows.is_empty() {
        bail!("measurement fixture is empty: {}", path.display());
    }
    Ok(rows)
}

fn write_json(path: &Path, artifact: &UxScorecardArtifact) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(artifact)?;
    fs::write(path, format!("{payload}\n")).with_context(|| format!("writing {}", path.display()))
}

fn render_status_markdown(
    artifact: &UxScorecardArtifact,
    raw: &[ScenarioMeasurement],
) -> String {
    let mut text = String::new();

    // Header
    text.push_str("# Editor UX Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&format!("Scenarios: `{}`\n\n", artifact.scenario_count));

    // Top-line summary
    let pass_rate_str = artifact
        .workflow_pass_rate
        .map(|r| format!("{:.1}%", r))
        .unwrap_or_else(|| "n/a".to_string());
    let p95_hover = artifact
        .latency_by_request_class
        .get("hover")
        .and_then(|l| l.p95_ms)
        .map(|ms| format!("{ms:.0}ms"))
        .unwrap_or_else(|| "n/a".to_string());
    text.push_str("## Summary\n\n");
    text.push_str("| | |\n|---|---|\n");
    text.push_str(&format!("| Workflow pass rate | **{pass_rate_str}** |\n"));
    text.push_str(&format!("| Scenarios measured | {} |\n", artifact.scenario_count));
    text.push_str(&format!("| Hover p95 latency | {p95_hover} |\n"));
    text.push('\n');

    // Correctness metrics
    text.push_str("## Correctness\n\n| Metric | Value |\n|---|---:|\n");
    for (k, v) in &artifact.rows {
        let value = v.value.map(|n| format!("{n:.2}%")).unwrap_or_else(|| "n/a".to_string());
        text.push_str(&format!("| {k} | {value} |\n"));
    }
    text.push('\n');

    // Latency table
    text.push_str("## Latency (ms)\n\n| Request class | p50 | p95 |\n|---|---:|---:|\n");
    for (k, v) in &artifact.latency_by_request_class {
        let p50 = v.p50_ms.map(|n| format!("{n:.2}")).unwrap_or_else(|| "n/a".to_string());
        let p95 = v.p95_ms.map(|n| format!("{n:.2}")).unwrap_or_else(|| "n/a".to_string());
        text.push_str(&format!("| {k} | {p50} | {p95} |\n"));
    }
    text.push('\n');

    // Per-scenario breakdown
    text.push_str("## Scenarios\n\n");
    text.push_str("| Scenario | Hover | Completion | Definition | Symbol | Cross-file | Rename | Diagnostics |\n");
    text.push_str("|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|\n");
    for m in raw {
        let cell = |v: Option<bool>| match v {
            Some(true) => "✓",
            Some(false) => "✗",
            None => "—",
        };
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            m.scenario_id,
            cell(m.hover_correct),
            cell(m.completion_top1_correct),
            cell(m.definition_exact_hit),
            cell(m.symbol_correct),
            cell(m.cross_file_success),
            cell(m.rename_success),
            cell(m.diagnostics_correct),
        ));
    }
    text.push('\n');

    // Ratchet policy
    text.push_str("## Ratchet policy\n\nRegression-only ratchet: floor metrics may improve or stay flat; any statistically meaningful regression fails `cargo xtask ux-scorecard --ratchet-check`.\n");
    text
}

/// A scenario "passes" if every metric it exercises is true (nulls are skipped).
fn compute_workflow_pass_rate(scenarios: &[ScenarioMeasurement]) -> Option<f64> {
    if scenarios.is_empty() {
        return None;
    }
    let mut pass = 0usize;
    for s in scenarios {
        let checks: &[Option<bool>] = &[
            s.hover_correct,
            s.completion_top1_correct,
            s.completion_top5_correct,
            s.definition_exact_hit,
            s.symbol_correct,
            s.cross_file_success,
            s.rename_success,
            s.diagnostics_correct,
        ];
        let any_measured = checks.iter().any(|c| c.is_some());
        let all_pass = checks.iter().filter_map(|c| *c).all(|v| v);
        if any_measured && all_pass {
            pass += 1;
        }
    }
    Some((pass as f64 / scenarios.len() as f64) * 100.0)
}

fn compute_latency_percentiles(
    scenarios: &[ScenarioMeasurement],
) -> BTreeMap<String, LatencyPercentiles> {
    let mut by_request: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for scenario in scenarios {
        for (request, samples) in &scenario.latency_ms_by_request {
            let entry = by_request.entry(request.clone()).or_default();
            entry.extend(samples.iter().copied());
        }
    }

    by_request
        .into_iter()
        .map(|(request, mut samples)| {
            samples.sort_unstable();
            (
                request,
                LatencyPercentiles {
                    p50_ms: percentile(&samples, 0.50),
                    p95_ms: percentile(&samples, 0.95),
                },
            )
        })
        .collect()
}

fn percentile(samples: &[u64], pct: f64) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let rank = ((samples.len() - 1) as f64 * pct).round() as usize;
    samples.get(rank).map(|value| *value as f64)
}

fn percent_true<I>(iter: I) -> Option<f64>
where
    I: Iterator<Item = bool>,
{
    let mut total = 0usize;
    let mut success = 0usize;
    for v in iter {
        total += 1;
        if v {
            success += 1;
        }
    }
    if total == 0 { None } else { Some((success as f64 / total as f64) * 100.0) }
}

fn maybe_embed_receipt_block(root: &Path, artifact: &UxScorecardArtifact) -> Result<()> {
    let receipt_path = root.join("target/receipts/receipt.json");
    if !receipt_path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&receipt_path)
        .with_context(|| format!("reading {}", receipt_path.display()))?;
    let mut json_value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", receipt_path.display()))?;
    if let Some(object) = json_value.as_object_mut() {
        object.insert(
            "ux_scorecard".to_string(),
            json!({
                "workflow_pass_rate": artifact.workflow_pass_rate,
                "hover_correctness_pct": artifact.rows.get("hover_correctness_pct").and_then(|m| m.value),
                "completion_top1_pct": artifact.rows.get("completion_top1_pct").and_then(|m| m.value),
                "completion_top5_pct": artifact.rows.get("completion_top5_pct").and_then(|m| m.value),
                "definition_exact_hit_pct": artifact.rows.get("definition_exact_hit_pct").and_then(|m| m.value),
                "symbol_correctness_pct": artifact.rows.get("symbol_correctness_pct").and_then(|m| m.value),
                "cross_file_success_pct": artifact.rows.get("cross_file_success_pct").and_then(|m| m.value),
                "rename_success_pct": artifact.rows.get("rename_success_pct").and_then(|m| m.value),
                "diagnostics_correctness_pct": artifact.rows.get("diagnostics_correctness_pct").and_then(|m| m.value),
            }),
        );
    }
    fs::write(&receipt_path, format!("{}\n", serde_json::to_string_pretty(&json_value)?))
        .with_context(|| format!("writing {}", receipt_path.display()))
}

fn enforce_ratchet(root: &Path, artifact: &UxScorecardArtifact) -> Result<()> {
    let baseline_path = root.join(".ci/metrics/baselines/editor_ux.json");
    let baseline_raw = fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline: SubsystemBaseline = serde_json::from_str(&baseline_raw)
        .with_context(|| format!("parsing {}", baseline_path.display()))?;

    let mut current_floor = BTreeMap::new();
    for (k, v) in &artifact.rows {
        current_floor.insert(k.clone(), v.value);
    }
    if let Some(wpr) = artifact.workflow_pass_rate {
        current_floor.insert("workflow_pass_rate".to_string(), Some(wpr));
    }
    for (request, latency) in &artifact.latency_by_request_class {
        current_floor.insert(format!("latency_{}_p50_ms", request), latency.p50_ms);
        current_floor.insert(format!("latency_{}_p95_ms", request), latency.p95_ms);
    }

    let violations = ratchet::check_floor_metrics(&baseline, &current_floor);
    if violations.is_empty() {
        return Ok(());
    }

    for violation in &violations {
        eprintln!(
            "VIOLATION [editor_ux] {} baseline={:.3} current={:.3} regression={:.2}%",
            violation.metric,
            violation.baseline_value,
            violation.current_value,
            violation.regression_pct * 100.0
        );
    }

    bail!("editor_ux ratchet check failed with {} violation(s)", violations.len())
}

fn path_relative_to_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_percentiles() {
        let rows = vec![ScenarioMeasurement {
            scenario_id: "s1".to_string(),
            hover_correct: Some(true),
            completion_top1_correct: Some(true),
            completion_top5_correct: Some(true),
            definition_exact_hit: Some(true),
            symbol_correct: Some(true),
            cross_file_success: Some(true),
            rename_success: None,
            diagnostics_correct: None,
            latency_ms_by_request: BTreeMap::from([("hover".to_string(), vec![10, 20, 30, 40])]),
        }];

        let latency = compute_latency_percentiles(&rows);
        let hover = latency.get("hover");
        assert!(hover.is_some());
        if let Some(metrics) = hover {
            assert_eq!(metrics.p50_ms, Some(30.0));
            assert_eq!(metrics.p95_ms, Some(40.0));
        }
    }

    #[test]
    fn workflow_pass_rate_counts_all_pass_scenarios() {
        let rows = vec![
            ScenarioMeasurement {
                scenario_id: "all-pass".to_string(),
                hover_correct: Some(true),
                completion_top1_correct: Some(true),
                completion_top5_correct: None,
                definition_exact_hit: None,
                symbol_correct: None,
                cross_file_success: None,
                rename_success: None,
                diagnostics_correct: None,
                latency_ms_by_request: BTreeMap::new(),
            },
            ScenarioMeasurement {
                scenario_id: "one-fail".to_string(),
                hover_correct: Some(false),
                completion_top1_correct: Some(true),
                completion_top5_correct: None,
                definition_exact_hit: None,
                symbol_correct: None,
                cross_file_success: None,
                rename_success: None,
                diagnostics_correct: None,
                latency_ms_by_request: BTreeMap::new(),
            },
            ScenarioMeasurement {
                scenario_id: "rename-pass".to_string(),
                hover_correct: None,
                completion_top1_correct: None,
                completion_top5_correct: None,
                definition_exact_hit: None,
                symbol_correct: None,
                cross_file_success: None,
                rename_success: Some(true),
                diagnostics_correct: Some(true),
                latency_ms_by_request: BTreeMap::new(),
            },
        ];
        // 2 out of 3 scenarios pass all their checks
        let rate = compute_workflow_pass_rate(&rows);
        assert!(rate.is_some());
        let pct = rate.unwrap();
        // "all-pass" and "rename-pass" pass; "one-fail" fails
        assert!((pct - 66.666_666_f64).abs() < 0.01, "expected ~66.7%, got {pct}");
    }

    /// Verifies the full measurement→rows→latency pipeline produces consistent
    /// values and that the artifact serialization round-trips without data loss.
    #[test]
    fn pipeline_round_trip_produces_correct_rows() {
        let raw = vec![
            ScenarioMeasurement {
                scenario_id: "hover_test".to_string(),
                hover_correct: Some(true),
                completion_top1_correct: None,
                completion_top5_correct: None,
                definition_exact_hit: None,
                symbol_correct: Some(true),
                cross_file_success: None,
                rename_success: None,
                diagnostics_correct: None,
                latency_ms_by_request: BTreeMap::from([("hover".to_string(), vec![10, 20, 30])]),
            },
            ScenarioMeasurement {
                scenario_id: "completion_test".to_string(),
                hover_correct: Some(false),
                completion_top1_correct: Some(true),
                completion_top5_correct: Some(true),
                definition_exact_hit: None,
                symbol_correct: Some(false),
                cross_file_success: Some(true),
                rename_success: Some(true),
                diagnostics_correct: Some(true),
                latency_ms_by_request: BTreeMap::from([
                    ("completion".to_string(), vec![5, 15, 25]),
                    ("hover".to_string(), vec![50, 60, 70]),
                ]),
            },
        ];

        let scenarios = load_measurements(&raw);
        let scorecard = aggregate_editor_ux_scorecard(&scenarios);

        assert_eq!(scorecard.hover_correctness_pct, Some(50.0));
        assert_eq!(scorecard.completion_top1_pct, Some(100.0));
        assert_eq!(scorecard.completion_top5_pct, Some(100.0));
        assert_eq!(scorecard.definition_exact_hit_pct, None);
        assert_eq!(scorecard.cross_file_success_pct, Some(100.0));
        assert_eq!(scorecard.rename_success_pct, Some(100.0));
        assert_eq!(scorecard.diagnostics_correctness_pct, Some(100.0));

        let symbol_pct = percent_true(raw.iter().filter_map(|s| s.symbol_correct));
        assert_eq!(symbol_pct, Some(50.0));

        let latencies = compute_latency_percentiles(&raw);
        let hover_lat = latencies.get("hover").expect("hover latency present");
        assert_eq!(hover_lat.p50_ms, Some(50.0));
        assert_eq!(hover_lat.p95_ms, Some(70.0));

        let comp_lat = latencies.get("completion").expect("completion latency present");
        assert_eq!(comp_lat.p50_ms, Some(15.0));
        assert_eq!(comp_lat.p95_ms, Some(25.0));

        // Pass rate: hover_test passes (hover=true); completion_test fails (hover=false).
        let wpr = compute_workflow_pass_rate(&raw);
        assert_eq!(wpr, Some(50.0));

        let mut rows = BTreeMap::new();
        rows.insert(
            "hover_correctness_pct".to_string(),
            PercentMetric { value: scorecard.hover_correctness_pct },
        );
        rows.insert("symbol_correctness_pct".to_string(), PercentMetric { value: symbol_pct });
        rows.insert(
            "rename_success_pct".to_string(),
            PercentMetric { value: scorecard.rename_success_pct },
        );
        rows.insert(
            "diagnostics_correctness_pct".to_string(),
            PercentMetric { value: scorecard.diagnostics_correctness_pct },
        );

        let artifact = UxScorecardArtifact {
            schema_version: 1,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            scenario_count: scorecard.scenario_count,
            workflow_pass_rate: wpr,
            rows,
            latency_by_request_class: latencies,
            provenance: serde_json::json!({"input": "fixture", "generator": "test"}),
        };

        let json_str =
            serde_json::to_string_pretty(&artifact).expect("artifact must serialize without error");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("serialized artifact must parse back");

        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["subsystem"], "editor_ux");
        assert_eq!(parsed["workflow_pass_rate"], serde_json::json!(50.0));
        assert_eq!(parsed["rows"]["hover_correctness_pct"]["value"], serde_json::json!(50.0));
        assert_eq!(parsed["rows"]["rename_success_pct"]["value"], serde_json::json!(100.0));
        assert_eq!(parsed["rows"]["diagnostics_correctness_pct"]["value"], serde_json::json!(100.0));
        assert!(parsed["latency_by_request_class"]["hover"]["p50_ms"].is_number());
        assert!(parsed["latency_by_request_class"]["completion"]["p95_ms"].is_number());
    }

    #[test]
    fn percent_true_returns_none_on_empty_iterator() {
        let result = percent_true(std::iter::empty::<bool>());
        assert_eq!(result, None);
    }

    #[test]
    fn single_sample_latency_p50_equals_p95() {
        let rows = vec![ScenarioMeasurement {
            scenario_id: "single".to_string(),
            hover_correct: None,
            completion_top1_correct: None,
            completion_top5_correct: None,
            definition_exact_hit: None,
            symbol_correct: None,
            cross_file_success: None,
            rename_success: None,
            diagnostics_correct: None,
            latency_ms_by_request: BTreeMap::from([("definition".to_string(), vec![42])]),
        }];
        let latency = compute_latency_percentiles(&rows);
        let def = latency.get("definition").expect("definition present");
        assert_eq!(def.p50_ms, Some(42.0));
        assert_eq!(def.p95_ms, Some(42.0));
    }

    #[test]
    fn render_markdown_includes_scenario_table_and_summary() {
        let raw = vec![ScenarioMeasurement {
            scenario_id: "hover_core".to_string(),
            hover_correct: Some(true),
            completion_top1_correct: None,
            completion_top5_correct: None,
            definition_exact_hit: None,
            symbol_correct: None,
            cross_file_success: None,
            rename_success: None,
            diagnostics_correct: None,
            latency_ms_by_request: BTreeMap::from([("hover".to_string(), vec![20, 30])]),
        }];
        let latencies = compute_latency_percentiles(&raw);
        let wpr = compute_workflow_pass_rate(&raw);
        let mut rows = BTreeMap::new();
        rows.insert(
            "hover_correctness_pct".to_string(),
            PercentMetric { value: Some(100.0) },
        );
        let artifact = UxScorecardArtifact {
            schema_version: 1,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            scenario_count: 1,
            workflow_pass_rate: wpr,
            rows,
            latency_by_request_class: latencies,
            provenance: serde_json::json!({}),
        };
        let md = render_status_markdown(&artifact, &raw);
        assert!(md.contains("## Summary"), "missing Summary section");
        assert!(md.contains("Workflow pass rate"), "missing pass rate row");
        assert!(md.contains("## Scenarios"), "missing Scenarios table");
        assert!(md.contains("hover_core"), "missing scenario row");
        assert!(md.contains("## Ratchet policy"), "missing ratchet section");
    }
}
