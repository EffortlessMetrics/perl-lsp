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
const FIXTURE_MATRIX: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json";
const BASELINE_PATH: &str = ".ci/metrics/baselines/editor_ux.json";

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
    diagnostics_correct: Option<bool>,
    rename_success: Option<bool>,
    cross_file_success: Option<bool>,
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
    scenario_ids: Vec<String>,
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
    let latencies = compute_latency_percentiles(&raw_measurements);
    let scenario_ids: Vec<String> =
        raw_measurements.iter().map(|m| m.scenario_id.clone()).collect();

    let declared_scenario_count = load_declared_scenario_count(&root);

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
        PercentMetric { value: scorecard.symbol_correctness_pct },
    );
    rows.insert(
        "diagnostics_correct_pct".to_string(),
        PercentMetric { value: scorecard.diagnostics_correct_pct },
    );
    rows.insert(
        "rename_success_pct".to_string(),
        PercentMetric { value: scorecard.rename_success_pct },
    );
    rows.insert(
        "cross_file_success_pct".to_string(),
        PercentMetric { value: scorecard.cross_file_success_pct },
    );

    let artifact = UxScorecardArtifact {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "editor_ux",
        scenario_count: scorecard.scenario_count,
        scenario_ids,
        rows,
        latency_by_request_class: latencies,
        provenance: json!({
            "input": path_relative_to_root(&root, &input_path),
            "generator": "cargo xtask ux-scorecard --format json",
            "ratchet_policy": "regression_only",
            "declared_scenario_count": declared_scenario_count
        }),
    };

    let baseline = load_baseline_opt(&root);
    write_json(&output_path, &artifact)?;
    fs::write(&status_path, render_status_markdown(&artifact, baseline.as_ref()))
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
        .map(|m| {
            let mean_latency_ms = m
                .latency_ms_by_request
                .iter()
                .filter_map(|(class, samples)| {
                    if samples.is_empty() {
                        return None;
                    }
                    let mean = samples.iter().sum::<u64>() as f64 / samples.len() as f64;
                    Some((class.clone(), mean))
                })
                .collect();
            ScenarioScore {
                scenario_id: m.scenario_id.clone(),
                hover_correct: m.hover_correct,
                completion_top1_correct: m.completion_top1_correct,
                completion_top5_correct: m.completion_top5_correct,
                definition_exact_hit: m.definition_exact_hit,
                symbol_correct: m.symbol_correct,
                diagnostics_correct: m.diagnostics_correct,
                rename_success: m.rename_success,
                cross_file_success: m.cross_file_success,
                mean_latency_ms,
            }
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

fn load_declared_scenario_count(root: &Path) -> Option<usize> {
    let path = root.join(FIXTURE_MATRIX);
    let raw = fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value.get("workflows")?.as_array().map(|arr| arr.len())
}

fn load_baseline_opt(root: &Path) -> Option<SubsystemBaseline> {
    let path = root.join(BASELINE_PATH);
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
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
    baseline: Option<&SubsystemBaseline>,
) -> String {
    let declared = artifact
        .provenance
        .get("declared_scenario_count")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let coverage_line = match declared {
        Some(total) if total > 0 => {
            let pct = (artifact.scenario_count as f64 / total as f64 * 100.0).round() as usize;
            format!(
                "Scenarios measured: `{}` of `{}` declared ({pct}% fixture coverage)  \n",
                artifact.scenario_count, total
            )
        }
        _ => format!("Scenarios: `{}`  \n", artifact.scenario_count),
    };

    let mut text = String::new();
    text.push_str("# Editor UX Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&coverage_line);

    if !artifact.scenario_ids.is_empty() {
        text.push('\n');
        text.push_str("**Measured scenarios:** ");
        let ids: Vec<&str> = artifact.scenario_ids.iter().map(String::as_str).collect();
        text.push_str(&ids.join(", "));
        text.push_str("  \n");
    }

    text.push_str("\n## Correctness\n\n| Metric | Value |\n|---|---:|\n");
    for (k, v) in &artifact.rows {
        let value = v.value.map(|n| format!("{n:.2}%")).unwrap_or_else(|| "n/a".to_string());
        text.push_str(&format!("| {k} | {value} |\n"));
    }

    text.push_str("\n## Latency (ms)\n\n");
    if baseline.is_some() {
        text.push_str("| Request class | p50 | p50 baseline | p95 | p95 baseline |\n");
        text.push_str("|---|---:|---:|---:|---:|\n");
    } else {
        text.push_str("| Request class | p50 | p95 |\n|---|---:|---:|\n");
    }
    for (k, v) in &artifact.latency_by_request_class {
        let p50 = v.p50_ms.map(|n| format!("{n:.2}")).unwrap_or_else(|| "n/a".to_string());
        let p95 = v.p95_ms.map(|n| format!("{n:.2}")).unwrap_or_else(|| "n/a".to_string());
        if let Some(bl) = baseline {
            let bl50_key = format!("latency_{k}_p50_ms");
            let bl95_key = format!("latency_{k}_p95_ms");
            let bl50 = bl
                .floor_metrics
                .get(&bl50_key)
                .and_then(|n| *n)
                .map(|n| format!("{n:.2}"))
                .unwrap_or_else(|| "—".to_string());
            let bl95 = bl
                .floor_metrics
                .get(&bl95_key)
                .and_then(|n| *n)
                .map(|n| format!("{n:.2}"))
                .unwrap_or_else(|| "—".to_string());
            text.push_str(&format!("| {k} | {p50} | {bl50} | {p95} | {bl95} |\n"));
        } else {
            text.push_str(&format!("| {k} | {p50} | {p95} |\n"));
        }
    }

    text.push_str("\n## Ratchet policy\n\nRegression-only ratchet: floor metrics may improve or stay flat; any statistically meaningful regression fails `cargo xtask ux-scorecard --ratchet-check`.\n");
    text
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
                "hover_correctness_pct": artifact.rows.get("hover_correctness_pct").and_then(|m| m.value),
                "completion_top1_pct": artifact.rows.get("completion_top1_pct").and_then(|m| m.value),
                "completion_top5_pct": artifact.rows.get("completion_top5_pct").and_then(|m| m.value),
                "definition_exact_hit_pct": artifact.rows.get("definition_exact_hit_pct").and_then(|m| m.value),
                "symbol_correctness_pct": artifact.rows.get("symbol_correctness_pct").and_then(|m| m.value),
                "diagnostics_correct_pct": artifact.rows.get("diagnostics_correct_pct").and_then(|m| m.value),
                "rename_success_pct": artifact.rows.get("rename_success_pct").and_then(|m| m.value),
                "cross_file_success_pct": artifact.rows.get("cross_file_success_pct").and_then(|m| m.value)
            }),
        );
    }
    fs::write(&receipt_path, format!("{}\n", serde_json::to_string_pretty(&json_value)?))
        .with_context(|| format!("writing {}", receipt_path.display()))
}

fn enforce_ratchet(root: &Path, artifact: &UxScorecardArtifact) -> Result<()> {
    let baseline_path = root.join(BASELINE_PATH);
    let baseline_raw = fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline: SubsystemBaseline = serde_json::from_str(&baseline_raw)
        .with_context(|| format!("parsing {}", baseline_path.display()))?;

    let mut current_floor = BTreeMap::new();
    for (k, v) in &artifact.rows {
        current_floor.insert(k.clone(), v.value);
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
            diagnostics_correct: Some(true),
            rename_success: None,
            cross_file_success: Some(true),
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

    /// load_measurements propagates mean latency from raw samples into ScenarioScore.
    #[test]
    fn load_measurements_populates_mean_latency() {
        let raw = vec![ScenarioMeasurement {
            scenario_id: "latency_test".to_string(),
            hover_correct: None,
            completion_top1_correct: None,
            completion_top5_correct: None,
            definition_exact_hit: None,
            symbol_correct: None,
            diagnostics_correct: None,
            rename_success: None,
            cross_file_success: None,
            latency_ms_by_request: BTreeMap::from([
                ("hover".to_string(), vec![10, 20, 30]),
                ("completion".to_string(), vec![5, 15]),
            ]),
        }];
        let scenarios = load_measurements(&raw);
        assert_eq!(scenarios.len(), 1);
        let s = &scenarios[0];
        assert_eq!(s.mean_latency_ms.get("hover"), Some(&20.0));
        assert_eq!(s.mean_latency_ms.get("completion"), Some(&10.0));
    }

    /// load_measurements populates all new correctness fields.
    #[test]
    fn load_measurements_maps_all_correctness_fields() {
        let raw = vec![ScenarioMeasurement {
            scenario_id: "full_fields".to_string(),
            hover_correct: Some(true),
            completion_top1_correct: Some(false),
            completion_top5_correct: Some(true),
            definition_exact_hit: Some(true),
            symbol_correct: Some(true),
            diagnostics_correct: Some(false),
            rename_success: Some(true),
            cross_file_success: Some(true),
            latency_ms_by_request: BTreeMap::new(),
        }];
        let scenarios = load_measurements(&raw);
        let s = &scenarios[0];
        assert_eq!(s.hover_correct, Some(true));
        assert_eq!(s.completion_top1_correct, Some(false));
        assert_eq!(s.symbol_correct, Some(true));
        assert_eq!(s.diagnostics_correct, Some(false));
        assert_eq!(s.rename_success, Some(true));
        assert_eq!(s.cross_file_success, Some(true));
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
                diagnostics_correct: Some(true),
                rename_success: None,
                cross_file_success: None,
                latency_ms_by_request: BTreeMap::from([("hover".to_string(), vec![10, 20, 30])]),
            },
            ScenarioMeasurement {
                scenario_id: "completion_test".to_string(),
                hover_correct: Some(false),
                completion_top1_correct: Some(true),
                completion_top5_correct: Some(true),
                definition_exact_hit: None,
                symbol_correct: Some(false),
                diagnostics_correct: None,
                rename_success: Some(true),
                cross_file_success: Some(true),
                latency_ms_by_request: BTreeMap::from([
                    ("completion".to_string(), vec![5, 15, 25]),
                    ("hover".to_string(), vec![50, 60, 70]),
                ]),
            },
        ];

        let scenarios = load_measurements(&raw);
        let scorecard = aggregate_editor_ux_scorecard(&scenarios);

        // hover_correct: true, false → 50%
        assert_eq!(scorecard.hover_correctness_pct, Some(50.0));
        // completion_top1: only scenario 2 measured → 100%
        assert_eq!(scorecard.completion_top1_pct, Some(100.0));
        // completion_top5: only scenario 2 measured → 100%
        assert_eq!(scorecard.completion_top5_pct, Some(100.0));
        // definition_exact_hit: neither measured → None
        assert_eq!(scorecard.definition_exact_hit_pct, None);
        // symbol_correct: true, false → 50%
        assert_eq!(scorecard.symbol_correctness_pct, Some(50.0));
        // diagnostics_correct: only scenario 1 measured → 100%
        assert_eq!(scorecard.diagnostics_correct_pct, Some(100.0));
        // rename_success: only scenario 2 measured → 100%
        assert_eq!(scorecard.rename_success_pct, Some(100.0));
        // cross_file_success: only scenario 2 measured → 100%
        assert_eq!(scorecard.cross_file_success_pct, Some(100.0));

        let latencies = compute_latency_percentiles(&raw);

        // "hover" samples: [10, 20, 30, 50, 60, 70] sorted.
        // p50: rank = (6-1)*0.50 = 2.5 → 3, samples[3]=50
        // p95: rank = (6-1)*0.95 = 4.75 → 5, samples[5]=70
        let hover_lat = latencies.get("hover").expect("hover latency present");
        assert_eq!(hover_lat.p50_ms, Some(50.0));
        assert_eq!(hover_lat.p95_ms, Some(70.0));

        // "completion" samples: [5, 15, 25] sorted.
        // p50: rank = (3-1)*0.50 = 1.0 → 1, samples[1]=15
        // p95: rank = (3-1)*0.95 = 1.9 → 2, samples[2]=25
        let comp_lat = latencies.get("completion").expect("completion latency present");
        assert_eq!(comp_lat.p50_ms, Some(15.0));
        assert_eq!(comp_lat.p95_ms, Some(25.0));

        let mut rows = BTreeMap::new();
        rows.insert(
            "hover_correctness_pct".to_string(),
            PercentMetric { value: scorecard.hover_correctness_pct },
        );
        rows.insert(
            "symbol_correctness_pct".to_string(),
            PercentMetric { value: scorecard.symbol_correctness_pct },
        );
        rows.insert(
            "diagnostics_correct_pct".to_string(),
            PercentMetric { value: scorecard.diagnostics_correct_pct },
        );
        rows.insert(
            "rename_success_pct".to_string(),
            PercentMetric { value: scorecard.rename_success_pct },
        );

        let artifact = UxScorecardArtifact {
            schema_version: 1,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            scenario_count: scorecard.scenario_count,
            scenario_ids: vec!["hover_test".to_string(), "completion_test".to_string()],
            rows,
            latency_by_request_class: latencies,
            provenance: serde_json::json!({"input": "fixture", "generator": "test", "declared_scenario_count": 22}),
        };

        let json_str =
            serde_json::to_string_pretty(&artifact).expect("artifact must serialize without error");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("serialized artifact must parse back");

        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["subsystem"], "editor_ux");
        assert_eq!(parsed["rows"]["hover_correctness_pct"]["value"], serde_json::json!(50.0));
        assert_eq!(parsed["rows"]["symbol_correctness_pct"]["value"], serde_json::json!(50.0));
        assert_eq!(parsed["rows"]["diagnostics_correct_pct"]["value"], serde_json::json!(100.0));
        assert_eq!(parsed["rows"]["rename_success_pct"]["value"], serde_json::json!(100.0));
        assert!(parsed["latency_by_request_class"]["hover"]["p50_ms"].is_number());
        assert!(parsed["latency_by_request_class"]["completion"]["p95_ms"].is_number());
        // scenario_ids round-trips
        assert_eq!(parsed["scenario_ids"][0], "hover_test");
        assert_eq!(parsed["scenario_ids"][1], "completion_test");
    }

    /// render_status_markdown shows coverage % when declared_scenario_count is in provenance.
    #[test]
    fn render_status_markdown_shows_coverage_pct() {
        let mut rows = BTreeMap::new();
        rows.insert("hover_correctness_pct".to_string(), PercentMetric { value: Some(100.0) });
        let artifact = UxScorecardArtifact {
            schema_version: 1,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            scenario_count: 8,
            scenario_ids: vec!["hover_core".to_string()],
            rows,
            latency_by_request_class: BTreeMap::new(),
            provenance: serde_json::json!({"declared_scenario_count": 22}),
        };
        let md = render_status_markdown(&artifact, None);
        assert!(md.contains("8` of `22"), "expected coverage fraction, got:\n{md}");
        assert!(md.contains("36%"), "expected 36% coverage, got:\n{md}");
        assert!(md.contains("hover_core"), "expected scenario id list, got:\n{md}");
    }

    /// render_status_markdown shows baseline deltas when baseline is provided.
    #[test]
    fn render_status_markdown_shows_baseline_deltas() {
        let mut rows = BTreeMap::new();
        rows.insert("hover_correctness_pct".to_string(), PercentMetric { value: Some(100.0) });
        let mut latency_by_request_class = BTreeMap::new();
        latency_by_request_class.insert(
            "hover".to_string(),
            LatencyPercentiles { p50_ms: Some(22.0), p95_ms: Some(29.0) },
        );
        let artifact = UxScorecardArtifact {
            schema_version: 1,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            scenario_count: 5,
            scenario_ids: vec![],
            rows,
            latency_by_request_class,
            provenance: serde_json::json!({"declared_scenario_count": 22}),
        };
        let mut floor_metrics = BTreeMap::new();
        floor_metrics.insert("latency_hover_p50_ms".to_string(), Some(24.0_f64));
        floor_metrics.insert("latency_hover_p95_ms".to_string(), Some(31.0_f64));
        let baseline = SubsystemBaseline {
            floor_metrics,
            improvement_metrics: BTreeMap::new(),
            tolerance_pct: 0.1,
            lower_is_better: vec![],
            schema_version: 1,
            measured_at: String::new(),
            subsystem: "editor_ux".to_string(),
            commit: String::new(),
        };
        let md = render_status_markdown(&artifact, Some(&baseline));
        assert!(md.contains("p50 baseline"), "expected baseline column header, got:\n{md}");
        assert!(md.contains("24.00"), "expected baseline p50 value, got:\n{md}");
        assert!(md.contains("31.00"), "expected baseline p95 value, got:\n{md}");
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
            diagnostics_correct: None,
            rename_success: None,
            cross_file_success: None,
            latency_ms_by_request: BTreeMap::from([("definition".to_string(), vec![42])]),
        }];
        let latency = compute_latency_percentiles(&rows);
        let def = latency.get("definition").expect("definition present");
        assert_eq!(def.p50_ms, Some(42.0));
        assert_eq!(def.p95_ms, Some(42.0));
    }
}

